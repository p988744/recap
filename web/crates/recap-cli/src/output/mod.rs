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
    use serde::Serialize;
    use tabled::Tabled;

    #[derive(Debug, Serialize, Tabled)]
    struct TestItem {
        name: String,
        value: i32,
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("table".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("TABLE".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_output_format_from_str_mixed_case() {
        assert_eq!("Table".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
        assert_eq!("Json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("TaBlE".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
    }

    #[test]
    fn test_output_format_from_str_error_message() {
        let err = "xml".parse::<OutputFormat>().unwrap_err();
        assert!(err.contains("xml"));
        assert!(err.contains("table"));
        assert!(err.contains("json"));
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
    }

    #[test]
    fn test_output_format_default() {
        let format: OutputFormat = Default::default();
        assert_eq!(format, OutputFormat::Table);
    }

    #[test]
    fn test_output_format_clone_copy() {
        let format = OutputFormat::Json;
        let cloned = format.clone();
        let copied = format;
        assert_eq!(format, cloned);
        assert_eq!(format, copied);
    }

    #[test]
    fn test_output_format_debug() {
        let format = OutputFormat::Table;
        let debug_str = format!("{:?}", format);
        assert_eq!(debug_str, "Table");
    }

    #[test]
    fn test_print_output_table_empty() {
        let items: Vec<TestItem> = vec![];
        // Should not panic
        let result = print_output(&items, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_output_table_with_data() {
        let items = vec![
            TestItem { name: "foo".to_string(), value: 1 },
            TestItem { name: "bar".to_string(), value: 2 },
        ];
        let result = print_output(&items, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_output_json() {
        let items = vec![
            TestItem { name: "test".to_string(), value: 42 },
        ];
        let result = print_output(&items, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_single_table() {
        let item = TestItem { name: "single".to_string(), value: 99 };
        let result = print_single(&item, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_single_json() {
        let item = TestItem { name: "single".to_string(), value: 99 };
        let result = print_single(&item, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_success_not_quiet() {
        // Should not panic
        print_success("Success message", false);
    }

    #[test]
    fn test_print_success_quiet() {
        // Should not panic and not print
        print_success("Success message", true);
    }

    #[test]
    fn test_print_error() {
        // Should not panic
        print_error("Error message");
    }

    #[test]
    fn test_print_info_not_quiet() {
        // Should not panic
        print_info("Info message", false);
    }

    #[test]
    fn test_print_info_quiet() {
        // Should not panic and not print
        print_info("Info message", true);
    }
}
