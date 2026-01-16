//! Report formatting
//!
//! Output formatters for tempo reports.

use super::types::TempoReport;

/// Print report in plain text format
pub fn print_text_report(report: &TempoReport) {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  {} 工作報告", report.period);
    println!("║  期間: {} ~ {}", report.start_date, report.end_date);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    for project in &report.projects {
        println!("📁 {} ({:.1} 小時)", project.project, project.hours);
        for summary in &project.summary {
            println!("   • {}", summary);
        }
        println!();
    }

    println!("───────────────────────────────────────────────────────────────");
    println!("總計: {:.1} 小時 / {} 項工作", report.total_hours, report.total_items);
}

/// Print report in markdown format
pub fn print_markdown_report(report: &TempoReport) {
    println!("# {} 工作報告", report.period);
    println!();
    println!("**期間:** {} ~ {}", report.start_date, report.end_date);
    println!();

    for project in &report.projects {
        println!("## {} ({:.1} 小時)", project.project, project.hours);
        println!();
        for summary in &project.summary {
            println!("- {}", summary);
        }
        println!();
    }

    println!("---");
    println!("**總計:** {:.1} 小時 / {} 項工作", report.total_hours, report.total_items);
}
