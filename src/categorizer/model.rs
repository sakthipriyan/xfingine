use serde::{Deserialize, Serialize};

/// A rule to map a transaction to a category and merchant.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CategoryRule {
    /// Substring to look for in the transaction text (case-insensitive).
    pub pattern: String,
    /// The category to assign (e.g., "Food/Delivery").
    pub category: String,
    /// The merchant to assign (e.g., "Swiggy").
    pub merchant: String,
}

/// A raw transaction input before mapping.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapInput {
    /// The raw transaction description from the bank.
    pub narration: String,
}

/// The result of mapping a transaction.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapOutput {
    /// The matched category path.
    pub category: String,
    /// The matched merchant name.
    pub merchant: String,
}

/// Input payload for the Map operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorizeRequest {
    /// Ordered list of inputs to categorize.
    pub data: Vec<MapInput>,
    /// Rules to evaluate.
    pub rules: Vec<CategoryRule>,
}

/// Output payload from the Map operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorizeResponse {
    /// Positional mapping results. `None` (JSON null) if no rule matched.
    pub data: Vec<Option<MapOutput>>,
}

/// Input for a manually categorized transaction to derive rules from.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeriveInput {
    /// The original transaction text.
    pub narration: String,
    /// The category the user assigned.
    pub category: String,
    /// The merchant the user assigned.
    pub merchant: String,
}

/// Input payload for the Derive Rules operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeriveRulesRequest {
    /// The manually mapped entries.
    pub data: Vec<DeriveInput>,
}

/// Output payload containing suggested new rules.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeriveRulesResponse {
    /// Suggested mapping rules.
    pub rules: Vec<CategoryRule>,
}
