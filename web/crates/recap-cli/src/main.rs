//! Recap CLI - Work tracking and reporting tool
//!
//! A command-line interface for managing work items, syncing data sources,
//! and generating reports.

mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::Write;

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

    /// Enable debug logging (outputs to console and log file)
    #[arg(long, global = true)]
    debug: bool,

    /// Log file path (default: ~/.recap/logs/recap-cli.log)
    #[arg(long, global = true)]
    log_file: Option<String>,
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

    // Initialize logging if debug mode is enabled
    if cli.debug {
        init_logging(cli.log_file.as_deref())?;
        log::info!("Debug logging enabled");
        log::debug!("CLI arguments parsed");
    }

    // Set up database path if provided
    if let Some(db_path) = &cli.db {
        std::env::set_var("RECAP_DB_PATH", db_path);
        if cli.debug {
            log::debug!("Database path set to: {}", db_path);
        }
    }

    // Initialize database
    if cli.debug {
        log::debug!("Initializing database connection...");
    }
    let db = recap_core::Database::new().await?;
    if cli.debug {
        log::info!("Database connection established");
    }

    // Create context for commands
    let ctx = commands::Context {
        db,
        format: cli.format,
        quiet: cli.quiet,
        debug: cli.debug,
    };

    // Execute command
    let result = match cli.command {
        Commands::Work { action } => commands::work::execute(&ctx, action).await,
        Commands::Sync { action } => commands::sync::execute(&ctx, action).await,
        Commands::Source { action } => commands::source::execute(&ctx, action).await,
        Commands::Report { action } => commands::report::execute(&ctx, action).await,
        Commands::Config { action } => commands::config::execute(&ctx, action).await,
        Commands::Tempo { action } => commands::tempo_report::execute(&ctx, action).await,
        Commands::Dashboard { action } => commands::dashboard::execute(&ctx, action).await,
        Commands::Claude { action } => commands::claude::execute(&ctx, action).await,
    };

    if cli.debug {
        match &result {
            Ok(_) => log::info!("Command completed successfully"),
            Err(e) => log::error!("Command failed: {}", e),
        }
    }

    result
}

/// Initialize logging with both console and file output
fn init_logging(log_file_path: Option<&str>) -> Result<()> {
    use env_logger::{Builder, Target};
    use std::fs::{self, OpenOptions};
    use std::sync::Mutex;

    // Determine log file path
    let log_path = match log_file_path {
        Some(path) => std::path::PathBuf::from(path),
        None => {
            let home = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
            let log_dir = home.join(".recap").join("logs");
            fs::create_dir_all(&log_dir)?;
            log_dir.join("recap-cli.log")
        }
    };

    // Open log file in append mode
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // Store file handle in a static for the format closure
    static LOG_FILE: std::sync::OnceLock<Mutex<std::fs::File>> = std::sync::OnceLock::new();
    let _ = LOG_FILE.set(Mutex::new(log_file));

    // Create a custom logger that writes to both stderr and file
    let mut builder = Builder::new();
    builder
        .filter_level(log::LevelFilter::Debug)
        .format(move |buf, record| {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let log_line = format!(
                "[{} {} {}:{}] {}",
                timestamp,
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            );

            // Write to file as well
            if let Some(file_mutex) = LOG_FILE.get() {
                if let Ok(mut file) = file_mutex.lock() {
                    let _ = writeln!(file, "{}", log_line);
                }
            }

            writeln!(buf, "{}", log_line)
        })
        .target(Target::Stderr);

    builder.init();

    eprintln!("[DEBUG] Log file: {}", log_path.display());

    Ok(())
}
