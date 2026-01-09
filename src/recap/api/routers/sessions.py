"""
Sessions Router - Claude Code session sync API
"""

import json
from datetime import datetime
from pathlib import Path
from typing import Optional

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from ...config import CONFIG_DIR

router = APIRouter()

# Session storage directory
SESSIONS_DIR = CONFIG_DIR / "sessions"


class SessionSyncRequest(BaseModel):
    """Request to sync a Claude Code session"""
    session_id: str
    project_dir: str
    transcript_path: Optional[str] = None
    hook_event: str  # SessionEnd, Stop, etc.
    timestamp: str
    summary: Optional[dict] = None  # Optional parsed summary


class SessionInfo(BaseModel):
    """Session information"""
    session_id: str
    project_dir: str
    project_name: str
    last_synced: str
    hook_event: str
    has_transcript: bool


class SessionListResponse(BaseModel):
    """List of synced sessions"""
    sessions: list[SessionInfo]
    total: int


def ensure_sessions_dir():
    """Ensure sessions directory exists"""
    SESSIONS_DIR.mkdir(parents=True, exist_ok=True)


def get_project_name(project_dir: str) -> str:
    """Extract project name from path"""
    return Path(project_dir).name


@router.post("/sync")
async def sync_session(request: SessionSyncRequest):
    """
    Sync a Claude Code session.
    Called by the hook script when a session ends or Claude stops.
    """
    ensure_sessions_dir()

    project_name = get_project_name(request.project_dir)

    # Create session record
    session_data = {
        "session_id": request.session_id,
        "project_dir": request.project_dir,
        "project_name": project_name,
        "transcript_path": request.transcript_path,
        "hook_event": request.hook_event,
        "timestamp": request.timestamp,
        "synced_at": datetime.now().isoformat(),
        "summary": request.summary,
    }

    # Save session data
    session_file = SESSIONS_DIR / f"{request.session_id}.json"
    with open(session_file, "w") as f:
        json.dump(session_data, f, indent=2, ensure_ascii=False)

    # Also update an index file for quick lookups
    index_file = SESSIONS_DIR / "index.json"
    index = {}
    if index_file.exists():
        try:
            with open(index_file) as f:
                index = json.load(f)
        except Exception:
            index = {}

    index[request.session_id] = {
        "project_name": project_name,
        "project_dir": request.project_dir,
        "last_synced": session_data["synced_at"],
        "hook_event": request.hook_event,
    }

    with open(index_file, "w") as f:
        json.dump(index, f, indent=2, ensure_ascii=False)

    return {
        "success": True,
        "message": f"Session {request.session_id} synced",
        "project_name": project_name,
    }


@router.get("", response_model=SessionListResponse)
async def list_sessions():
    """List all synced sessions"""
    ensure_sessions_dir()

    index_file = SESSIONS_DIR / "index.json"
    if not index_file.exists():
        return SessionListResponse(sessions=[], total=0)

    try:
        with open(index_file) as f:
            index = json.load(f)
    except Exception:
        return SessionListResponse(sessions=[], total=0)

    sessions = []
    for session_id, info in index.items():
        session_file = SESSIONS_DIR / f"{session_id}.json"
        sessions.append(SessionInfo(
            session_id=session_id,
            project_dir=info.get("project_dir", ""),
            project_name=info.get("project_name", "Unknown"),
            last_synced=info.get("last_synced", ""),
            hook_event=info.get("hook_event", ""),
            has_transcript=session_file.exists(),
        ))

    # Sort by last_synced descending
    sessions.sort(key=lambda s: s.last_synced, reverse=True)

    return SessionListResponse(sessions=sessions, total=len(sessions))


@router.get("/{session_id}")
async def get_session(session_id: str):
    """Get details for a specific session"""
    ensure_sessions_dir()

    session_file = SESSIONS_DIR / f"{session_id}.json"
    if not session_file.exists():
        raise HTTPException(status_code=404, detail=f"Session {session_id} not found")

    with open(session_file) as f:
        return json.load(f)


@router.delete("/{session_id}")
async def delete_session(session_id: str):
    """Delete a synced session"""
    ensure_sessions_dir()

    session_file = SESSIONS_DIR / f"{session_id}.json"
    if session_file.exists():
        session_file.unlink()

    # Update index
    index_file = SESSIONS_DIR / "index.json"
    if index_file.exists():
        try:
            with open(index_file) as f:
                index = json.load(f)
            if session_id in index:
                del index[session_id]
                with open(index_file, "w") as f:
                    json.dump(index, f, indent=2, ensure_ascii=False)
        except Exception:
            pass

    return {"success": True, "message": f"Session {session_id} deleted"}
