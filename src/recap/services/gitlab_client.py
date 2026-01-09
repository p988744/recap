"""
GitLab Client - Interact with GitLab API for commits, merge requests, and issues
"""

from dataclasses import dataclass
from datetime import datetime
from typing import Optional

import requests


@dataclass
class GitLabCommit:
    """GitLab commit data"""
    id: str
    short_id: str
    title: str
    message: str
    author_name: str
    author_email: str
    authored_date: datetime
    committed_date: datetime
    web_url: str


@dataclass
class GitLabMergeRequest:
    """GitLab merge request data"""
    id: int
    iid: int
    title: str
    description: str
    state: str
    author_username: str
    created_at: datetime
    merged_at: Optional[datetime]
    web_url: str
    source_branch: str
    target_branch: str


@dataclass
class GitLabProject:
    """GitLab project data"""
    id: int
    name: str
    path_with_namespace: str
    description: Optional[str]
    web_url: str
    default_branch: str
    last_activity_at: datetime


class GitLabClient:
    """Client for interacting with GitLab API"""

    def __init__(self, base_url: str, access_token: str):
        """
        Initialize GitLab client.

        Args:
            base_url: GitLab instance URL (e.g., https://gitlab.company.com)
            access_token: Personal Access Token with api scope
        """
        self.base_url = base_url.rstrip("/")
        self.access_token = access_token
        self.api_url = f"{self.base_url}/api/v4"
        self.headers = {"PRIVATE-TOKEN": access_token}

    def _request(
        self,
        method: str,
        endpoint: str,
        params: Optional[dict] = None,
        json_data: Optional[dict] = None,
    ) -> dict | list:
        """Make a request to the GitLab API."""
        url = f"{self.api_url}{endpoint}"
        response = requests.request(
            method=method,
            url=url,
            headers=self.headers,
            params=params,
            json=json_data,
            timeout=30,
        )
        response.raise_for_status()
        return response.json()

    def test_connection(self) -> dict:
        """Test the connection and return current user info."""
        return self._request("GET", "/user")

    def get_projects(
        self,
        membership: bool = True,
        per_page: int = 100,
        page: int = 1,
        search: Optional[str] = None,
    ) -> list[GitLabProject]:
        """
        Get projects the user has access to.

        Args:
            membership: Only return projects where user is a member
            per_page: Number of projects per page
            page: Page number
            search: Search for projects by name

        Returns:
            List of GitLabProject objects
        """
        params = {
            "membership": str(membership).lower(),
            "per_page": per_page,
            "page": page,
            "order_by": "last_activity_at",
            "sort": "desc",
        }
        if search:
            params["search"] = search

        data = self._request("GET", "/projects", params=params)
        return [
            GitLabProject(
                id=p["id"],
                name=p["name"],
                path_with_namespace=p["path_with_namespace"],
                description=p.get("description"),
                web_url=p["web_url"],
                default_branch=p.get("default_branch", "main"),
                last_activity_at=datetime.fromisoformat(
                    p["last_activity_at"].replace("Z", "+00:00")
                ),
            )
            for p in data
        ]

    def get_project(self, project_id: int) -> GitLabProject:
        """Get a single project by ID."""
        data = self._request("GET", f"/projects/{project_id}")
        return GitLabProject(
            id=data["id"],
            name=data["name"],
            path_with_namespace=data["path_with_namespace"],
            description=data.get("description"),
            web_url=data["web_url"],
            default_branch=data.get("default_branch", "main"),
            last_activity_at=datetime.fromisoformat(
                data["last_activity_at"].replace("Z", "+00:00")
            ),
        )

    def get_commits(
        self,
        project_id: int,
        since: Optional[datetime] = None,
        until: Optional[datetime] = None,
        ref_name: Optional[str] = None,
        per_page: int = 100,
        page: int = 1,
        author: Optional[str] = None,
    ) -> list[GitLabCommit]:
        """
        Get commits from a project.

        Args:
            project_id: Project ID
            since: Only commits after this date
            until: Only commits before this date
            ref_name: Branch or tag name
            per_page: Number of commits per page
            page: Page number
            author: Filter by author email or name

        Returns:
            List of GitLabCommit objects
        """
        params = {"per_page": per_page, "page": page}
        if since:
            params["since"] = since.isoformat()
        if until:
            params["until"] = until.isoformat()
        if ref_name:
            params["ref_name"] = ref_name
        if author:
            params["author"] = author

        data = self._request("GET", f"/projects/{project_id}/repository/commits", params=params)
        return [
            GitLabCommit(
                id=c["id"],
                short_id=c["short_id"],
                title=c["title"],
                message=c["message"],
                author_name=c["author_name"],
                author_email=c["author_email"],
                authored_date=datetime.fromisoformat(
                    c["authored_date"].replace("Z", "+00:00")
                ),
                committed_date=datetime.fromisoformat(
                    c["committed_date"].replace("Z", "+00:00")
                ),
                web_url=c["web_url"],
            )
            for c in data
        ]

    def get_merge_requests(
        self,
        project_id: int,
        state: str = "all",
        scope: str = "all",
        author_id: Optional[int] = None,
        created_after: Optional[datetime] = None,
        created_before: Optional[datetime] = None,
        per_page: int = 100,
        page: int = 1,
    ) -> list[GitLabMergeRequest]:
        """
        Get merge requests from a project.

        Args:
            project_id: Project ID
            state: Filter by state (opened, closed, merged, all)
            scope: Filter by scope (created_by_me, assigned_to_me, all)
            author_id: Filter by author user ID
            created_after: Only MRs created after this date
            created_before: Only MRs created before this date
            per_page: Number of MRs per page
            page: Page number

        Returns:
            List of GitLabMergeRequest objects
        """
        params = {
            "state": state,
            "scope": scope,
            "per_page": per_page,
            "page": page,
            "order_by": "created_at",
            "sort": "desc",
        }
        if author_id:
            params["author_id"] = author_id
        if created_after:
            params["created_after"] = created_after.isoformat()
        if created_before:
            params["created_before"] = created_before.isoformat()

        data = self._request("GET", f"/projects/{project_id}/merge_requests", params=params)
        return [
            GitLabMergeRequest(
                id=mr["id"],
                iid=mr["iid"],
                title=mr["title"],
                description=mr.get("description") or "",
                state=mr["state"],
                author_username=mr["author"]["username"],
                created_at=datetime.fromisoformat(
                    mr["created_at"].replace("Z", "+00:00")
                ),
                merged_at=datetime.fromisoformat(
                    mr["merged_at"].replace("Z", "+00:00")
                ) if mr.get("merged_at") else None,
                web_url=mr["web_url"],
                source_branch=mr["source_branch"],
                target_branch=mr["target_branch"],
            )
            for mr in data
        ]

    def get_user_contributions(
        self,
        project_id: int,
        since: datetime,
        until: datetime,
        author_email: Optional[str] = None,
    ) -> dict:
        """
        Get a summary of user contributions for a project.

        Args:
            project_id: Project ID
            since: Start date
            until: End date
            author_email: Filter by author email

        Returns:
            Dictionary with commits and merge_requests counts
        """
        commits = self.get_commits(
            project_id=project_id,
            since=since,
            until=until,
            author=author_email,
            per_page=100,
        )

        # Get current user ID if we need to filter MRs
        current_user = None
        if author_email:
            current_user = self.test_connection()

        merge_requests = self.get_merge_requests(
            project_id=project_id,
            state="all",
            scope="created_by_me" if not author_email else "all",
            author_id=current_user["id"] if current_user else None,
            created_after=since,
            created_before=until,
            per_page=100,
        )

        return {
            "commits": commits,
            "merge_requests": merge_requests,
            "summary": {
                "total_commits": len(commits),
                "total_merge_requests": len(merge_requests),
                "merged_mrs": len([mr for mr in merge_requests if mr.state == "merged"]),
            },
        }
