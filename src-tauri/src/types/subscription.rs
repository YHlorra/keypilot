










use serde::{Deserialize, Serialize};






#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionQuota {
    
    pub provider_id: String,
    
    pub credential_status: CredentialStatus,
    
    
    pub credential_message: Option<String>,
    
    pub success: bool,
    
    
    pub tiers: Vec<QuotaTier>,
    
    pub error: Option<String>,
    
    
    pub queried_at_ms: i64,
}









#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CredentialStatus {
    Valid,
    Invalid,
    Expired,
    Unknown,
}






#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaTier {
    
    pub kind: QuotaTierKind,
    
    
    pub label: String,
    
    
    pub used: Option<f64>,
    
    pub limit: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_percent: Option<f64>,
    
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at_ms: Option<i64>,
    
    pub reset_description: String,
    
    
    
    pub status: TierStatus,
}





#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaTierKind {
    
    FiveHour,
    
    Weekly,
    
    Monthly,
}





#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TierStatus {
    Active,
    Inactive,
    Unknown,
}
