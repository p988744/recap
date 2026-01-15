//! Output formatting module
//!
//! Provides table and JSON output formatting for CLI commands.

use serde::Serialize;
use std::fmt::Display;
use tabled::{Table, Tabled};

/// Output format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Invalid format: {}. Use 'table' or 'json'", s)),
        }
    }
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

/// Print data in the specified format
pub fn print_output<T>(data: &[T], format: OutputFormat) -> anyhow::Result<()>
where
    T: Serialize + Tabled,
{
    match format {
        OutputFormat::Table => {
            if data.is_empty() {
                println!("No items found.");
            } else {
                let table = Table::new(data).to_string();
                println!("{}", table);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{}", json);
        }
    }
    Ok(())
}

/// Print a single item in the specified format
pub fn print_single<T>(data: &T, format: OutputFormat) -> anyhow::Result<()>
where
    T: Serialize + Tabled,
{
    match format {
        OutputFormat::Table => {
            let table = Table::new([data]).to_string();
            println!("{}", table);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{}", json);
        }
    }
    Ok(())
}

/// Print a success message (respects quiet mode)
pub fn print_success(message: &str, quiet: bool) {
    if !quiet {
        println!("{}", colored::Colorize::green(message));
    }
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{}", colored::Colorize::red(message));
}

/// Print an info message (respects quiet mode)
pub fn print_info(message: &str, quiet: bool) {
    if !quiet {
        println!("{}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("table".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("TABLE".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
    }
}
