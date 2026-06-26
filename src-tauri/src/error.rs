use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("invalid visibility: {0}")]
    InvalidVisibility(String),

    #[error("invalid theme: {0}")]
    InvalidTheme(String),

    #[error("provider not found: id={0}")]
    ProviderNotFound(i64),

    #[error("category not found: id={0}")]
    CategoryNotFound(i64),

    #[error("category is default and cannot be deleted: id={0}")]
    CategoryIsDefault(i64),

    #[error("provider {0} cannot be tested")]
    ProviderCannotTest(String),

    #[error("provider {0} does not support fetch_quota")]
    ProviderQuotaUnsupported(String),

    #[error("http error: {0}")]
    Http(String),

    #[error("invalid token usage format: {0}")]
    TokenUsageInvalidFormat(String),

    #[error("duplicate token usage record: {0}")]
    TokenUsageDuplicate(String),

    #[error("pricing not found for model: {0}")]
    TokenUsagePricingNotFound(String),

    #[error("action validation error: {0}")]
    ActionValidation(String),

    #[error("unknown action: {0}")]
    ActionNotFound(String),
}

// Tauri command serialization: { code: String, message: String }
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        let code = match self {
            Self::Database(_) => "DATABASE",
            Self::Io(_) => "IO",
            Self::Serde(_) => "SERDE",
            Self::InvalidVisibility(_) => "INVALID_VISIBILITY",
            Self::InvalidTheme(_) => "INVALID_THEME",
            Self::ProviderNotFound(_) => "PROVIDER_NOT_FOUND",
            Self::CategoryNotFound(_) => "CATEGORY_NOT_FOUND",
            Self::CategoryIsDefault(_) => "CATEGORY_IS_DEFAULT",
            Self::ProviderCannotTest(_) => "PROVIDER_CANNOT_TEST",
            Self::ProviderQuotaUnsupported(_) => "PROVIDER_QUOTA_UNSUPPORTED",
            Self::Http(_) => "HTTP",
            Self::TokenUsageInvalidFormat(_) => "TOKEN_USAGE_INVALID_FORMAT",
            Self::TokenUsageDuplicate(_) => "TOKEN_USAGE_DUPLICATE",
            Self::TokenUsagePricingNotFound(_) => "TOKEN_USAGE_PRICING_NOT_FOUND",
            Self::ActionValidation(_) => "ACTION_VALIDATION",
            Self::ActionNotFound(_) => "ACTION_NOT_FOUND",
        };
        s.serialize_field("code", code)?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
