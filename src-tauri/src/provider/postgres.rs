use async_trait::async_trait;
use crate::provider::adapter::{QuotaError, ValidateError};
use crate::types::QuotaSnapshot;
use std::collections::HashMap;

pub struct PostgresAdapter;

#[async_trait]
impl super::ProviderAdapter for PostgresAdapter {
    fn preset(&self) -> &'static str {
        "postgres"
    }

    fn can_test(&self) -> bool {
        false // V0.1 doesn't implement DB connectivity test
    }

    fn can_fetch_quota(&self) -> bool {
        true
    }

    async fn validate_key(&self, _base_url: &str, _api_key: &str) -> Result<(), ValidateError> {
        // Not implemented - can_test() returns false, caller must check first
        Err(ValidateError::Ambiguous)
    }

    async fn fetch_quota(&self, _base_url: &str, api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
        // Parse fields: host, port, database, user, password
        // base_url format: not used, fields are passed via api_key as JSON or we use the HashMap style
        // For PostgreSQL, we expect fields map in a specific format
        // Since we can't easily parse from single string, we expect the caller to pack params
        let params: HashMap<String, String> = serde_json::from_str(api_key)
            .unwrap_or_default();

        let host = params.get("host").cloned().unwrap_or_else(|| "localhost".to_string());
        let port = params.get("port").cloned().unwrap_or_else(|| "5432".to_string());
        let database = params.get("database").cloned().unwrap_or_else(|| "postgres".to_string());
        let user = params.get("user").cloned().unwrap_or_else(|| "postgres".to_string());
        let password = params.get("password").cloned().unwrap_or_default();

        let conn_str = format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, database, user, password
        );

        // Connect using tokio-postgres
        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls)
            .await
            .map_err(|e| QuotaError::Network(format!("connection failed: {}", e)))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("postgres connection error: {}", e);
            }
        });

        // Query pg_database_size
        let row = client
            .query_one("SELECT pg_database_size(current_database())", &[])
            .await
            .map_err(|e| QuotaError::Network(format!("query failed: {}", e)))?;

        let bytes: i64 = row.get(0);
        let size_gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        Ok(QuotaSnapshot {
            total: None,
            used: size_gb,
            remaining: None,
            unit: "GB".to_string(),
            level: None,
            reset_at: None,
        })
    }
}