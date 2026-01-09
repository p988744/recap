"""
Sources Router - Data sources management API
"""

import subprocess
from pathlib import Path
from datetime import datetime

from fastapi import APIRouter, HTTPException

from ...config import Config
from ..models.schemas import (
    SourcesResponse,
    GitRepoInfo,
    AddGitRepoRequest,
    SuccessResponse,
)

router = APIRouter()


def get_git_repo_info(repo_path: str) -> GitRepoInfo:
    """Get information about a Git repository"""
    path = Path(repo_path).expanduser().resolve()
    name = path.name

    if not path.exists():
        return GitRepoInfo(path=str(path), name=name, valid=False)

    git_dir = path / ".git"
    if not git_dir.exists():
        return GitRepoInfo(path=str(path), name=name, valid=False)

    # Get last commit info
    try:
        result = subprocess.run(
            ["git", "log", "-1", "--format=%H|%aI"],
            cwd=str(path),
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0 and result.stdout.strip():
            parts = result.stdout.strip().split("|")
            commit_hash = parts[0][:7] if parts else None
            commit_date = None
            if len(parts) > 1:
                try:
                    commit_date = datetime.fromisoformat(parts[1].replace("Z", "+00:00"))
                except ValueError:
                    pass
            return GitRepoInfo(
                path=str(path),
                name=name,
                valid=True,
                last_commit=commit_hash,
                last_commit_date=commit_date,
            )
    except Exception:
        pass

    return GitRepoInfo(path=str(path), name=name, valid=True)


def check_claude_code() -> tuple[bool, str | None]:
    """Check if Claude Code sessions are available"""
    claude_path = Path.home() / ".claude"
    projects_path = claude_path / "projects"

    if projects_path.exists() and any(projects_path.iterdir()):
        return True, str(claude_path)
    return False, None


@router.get("", response_model=SourcesResponse)
async def get_sources():
    """Get all configured data sources"""
    config = Config.load()

    # Get Git repos info
    git_repos = []
    for repo_path in config.git_repos:
        git_repos.append(get_git_repo_info(repo_path))

    # Check Claude Code
    claude_connected, claude_path = check_claude_code()

    return SourcesResponse(
        mode="git" if config.use_git_mode else "claude",
        git_repos=git_repos,
        claude_connected=claude_connected,
        claude_path=claude_path,
        outlook_enabled=config.outlook_enabled,
    )


@router.post("/git", response_model=SuccessResponse)
async def add_git_repo(request: AddGitRepoRequest):
    """Add a Git repository to track"""
    path = Path(request.path).expanduser().resolve()

    if not path.exists():
        raise HTTPException(status_code=400, detail=f"Path does not exist: {path}")

    git_dir = path / ".git"
    if not git_dir.exists():
        raise HTTPException(status_code=400, detail=f"Not a Git repository: {path}")

    config = Config.load()
    path_str = str(path)

    if path_str in config.git_repos:
        raise HTTPException(status_code=400, detail=f"Repository already added: {path}")

    config.git_repos.append(path_str)
    config.save()

    return SuccessResponse(message=f"Added Git repository: {path.name}")


@router.delete("/git/{repo_name}", response_model=SuccessResponse)
async def remove_git_repo(repo_name: str):
    """Remove a Git repository from tracking"""
    config = Config.load()

    # Find repo by name
    found = None
    for repo_path in config.git_repos:
        if Path(repo_path).name == repo_name:
            found = repo_path
            break

    if not found:
        raise HTTPException(status_code=404, detail=f"Repository not found: {repo_name}")

    config.git_repos.remove(found)
    config.save()

    return SuccessResponse(message=f"Removed Git repository: {repo_name}")


@router.post("/mode/git", response_model=SuccessResponse)
async def set_git_mode():
    """Switch to Git mode"""
    config = Config.load()
    config.use_git_mode = True
    config.save()
    return SuccessResponse(message="Switched to Git mode")


@router.post("/mode/claude", response_model=SuccessResponse)
async def set_claude_mode():
    """Switch to Claude Code mode"""
    config = Config.load()
    config.use_git_mode = False
    config.save()
    return SuccessResponse(message="Switched to Claude Code mode")
