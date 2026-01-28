//! Safe SQL query builder
//!
//! Helper for building parameterized SQL queries safely (prevents SQL injection)

use sqlx::{sqlite::SqliteRow, FromRow, SqlitePool};

/// Represents a single WHERE condition with its bound value
#[derive(Clone)]
pub enum BindValue {
    String(String),
    Int(i64),
}

/// Query builder that constructs parameterized queries
pub struct SafeQueryBuilder {
    conditions: Vec<String>,
    bindings: Vec<BindValue>,
}

impl SafeQueryBuilder {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            bindings: Vec::new(),
        }
    }

    /// Add a condition with a string value
    pub fn add_string_condition(&mut self, column: &str, op: &str, value: &str) {
        self.conditions.push(format!("{} {} ?", column, op));
        self.bindings.push(BindValue::String(value.to_string()));
    }

    /// Add a condition with an integer value
    pub fn add_int_condition(&mut self, column: &str, op: &str, value: i64) {
        self.conditions.push(format!("{} {} ?", column, op));
        self.bindings.push(BindValue::Int(value));
    }

    /// Add a NULL check condition (no binding needed)
    pub fn add_null_condition(&mut self, column: &str, is_null: bool) {
        if is_null {
            self.conditions.push(format!("{} IS NULL", column));
        } else {
            self.conditions.push(format!("{} IS NOT NULL", column));
        }
    }

    /// Add a raw SQL condition (no additional bindings)
    /// Safety: Caller must ensure no user input is interpolated into the SQL string.
    pub fn add_raw_condition(&mut self, condition: &str) {
        self.conditions.push(condition.to_string());
    }

    /// Build the WHERE clause
    pub fn build_where_clause(&self) -> String {
        if self.conditions.is_empty() {
            "1=1".to_string()
        } else {
            self.conditions.join(" AND ")
        }
    }

    /// Get the bindings for testing
    #[cfg(test)]
    pub fn bindings(&self) -> &[BindValue] {
        &self.bindings
    }

    /// Get the conditions for testing
    #[cfg(test)]
    pub fn conditions(&self) -> &[String] {
        &self.conditions
    }

    /// Execute a SELECT query and return results
    pub async fn fetch_all<T>(
        &self,
        pool: &SqlitePool,
        base_query: &str,
        order_by: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<T>, String>
    where
        T: for<'r> FromRow<'r, SqliteRow> + Send + Unpin,
    {
        let where_clause = self.build_where_clause();
        let mut sql = format!("{} WHERE {} {}", base_query, where_clause, order_by);

        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT {}", l));
        }
        if let Some(o) = offset {
            sql.push_str(&format!(" OFFSET {}", o));
        }

        let mut query = sqlx::query_as::<_, T>(&sql);

        for binding in &self.bindings {
            query = match binding {
                BindValue::String(s) => query.bind(s),
                BindValue::Int(i) => query.bind(*i),
            };
        }

        query.fetch_all(pool).await.map_err(|e| e.to_string())
    }

    /// Execute a COUNT query
    pub async fn count(&self, pool: &SqlitePool, table: &str) -> Result<i64, String> {
        let where_clause = self.build_where_clause();
        let sql = format!("SELECT COUNT(*) FROM {} WHERE {}", table, where_clause);

        let mut query = sqlx::query_scalar::<_, i64>(&sql);

        for binding in &self.bindings {
            query = match binding {
                BindValue::String(s) => query.bind(s),
                BindValue::Int(i) => query.bind(*i),
            };
        }

        query.fetch_one(pool).await.map_err(|e| e.to_string())
    }
}

impl Default for SafeQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_builder() {
        let builder = SafeQueryBuilder::new();
        assert_eq!(builder.build_where_clause(), "1=1");
    }

    #[test]
    fn test_single_string_condition() {
        let mut builder = SafeQueryBuilder::new();
        builder.add_string_condition("user_id", "=", "user-123");
        assert_eq!(builder.build_where_clause(), "user_id = ?");
        assert_eq!(builder.bindings().len(), 1);
    }

    #[test]
    fn test_single_int_condition() {
        let mut builder = SafeQueryBuilder::new();
        builder.add_int_condition("synced_to_tempo", "=", 1);
        assert_eq!(builder.build_where_clause(), "synced_to_tempo = ?");
    }

    #[test]
    fn test_null_condition_is_null() {
        let mut builder = SafeQueryBuilder::new();
        builder.add_null_condition("parent_id", true);
        assert_eq!(builder.build_where_clause(), "parent_id IS NULL");
        assert_eq!(builder.bindings().len(), 0);
    }

    #[test]
    fn test_null_condition_is_not_null() {
        let mut builder = SafeQueryBuilder::new();
        builder.add_null_condition("jira_issue_key", false);
        assert_eq!(builder.build_where_clause(), "jira_issue_key IS NOT NULL");
    }

    #[test]
    fn test_multiple_conditions() {
        let mut builder = SafeQueryBuilder::new();
        builder.add_string_condition("user_id", "=", "user-123");
        builder.add_string_condition("source", "=", "git");
        builder.add_null_condition("parent_id", true);
        builder.add_int_condition("synced_to_tempo", "=", 0);

        let where_clause = builder.build_where_clause();
        assert!(where_clause.contains("user_id = ?"));
        assert!(where_clause.contains("source = ?"));
        assert!(where_clause.contains("parent_id IS NULL"));
        assert!(where_clause.contains("synced_to_tempo = ?"));
        assert!(where_clause.contains(" AND "));
        assert_eq!(builder.bindings().len(), 3);
    }

    #[test]
    fn test_date_range_conditions() {
        let mut builder = SafeQueryBuilder::new();
        builder.add_string_condition("date", ">=", "2024-01-01");
        builder.add_string_condition("date", "<=", "2024-01-31");

        let where_clause = builder.build_where_clause();
        assert!(where_clause.contains("date >= ?"));
        assert!(where_clause.contains("date <= ?"));
        assert_eq!(builder.bindings().len(), 2);
    }

    #[test]
    fn test_default_impl() {
        let builder = SafeQueryBuilder::default();
        assert_eq!(builder.build_where_clause(), "1=1");
    }
}
