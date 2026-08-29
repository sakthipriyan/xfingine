use crate::categorizer::model::*;
use crate::error::XfingineError;
use std::collections::{HashMap, HashSet};

/// Applies configured rules to categorize raw transactions.
pub fn categorize_transactions(
    req: &CategorizeRequest,
) -> Result<CategorizeResponse, XfingineError> {
    let mut mapped_transactions = Vec::with_capacity(req.data.len());

    // Pre-lowercase rules for fast matching
    let rules: Vec<(String, &CategoryRule)> = req
        .rules
        .iter()
        .map(|r| (r.pattern.to_lowercase(), r))
        .collect();

    for txn in &req.data {
        let narration_lower = txn.narration.to_lowercase();
        let mut matched_rule = None;

        for (pattern, rule) in &rules {
            if narration_lower.contains(pattern) {
                matched_rule = Some(*rule);
                break; // First match wins
            }
        }

        if let Some(rule) = matched_rule {
            mapped_transactions.push(Some(MapOutput {
                category: rule.category.clone(),
                merchant: rule.merchant.clone(),
            }));
        } else {
            mapped_transactions.push(None);
        }
    }

    Ok(CategorizeResponse {
        data: mapped_transactions,
    })
}

/// Analyzes manually mapped transactions to automatically suggest new rules
/// using a lightweight TF-IDF-inspired token exclusivity algorithm.
pub fn derive_rules(req: &DeriveRulesRequest) -> Result<DeriveRulesResponse, XfingineError> {
    // Group narrations by their mapped category + merchant
    let mut group_to_narrations: HashMap<(String, String), Vec<String>> = HashMap::new();

    for input in &req.data {
        let key = (input.category.clone(), input.merchant.clone());
        group_to_narrations
            .entry(key)
            .or_default()
            .push(input.narration.clone());
    }

    let mut suggested_rules = Vec::new();

    // Helper: extracts unique, non-trivial lowercase tokens from a string
    let tokenize = |s: &str| -> HashSet<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 2) // Ignore tiny noise words like "to", "in", "of"
            .map(|t| t.to_string())
            .collect()
    };

    // Calculate global Document Frequency (DF) for each token
    let mut global_df: HashMap<String, usize> = HashMap::new();
    for narrations in group_to_narrations.values() {
        for narration in narrations {
            for token in tokenize(narration) {
                *global_df.entry(token).or_insert(0) += 1;
            }
        }
    }

    // Evaluate tokens for each group to find the most representative pattern
    for (group_key, narrations) in group_to_narrations {
        let (category, merchant) = group_key;

        // Calculate group-specific Document Frequency
        let mut group_df: HashMap<String, usize> = HashMap::new();
        for narration in &narrations {
            for token in tokenize(narration) {
                *group_df.entry(token).or_insert(0) += 1;
            }
        }

        let mut best_token: Option<String> = None;
        let mut best_score = 0;

        for (token, g_freq) in &group_df {
            let total_freq = global_df.get(token).unwrap();

            // Exclusivity: 1.0 means the token NEVER appears in any other group.
            let exclusivity = (*g_freq as f64) / (*total_freq as f64);

            // Only consider tokens that are highly exclusive to this group
            if exclusivity >= 0.8 {
                // Score = (frequency in group * large weight) + length of token (tie-breaker)
                let score = *g_freq * 1000 + token.len();
                if score > best_score {
                    best_score = score;
                    best_token = Some(token.clone());
                }
            }
        }

        if let Some(token) = best_token {
            suggested_rules.push(CategoryRule {
                pattern: token,
                category,
                merchant,
            });
        }
    }

    // Sort output deterministically
    suggested_rules.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.merchant.cmp(&b.merchant))
    });

    Ok(DeriveRulesResponse {
        rules: suggested_rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_transactions() {
        let req = CategorizeRequest {
            data: vec![
                MapInput {
                    narration: "UPI/SWIGGY/12345/Food".to_string(),
                },
                MapInput {
                    narration: "POS 99 APOLLO CRADLE".to_string(),
                },
                MapInput {
                    narration: "UNKNOWN VENDOR".to_string(),
                },
            ],
            rules: vec![
                CategoryRule {
                    pattern: "swiggy".to_string(),
                    category: "Food/Delivery".to_string(),
                    merchant: "Swiggy".to_string(),
                },
                CategoryRule {
                    pattern: "apollo cradle".to_string(),
                    category: "Health/Hospital".to_string(),
                    merchant: "Apollo Cradle".to_string(),
                },
            ],
        };

        let resp = categorize_transactions(&req).unwrap();

        assert_eq!(resp.data.len(), 3);

        let out0 = resp.data[0].as_ref().unwrap();
        assert_eq!(out0.category, "Food/Delivery");
        assert_eq!(out0.merchant, "Swiggy");

        let out1 = resp.data[1].as_ref().unwrap();
        assert_eq!(out1.category, "Health/Hospital");
        assert_eq!(out1.merchant, "Apollo Cradle");

        // Uncategorized becomes None
        assert!(resp.data[2].is_none());
    }

    #[test]
    fn test_derive_rules() {
        let req = DeriveRulesRequest {
            data: vec![
                DeriveInput {
                    narration: "UPI/SWIGGY/BANGALORE/123".to_string(),
                    category: "Food/Delivery".to_string(),
                    merchant: "Swiggy".to_string(),
                },
                DeriveInput {
                    narration: "SWIGGY INSTAMART POS".to_string(),
                    category: "Food/Delivery".to_string(),
                    merchant: "Swiggy".to_string(),
                },
                DeriveInput {
                    narration: "POS 887 ZEPTO GROCERY".to_string(),
                    category: "Home/Grocery".to_string(),
                    merchant: "Zepto".to_string(),
                },
                DeriveInput {
                    narration: "UPI ZEPTO MUMBAI".to_string(),
                    category: "Home/Grocery".to_string(),
                    merchant: "Zepto".to_string(),
                },
            ],
        };

        let resp = derive_rules(&req).unwrap();

        assert_eq!(resp.rules.len(), 2);

        assert_eq!(resp.rules[0].category, "Food/Delivery");
        assert_eq!(resp.rules[0].merchant, "Swiggy");
        assert_eq!(resp.rules[0].pattern, "swiggy");

        assert_eq!(resp.rules[1].category, "Home/Grocery");
        assert_eq!(resp.rules[1].merchant, "Zepto");
        assert_eq!(resp.rules[1].pattern, "zepto");
    }
}
