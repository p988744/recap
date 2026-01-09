"""
Recap API - Main FastAPI application
"""

from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from .routers import config, sources, analyze, tempo, sessions, auth, users, gitlab, work_items, reports
from .models.database import init_db


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Lifespan context manager for startup/shutdown events"""
    # Startup: Initialize database
    await init_db()
    yield
    # Shutdown: cleanup if needed


def create_app() -> FastAPI:
    """Create and configure the FastAPI application"""
    app = FastAPI(
        title="Recap API",
        description="Auto-capture your work from Git, Claude Code, and more. Generate reports and sync to Tempo.",
        version="2.0.0",
        docs_url="/api/docs",
        redoc_url="/api/redoc",
        openapi_url="/api/openapi.json",
        lifespan=lifespan,
    )

    # CORS middleware for frontend (local app mode)
    app.add_middleware(
        CORSMiddleware,
        allow_origins=[
            "http://localhost:3000",
            "http://localhost:5173",
            "http://localhost:5174",
            "http://localhost:5175",
            "http://localhost:8000",
            "http://127.0.0.1:3000",
            "http://127.0.0.1:5173",
            "http://127.0.0.1:5174",
            "http://127.0.0.1:5175",
            "http://127.0.0.1:8000",
        ],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    # Auth routers (no auth required)
    app.include_router(auth.router, prefix="/api/auth", tags=["auth"])

    # User routers (auth required)
    app.include_router(users.router, prefix="/api/users", tags=["users"])

    # GitLab integration (auth required)
    app.include_router(gitlab.router, prefix="/api/gitlab", tags=["gitlab"])

    # Work items (auth required)
    app.include_router(work_items.router, prefix="/api/work-items", tags=["work-items"])

    # Reports (auth required)
    app.include_router(reports.router, prefix="/api/reports", tags=["reports"])

    # Existing routers
    app.include_router(config.router, prefix="/api/config", tags=["config"])
    app.include_router(sources.router, prefix="/api/sources", tags=["sources"])
    app.include_router(analyze.router, prefix="/api/analyze", tags=["analyze"])
    app.include_router(tempo.router, prefix="/api/tempo", tags=["tempo"])
    app.include_router(sessions.router, prefix="/api/sessions", tags=["sessions"])

    @app.get("/api/health")
    async def health_check():
        """Health check endpoint"""
        return {"status": "ok", "version": "2.0.0"}

    return app


# Create the default app instance
app = create_app()
