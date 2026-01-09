//! Tempo and Jira API clients
//!
//! This module provides clients for interacting with:
//! - Jira REST API (for issue validation and worklog creation)
//! - Tempo Timesheets API (for worklog management)

use anyhow::{anyhow, Result};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Worklog entry to upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorklogEntry {
    pub issue_key: String,
    pub date: String,           // YYYY-MM-DD
    pub time_spent_seconds: i64,
    pub description: String,
    pub account_id: Option<String>,
}

/// Jira user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraUser {
    #[serde(rename = "accountId", default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    #[serde(rename = "emailAddress", default)]
    pub email_address: Option<String>,
}

impl JiraUser {
    /// Get the user identifier (accountId for Cloud, name/key for Server)
    pub fn get_identifier(&self) -> Option<String> {
        self.account_id.clone()
            .or_else(|| self.name.clone())
            .or_else(|| self.key.clone())
    }
}

/// Jira issue information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: JiraIssueFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueFields {
    pub summary: Option<String>,
    #[serde(rename = "issuetype")]
    pub issue_type: Option<JiraIssueType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueType {
    pub name: String,
}

/// Worklog response from Jira/Tempo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorklogResponse {
    pub id: Option<String>,
    #[serde(rename = "tempoWorklogId")]
    pub tempo_worklog_id: Option<i64>,
}

/// Authentication type for Jira
#[derive(Debug, Clone, PartialEq)]
pub enum JiraAuthType {
    /// Personal Access Token (Jira Server/DC)
    Pat,
    /// Basic Auth with email:token (Jira Cloud)
    Basic,
}

impl From<&str> for JiraAuthType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "basic" => JiraAuthType::Basic,
            _ => JiraAuthType::Pat,
        }
    }
}

/// Jira REST API client
pub struct JiraClient {
    base_url: String,
    client: Client,
}

impl JiraClient {
    /// Create a new Jira client
    pub fn new(
        base_url: &str,
        token: &str,
        email: Option<&str>,
        auth_type: JiraAuthType,
    ) -> Result<Self> {
        let base_url = base_url.trim_end_matches('/').to_string();

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        // Set authorization header based on auth type
        let auth_value = match auth_type {
            JiraAuthType::Pat => {
                format!("Bearer {}", token)
            }
            JiraAuthType::Basic => {
                let email = email.ok_or_else(|| anyhow!("Email required for Basic auth"))?;
                let credentials = format!("{}:{}", email, token);
                let encoded = BASE64.encode(credentials.as_bytes());
                format!("Basic {}", encoded)
            }
        };
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&auth_value)?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()?;

        Ok(Self { base_url, client })
    }

    /// Get current user information
    pub async fn get_myself(&self) -> Result<JiraUser> {
        let url = format!("{}/rest/api/2/myself", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Jira API error {}: {}", status, text));
        }

        let user: JiraUser = response.json().await?;
        Ok(user)
    }

    /// Get issue information
    pub async fn get_issue(&self, issue_key: &str) -> Result<Option<JiraIssue>> {
        let url = format!("{}/rest/api/2/issue/{}", self.base_url, issue_key);
        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Jira API error {}: {}", status, text));
        }

        let issue: JiraIssue = response.json().await?;
        Ok(Some(issue))
    }

    /// Validate an issue key exists
    pub async fn validate_issue_key(&self, issue_key: &str) -> Result<(bool, String)> {
        match self.get_issue(issue_key).await? {
            Some(issue) => {
                let summary = issue.fields.summary.unwrap_or_else(|| "Unknown".to_string());
                Ok((true, summary))
            }
            None => Ok((false, "Issue not found".to_string())),
        }
    }

    /// Add worklog to Jira issue (using Jira native worklog API)
    pub async fn add_worklog(&self, entry: &WorklogEntry) -> Result<WorklogResponse> {
        let url = format!("{}/rest/api/2/issue/{}/worklog", self.base_url, entry.issue_key);

        let started = format_jira_datetime(&entry.date);

        let payload = serde_json::json!({
            "timeSpentSeconds": entry.time_spent_seconds,
            "comment": entry.description,
            "started": started
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Jira worklog error {}: {}", status, text));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(WorklogResponse {
            id: result.get("id").and_then(|v| v.as_str().map(String::from)),
            tempo_worklog_id: None,
        })
    }

    /// Get group members from Jira
    pub async fn get_group_members(&self, group_name: &str) -> Result<Vec<JiraUser>> {
        let mut members = Vec::new();
        let mut start_at = 0;
        let max_results = 50;

        loop {
            let url = format!("{}/rest/api/2/group/member", self.base_url);
            let response = self.client
                .get(&url)
                .query(&[
                    ("groupname", group_name),
                    ("startAt", &start_at.to_string()),
                    ("maxResults", &max_results.to_string()),
                ])
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(anyhow!("Jira group API error {}: {}", status, text));
            }

            let data: serde_json::Value = response.json().await?;

            if let Some(values) = data.get("values").and_then(|v| v.as_array()) {
                for value in values {
                    if let Ok(user) = serde_json::from_value::<JiraUser>(value.clone()) {
                        members.push(user);
                    }
                }
            }

            let is_last = data.get("isLast").and_then(|v| v.as_bool()).unwrap_or(true);
            if is_last {
                break;
            }
            start_at += max_results;
        }

        Ok(members)
    }

    /// Batch get issue types for multiple issues
    pub async fn batch_get_issue_types(&self, issue_keys: &[String]) -> Result<std::collections::HashMap<String, String>> {
        let mut result = std::collections::HashMap::new();
        if issue_keys.is_empty() {
            return Ok(result);
        }

        let batch_size = 50;
        for chunk in issue_keys.chunks(batch_size) {
            let jql = format!("key in ({})", chunk.join(","));
            let url = format!("{}/rest/api/2/search", self.base_url);

            match self.client
                .get(&url)
                .query(&[
                    ("jql", jql.as_str()),
                    ("fields", "issuetype"),
                    ("maxResults", &batch_size.to_string()),
                ])
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        if let Some(issues) = data.get("issues").and_then(|v| v.as_array()) {
                            for issue in issues {
                                if let (Some(key), Some(issue_type)) = (
                                    issue.get("key").and_then(|v| v.as_str()),
                                    issue
                                        .get("fields")
                                        .and_then(|f| f.get("issuetype"))
                                        .and_then(|t| t.get("name"))
                                        .and_then(|n| n.as_str()),
                                ) {
                                    result.insert(key.to_string(), issue_type.to_string());
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Mark as Unknown if batch query fails
                    for key in chunk {
                        if !result.contains_key(key) {
                            result.insert(key.clone(), "Unknown".to_string());
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Tempo Timesheets API client
pub struct TempoClient {
    base_url: String,
    client: Client,
}

impl TempoClient {
    /// Create a new Tempo client
    pub fn new(base_url: &str, api_token: &str) -> Result<Self> {
        let base_url = base_url.trim_end_matches('/').to_string();

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", api_token))?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()?;

        Ok(Self { base_url, client })
    }

    /// Get worklogs for a date range
    pub async fn get_worklogs(&self, date_from: &str, date_to: &str) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/rest/tempo-timesheets/4/worklogs", self.base_url);
        let response = self.client
            .get(&url)
            .query(&[("dateFrom", date_from), ("dateTo", date_to)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Tempo API error {}: {}", status, text));
        }

        let worklogs: Vec<serde_json::Value> = response.json().await?;
        Ok(worklogs)
    }

    /// Get worklogs for a specific user
    pub async fn get_worklogs_for_user(
        &self,
        account_id: &str,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/rest/tempo-timesheets/4/worklogs", self.base_url);
        let response = self.client
            .get(&url)
            .query(&[
                ("worker", account_id),
                ("dateFrom", date_from),
                ("dateTo", date_to),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Tempo API error {}: {}", status, text));
        }

        let worklogs: Vec<serde_json::Value> = response.json().await?;
        Ok(worklogs)
    }

    /// Create a worklog in Tempo
    pub async fn create_worklog(&self, entry: &WorklogEntry) -> Result<WorklogResponse> {
        let url = format!("{}/rest/tempo-timesheets/4/worklogs", self.base_url);

        let payload = serde_json::json!({
            "issueKey": entry.issue_key,
            "timeSpentSeconds": entry.time_spent_seconds,
            "startDate": entry.date,
            "startTime": "09:00:00",
            "description": entry.description,
            "authorAccountId": entry.account_id
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Tempo worklog error {}: {}", status, text));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(WorklogResponse {
            id: result.get("id").and_then(|v| v.as_str().map(String::from)),
            tempo_worklog_id: result.get("tempoWorklogId").and_then(|v| v.as_i64()),
        })
    }

    /// Get all Tempo teams
    pub async fn get_teams(&self) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/rest/tempo-teams/2/team", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Tempo teams API error {}: {}", status, text));
        }

        let teams: Vec<serde_json::Value> = response.json().await?;
        Ok(teams)
    }

    /// Get team members for a specific team
    pub async fn get_team_members(&self, team_id: i64) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/rest/tempo-teams/2/team/{}/member", self.base_url, team_id);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Tempo team members API error {}: {}", status, text));
        }

        let members: Vec<serde_json::Value> = response.json().await?;
        Ok(members)
    }
}

/// Worklog uploader - unified interface for Jira and Tempo
pub struct WorklogUploader {
    jira: JiraClient,
    tempo: Option<TempoClient>,
    account_id: Option<String>,
}

impl WorklogUploader {
    /// Create a new worklog uploader
    pub fn new(
        jira_url: &str,
        token: &str,
        email: Option<&str>,
        auth_type: &str,
        tempo_token: Option<&str>,
    ) -> Result<Self> {
        let jira = JiraClient::new(jira_url, token, email, JiraAuthType::from(auth_type))?;
        let tempo = tempo_token
            .map(|t| TempoClient::new(jira_url, t))
            .transpose()?;

        Ok(Self {
            jira,
            tempo,
            account_id: None,
        })
    }

    /// Get current user's account ID
    pub async fn get_account_id(&mut self) -> Result<String> {
        if let Some(ref id) = self.account_id {
            return Ok(id.clone());
        }

        let user = self.jira.get_myself().await?;
        let id = user.get_identifier()
            .ok_or_else(|| anyhow!("Could not determine user identifier"))?;
        self.account_id = Some(id.clone());
        Ok(id)
    }

    /// Validate an issue
    pub async fn validate_issue(&self, issue_key: &str) -> Result<(bool, String)> {
        self.jira.validate_issue_key(issue_key).await
    }

    /// Upload a worklog
    pub async fn upload_worklog(&mut self, mut entry: WorklogEntry, use_tempo: bool) -> Result<WorklogResponse> {
        // Ensure account_id is set
        if entry.account_id.is_none() {
            entry.account_id = Some(self.get_account_id().await?);
        }

        if use_tempo {
            if let Some(ref tempo) = self.tempo {
                return tempo.create_worklog(&entry).await;
            }
        }

        self.jira.add_worklog(&entry).await
    }

    /// Test connection
    pub async fn test_connection(&self) -> Result<(bool, String)> {
        match self.jira.get_myself().await {
            Ok(user) => {
                let display_name = user.display_name
                    .or(user.name)
                    .unwrap_or_else(|| "Unknown".to_string());
                Ok((true, format!("Connected as: {}", display_name)))
            }
            Err(e) => Ok((false, format!("Connection failed: {}", e))),
        }
    }
}

/// Format date string to Jira datetime format
fn format_jira_datetime(date_str: &str) -> String {
    // Jira requires ISO 8601 format: 2025-12-31T09:00:00.000+0800
    format!("{}T09:00:00.000+0800", date_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_jira_datetime() {
        let result = format_jira_datetime("2025-12-31");
        assert_eq!(result, "2025-12-31T09:00:00.000+0800");
    }

    #[test]
    fn test_jira_auth_type_from_str() {
        assert_eq!(JiraAuthType::from("pat"), JiraAuthType::Pat);
        assert_eq!(JiraAuthType::from("PAT"), JiraAuthType::Pat);
        assert_eq!(JiraAuthType::from("basic"), JiraAuthType::Basic);
        assert_eq!(JiraAuthType::from("BASIC"), JiraAuthType::Basic);
        assert_eq!(JiraAuthType::from("unknown"), JiraAuthType::Pat);
    }
}
