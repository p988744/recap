//! Excel export service
//!
//! Generate Excel reports for work items

use anyhow::Result;
use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Work item data for Excel export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelWorkItem {
    pub date: String,
    pub title: String,
    pub description: Option<String>,
    pub hours: f64,
    pub project: Option<String>,
    pub jira_key: Option<String>,
    pub source: String,
    pub synced_to_tempo: bool,
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub user_name: String,
    pub start_date: String,
    pub end_date: String,
    pub generated_at: String,
}

/// Project summary for aggregated view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_name: String,
    pub total_hours: f64,
    pub item_count: usize,
}

/// Excel report generator
pub struct ExcelReportGenerator {
    workbook: Workbook,
    // Styles
    header_format: Format,
    subheader_format: Format,
    total_format: Format,
    date_format: Format,
    number_format: Format,
}

impl ExcelReportGenerator {
    /// Create a new Excel report generator
    pub fn new() -> Result<Self> {
        let workbook = Workbook::new();

        // Header style: blue background, white bold text
        let header_format = Format::new()
            .set_bold()
            .set_font_color(Color::White)
            .set_background_color(Color::RGB(0x4472C4))
            .set_align(rust_xlsxwriter::FormatAlign::Center)
            .set_border(FormatBorder::Thin);

        // Subheader style: light blue background
        let subheader_format = Format::new()
            .set_background_color(Color::RGB(0xB4C6E7))
            .set_align(rust_xlsxwriter::FormatAlign::Center)
            .set_border(FormatBorder::Thin);

        // Total row style: yellow background, bold
        let total_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0xFFC000))
            .set_align(rust_xlsxwriter::FormatAlign::Center)
            .set_border(FormatBorder::Thin);

        // Date format
        let date_format = Format::new()
            .set_align(rust_xlsxwriter::FormatAlign::Center)
            .set_border(FormatBorder::Thin);

        // Number format with 1 decimal
        let number_format = Format::new()
            .set_num_format("0.0")
            .set_align(rust_xlsxwriter::FormatAlign::Center)
            .set_border(FormatBorder::Thin);

        Ok(Self {
            workbook,
            header_format,
            subheader_format,
            total_format,
            date_format,
            number_format,
        })
    }

    /// Create a personal work report
    pub fn create_personal_report(
        &mut self,
        metadata: &ReportMetadata,
        items: &[ExcelWorkItem],
        projects: &[ProjectSummary],
    ) -> Result<()> {
        self.create_summary_sheet(metadata, items, projects)?;
        self.create_details_sheet(items)?;
        self.create_by_project_sheet(projects)?;
        Ok(())
    }

    /// Create summary sheet
    fn create_summary_sheet(
        &mut self,
        metadata: &ReportMetadata,
        items: &[ExcelWorkItem],
        projects: &[ProjectSummary],
    ) -> Result<()> {
        let worksheet = self.workbook.add_worksheet();
        worksheet.set_name("Summary")?;

        // Title
        let title_format = Format::new()
            .set_bold()
            .set_font_size(16);
        worksheet.write_with_format(0, 0, "Work Report", &title_format)?;
        worksheet.merge_range(0, 0, 0, 3, "Work Report", &title_format)?;

        // Metadata
        let label_format = Format::new().set_bold();
        worksheet.write_with_format(2, 0, "Name:", &label_format)?;
        worksheet.write(2, 1, &metadata.user_name)?;
        worksheet.write_with_format(3, 0, "Period:", &label_format)?;
        worksheet.write(3, 1, format!("{} ~ {}", metadata.start_date, metadata.end_date))?;
        worksheet.write_with_format(4, 0, "Generated:", &label_format)?;
        worksheet.write(4, 1, &metadata.generated_at)?;

        // Calculate totals
        let total_hours: f64 = items.iter().map(|i| i.hours).sum();
        let synced_count = items.iter().filter(|i| i.synced_to_tempo).count();
        let mapped_count = items.iter().filter(|i| i.jira_key.is_some()).count();

        worksheet.write_with_format(5, 0, "Total Hours:", &label_format)?;
        worksheet.write_with_format(5, 1, total_hours, &self.number_format)?;
        worksheet.write_with_format(6, 0, "Total Items:", &label_format)?;
        worksheet.write(6, 1, items.len() as u32)?;
        worksheet.write_with_format(7, 0, "Synced to Tempo:", &label_format)?;
        worksheet.write(7, 1, format!("{}/{}", synced_count, items.len()))?;
        worksheet.write_with_format(8, 0, "Mapped to Jira:", &label_format)?;
        worksheet.write(8, 1, format!("{}/{}", mapped_count, items.len()))?;

        // Project summary table
        let start_row = 10;
        worksheet.write_with_format(start_row, 0, "Project", &self.header_format)?;
        worksheet.write_with_format(start_row, 1, "Hours", &self.header_format)?;
        worksheet.write_with_format(start_row, 2, "Items", &self.header_format)?;

        for (idx, project) in projects.iter().enumerate() {
            let row = start_row + 1 + idx as u32;
            worksheet.write_with_format(row, 0, &project.project_name, &self.date_format)?;
            worksheet.write_with_format(row, 1, project.total_hours, &self.number_format)?;
            worksheet.write_with_format(row, 2, project.item_count as u32, &self.date_format)?;
        }

        // Total row
        let total_row = start_row + 1 + projects.len() as u32;
        worksheet.write_with_format(total_row, 0, "Total", &self.total_format)?;
        worksheet.write_with_format(total_row, 1, total_hours, &self.total_format)?;
        worksheet.write_with_format(total_row, 2, items.len() as u32, &self.total_format)?;

        // Column widths
        worksheet.set_column_width(0, 20)?;
        worksheet.set_column_width(1, 15)?;
        worksheet.set_column_width(2, 10)?;
        worksheet.set_column_width(3, 15)?;

        Ok(())
    }

    /// Create details sheet with all work items
    fn create_details_sheet(&mut self, items: &[ExcelWorkItem]) -> Result<()> {
        let worksheet = self.workbook.add_worksheet();
        worksheet.set_name("Details")?;

        // Headers
        let headers = ["Date", "Title", "Hours", "Project", "Jira", "Source", "Synced"];
        for (col, header) in headers.iter().enumerate() {
            worksheet.write_with_format(0, col as u16, *header, &self.header_format)?;
        }

        // Data rows
        for (idx, item) in items.iter().enumerate() {
            let row = 1 + idx as u32;
            worksheet.write_with_format(row, 0, &item.date, &self.date_format)?;
            worksheet.write(row, 1, &item.title)?;
            worksheet.write_with_format(row, 2, item.hours, &self.number_format)?;
            worksheet.write(row, 3, item.project.as_deref().unwrap_or(""))?;
            worksheet.write(row, 4, item.jira_key.as_deref().unwrap_or(""))?;
            worksheet.write(row, 5, &item.source)?;
            worksheet.write(row, 6, if item.synced_to_tempo { "Yes" } else { "No" })?;
        }

        // Column widths
        worksheet.set_column_width(0, 12)?;
        worksheet.set_column_width(1, 50)?;
        worksheet.set_column_width(2, 10)?;
        worksheet.set_column_width(3, 20)?;
        worksheet.set_column_width(4, 15)?;
        worksheet.set_column_width(5, 12)?;
        worksheet.set_column_width(6, 10)?;

        Ok(())
    }

    /// Create by-project sheet
    fn create_by_project_sheet(&mut self, projects: &[ProjectSummary]) -> Result<()> {
        let worksheet = self.workbook.add_worksheet();
        worksheet.set_name("By Project")?;

        // Headers
        worksheet.write_with_format(0, 0, "Project", &self.header_format)?;
        worksheet.write_with_format(0, 1, "Total Hours", &self.header_format)?;
        worksheet.write_with_format(0, 2, "Items", &self.header_format)?;

        // Data rows
        let mut total_hours = 0.0;
        let mut total_items = 0;
        for (idx, project) in projects.iter().enumerate() {
            let row = 1 + idx as u32;
            worksheet.write_with_format(row, 0, &project.project_name, &self.date_format)?;
            worksheet.write_with_format(row, 1, project.total_hours, &self.number_format)?;
            worksheet.write_with_format(row, 2, project.item_count as u32, &self.date_format)?;
            total_hours += project.total_hours;
            total_items += project.item_count;
        }

        // Total row
        let total_row = 1 + projects.len() as u32;
        worksheet.write_with_format(total_row, 0, "Total", &self.total_format)?;
        worksheet.write_with_format(total_row, 1, total_hours, &self.total_format)?;
        worksheet.write_with_format(total_row, 2, total_items as u32, &self.total_format)?;

        // Column widths
        worksheet.set_column_width(0, 30)?;
        worksheet.set_column_width(1, 15)?;
        worksheet.set_column_width(2, 10)?;

        Ok(())
    }

    /// Save the workbook to a file
    pub fn save<P: AsRef<Path>>(mut self, path: P) -> Result<()> {
        self.workbook.save(path)?;
        Ok(())
    }

    /// Save the workbook to a byte vector (for HTTP response)
    pub fn save_to_buffer(mut self) -> Result<Vec<u8>> {
        let buffer = self.workbook.save_to_buffer()?;
        Ok(buffer)
    }
}

impl Default for ExcelReportGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create ExcelReportGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_report() {
        let mut generator = ExcelReportGenerator::new().unwrap();

        let metadata = ReportMetadata {
            user_name: "Test User".to_string(),
            start_date: "2025-01-01".to_string(),
            end_date: "2025-01-31".to_string(),
            generated_at: "2025-01-31 10:00:00".to_string(),
        };

        let items = vec![
            ExcelWorkItem {
                date: "2025-01-15".to_string(),
                title: "Test task 1".to_string(),
                description: Some("Description".to_string()),
                hours: 2.5,
                project: Some("Project A".to_string()),
                jira_key: Some("PROJ-123".to_string()),
                source: "claude_code".to_string(),
                synced_to_tempo: true,
            },
            ExcelWorkItem {
                date: "2025-01-16".to_string(),
                title: "Test task 2".to_string(),
                description: None,
                hours: 3.0,
                project: Some("Project B".to_string()),
                jira_key: None,
                source: "manual".to_string(),
                synced_to_tempo: false,
            },
        ];

        let projects = vec![
            ProjectSummary {
                project_name: "Project A".to_string(),
                total_hours: 2.5,
                item_count: 1,
            },
            ProjectSummary {
                project_name: "Project B".to_string(),
                total_hours: 3.0,
                item_count: 1,
            },
        ];

        let result = generator.create_personal_report(&metadata, &items, &projects);
        assert!(result.is_ok());

        // Test saving to buffer
        let buffer = generator.save_to_buffer();
        assert!(buffer.is_ok());
        assert!(!buffer.unwrap().is_empty());
    }
}
