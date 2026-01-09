"""
SQLAlchemy Database Models for Recap

Multi-user support with authentication, departments, work items, and goals.
"""

import uuid
from datetime import datetime
from typing import AsyncGenerator, Optional

from sqlalchemy import (
    Boolean,
    Column,
    DateTime,
    Enum,
    Float,
    ForeignKey,
    Integer,
    String,
    Text,
    create_engine,
)
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy.orm import DeclarativeBase, relationship

from ...config import CONFIG_DIR

# Database file location
DATABASE_PATH = CONFIG_DIR / "recap.db"
DATABASE_URL = f"sqlite+aiosqlite:///{DATABASE_PATH}"

# Create async engine
engine = create_async_engine(DATABASE_URL, echo=False)
async_session_maker = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)


class Base(DeclarativeBase):
    """Base class for all models"""
    pass


class User(Base):
    """User model for authentication and profile"""
    __tablename__ = "users"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    email = Column(String(255), unique=True, nullable=False, index=True)
    password_hash = Column(String(255), nullable=False)
    name = Column(String(100), nullable=False)
    employee_id = Column(String(50), nullable=True)
    department_id = Column(String(36), ForeignKey("departments.id"), nullable=True)
    title = Column(String(100), nullable=True)
    gitlab_url = Column(String(255), nullable=True)  # GitLab server URL
    gitlab_pat = Column(Text, nullable=True)  # Encrypted PAT
    jira_email = Column(String(255), nullable=True)  # User's Jira email
    jira_account_id = Column(String(100), nullable=True)  # Jira account ID
    is_active = Column(Boolean, default=True)
    is_admin = Column(Boolean, default=False)
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    # Relationships
    department = relationship("Department", back_populates="members", foreign_keys=[department_id])
    managed_departments = relationship(
        "Department",
        back_populates="manager",
        foreign_keys="Department.manager_id"
    )
    yearly_goals = relationship("YearlyGoal", back_populates="user", cascade="all, delete-orphan")
    work_items = relationship("WorkItem", back_populates="user", cascade="all, delete-orphan")
    gitlab_projects = relationship("GitLabProject", back_populates="user", cascade="all, delete-orphan")
    claude_sessions = relationship("ClaudeSession", back_populates="user", cascade="all, delete-orphan")


class Department(Base):
    """Department model"""
    __tablename__ = "departments"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    name = Column(String(100), nullable=False, unique=True)
    manager_id = Column(String(36), ForeignKey("users.id"), nullable=True)
    jira_group = Column(String(100), nullable=True)  # Jira group name
    tempo_team_id = Column(String(100), nullable=True)  # Tempo team ID
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    # Relationships
    manager = relationship("User", back_populates="managed_departments", foreign_keys=[manager_id])
    members = relationship("User", back_populates="department", foreign_keys="User.department_id")
    goals = relationship("DepartmentGoal", back_populates="department", cascade="all, delete-orphan")


class DepartmentGoal(Base):
    """Department-level goals (半年度目標)"""
    __tablename__ = "department_goals"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    department_id = Column(String(36), ForeignKey("departments.id"), nullable=False)
    year = Column(Integer, nullable=False)
    half = Column(Integer, nullable=False)  # 1 = 上半年, 2 = 下半年
    title = Column(String(255), nullable=False)
    description = Column(Text, nullable=True)
    category = Column(String(100), nullable=True)  # 目標類別
    created_at = Column(DateTime, default=datetime.utcnow)

    # Relationships
    department = relationship("Department", back_populates="goals")
    yearly_goals = relationship("YearlyGoal", back_populates="department_goal")


class YearlyGoal(Base):
    """Personal yearly goals linked to department goals"""
    __tablename__ = "yearly_goals"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = Column(String(36), ForeignKey("users.id"), nullable=False)
    department_goal_id = Column(String(36), ForeignKey("department_goals.id"), nullable=True)
    year = Column(Integer, nullable=False)
    title = Column(String(255), nullable=False)
    description = Column(Text, nullable=True)
    category = Column(String(100), nullable=True)  # PE 分類: 工作成果, 技能發展, etc.
    weight = Column(Float, default=0.1)  # 權重
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    # Relationships
    user = relationship("User", back_populates="yearly_goals")
    department_goal = relationship("DepartmentGoal", back_populates="yearly_goals")
    work_items = relationship("WorkItem", back_populates="yearly_goal")


class WorkItem(Base):
    """Work items collected from various sources"""
    __tablename__ = "work_items"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = Column(String(36), ForeignKey("users.id"), nullable=False)

    # Source information
    source = Column(String(20), nullable=False)  # 'gitlab', 'claude_code', 'manual'
    source_id = Column(String(255), nullable=True)  # commit SHA, session ID, etc.
    source_url = Column(String(500), nullable=True)  # Link to source

    # Work item details
    title = Column(String(500), nullable=False)
    description = Column(Text, nullable=True)
    hours = Column(Float, nullable=False, default=0)
    date = Column(DateTime, nullable=False)

    # Jira mapping
    jira_issue_key = Column(String(50), nullable=True)  # Confirmed Jira issue
    jira_issue_suggested = Column(String(50), nullable=True)  # LLM suggested
    jira_issue_title = Column(String(500), nullable=True)  # Cached issue title

    # Classification
    category = Column(String(100), nullable=True)  # PE 分類
    tags = Column(Text, nullable=True)  # JSON array of tags
    yearly_goal_id = Column(String(36), ForeignKey("yearly_goals.id"), nullable=True)

    # Tempo sync status
    synced_to_tempo = Column(Boolean, default=False)
    tempo_worklog_id = Column(String(100), nullable=True)
    synced_at = Column(DateTime, nullable=True)

    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    # Relationships
    user = relationship("User", back_populates="work_items")
    yearly_goal = relationship("YearlyGoal", back_populates="work_items")


class GitLabProject(Base):
    """GitLab projects tracked by users"""
    __tablename__ = "gitlab_projects"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = Column(String(36), ForeignKey("users.id"), nullable=False)
    gitlab_project_id = Column(Integer, nullable=False)  # GitLab's internal project ID
    gitlab_url = Column(String(500), nullable=False)  # Full project URL
    name = Column(String(255), nullable=False)
    path_with_namespace = Column(String(500), nullable=True)  # e.g., "group/project"
    default_branch = Column(String(100), default="main")  # Default branch
    default_jira_issue = Column(String(50), nullable=True)  # Default Jira issue for this project
    last_synced = Column(DateTime, nullable=True)
    enabled = Column(Boolean, default=True)
    created_at = Column(DateTime, default=datetime.utcnow)

    # Relationships
    user = relationship("User", back_populates="gitlab_projects")


class ClaudeSession(Base):
    """Claude Code sessions synced via hooks"""
    __tablename__ = "claude_sessions"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = Column(String(36), ForeignKey("users.id"), nullable=True)  # Can be null before user mapping
    session_id = Column(String(100), nullable=False, unique=True, index=True)
    project_dir = Column(String(500), nullable=False)
    project_name = Column(String(255), nullable=False)
    transcript_path = Column(String(500), nullable=True)
    hook_event = Column(String(50), nullable=False)  # 'SessionEnd', 'Stop', etc.

    # Analysis
    analyzed = Column(Boolean, default=False)
    summary = Column(Text, nullable=True)  # LLM-generated summary

    synced_at = Column(DateTime, default=datetime.utcnow)
    created_at = Column(DateTime, default=datetime.utcnow)

    # Relationships
    user = relationship("User", back_populates="claude_sessions")


# Database initialization
async def init_db():
    """Initialize database and create all tables"""
    # Ensure config directory exists
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)


async def get_db() -> AsyncGenerator[AsyncSession, None]:
    """Dependency for getting database session"""
    async with async_session_maker() as session:
        try:
            yield session
        finally:
            await session.close()
