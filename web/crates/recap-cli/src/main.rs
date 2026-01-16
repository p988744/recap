//! Recap CLI - Work tracking and reporting tool
//!
//! A command-line interface for managing work items, syncing data sources,
//! and generating reports.

mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "recap")]
#[command(author, version, about = "Work tracking and reporting CLI", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format: table (default) or json
    #[arg(long, global = true, default_value = "table")]
    format: output::OutputFormat,

    /// Suppress progress messages
    #[arg(long, short, global = true)]
    quiet: bool,

    /// Override database path (or set RECAP_DB_PATH env var)
    #[arg(long, env = "RECAP_DB_PATH", global = true)]
    db: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage work items
    Work {
        #[command(subcommand)]
        action: commands::work::WorkAction,
    },

    /// Sync data from sources
    Sync {
        #[command(subcommand)]
        action: commands::sync::SyncAction,
    },

    /// Manage data sources (git repos, Claude, GitLab)
    Source {
        #[command(subcommand)]
        action: commands::source::SourceAction,
    },

    /// Generate reports
    Report {
        #[command(subcommand)]
        action: commands::report::ReportAction,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: commands::config::ConfigAction,
    },

    /// Generate smart Tempo reports (daily/weekly/monthly/quarterly/semi-annual)
    Tempo {
        #[command(subcommand)]
        action: commands::tempo_report::TempoReportAction,
    },

    /// Dashboard statistics and visualizations
    Dashboard {
        #[command(subcommand)]
        action: commands::dashboard::DashboardAction,
    },

    /// View and manage Claude Code sessions
    Claude {
        #[command(subcommand)]
        action: commands::claude::ClaudeAction,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up database path if provided
    if let Some(db_path) = &cli.db {
        std::env::set_var("RECAP_DB_PATH", db_path);
    }

    // Initialize database
    let db = recap_core::Database::new().await?;

    // Create context for commands
    let ctx = commands::Context {
        db,
        format: cli.format,
        quiet: cli.quiet,
    };

    // Execute command
    match cli.command {
        Commands::Work { action } => commands::work::execute(&ctx, action).await,
        Commands::Sync { action } => commands::sync::execute(&ctx, action).await,
        Commands::Source { action } => commands::source::execute(&ctx, action).await,
        Commands::Report { action } => commands::report::execute(&ctx, action).await,
        Commands::Config { action } => commands::config::execute(&ctx, action).await,
        Commands::Tempo { action } => commands::tempo_report::execute(&ctx, action).await,
        Commands::Dashboard { action } => commands::dashboard::execute(&ctx, action).await,
        Commands::Claude { action } => commands::claude::execute(&ctx, action).await,
    }
}
