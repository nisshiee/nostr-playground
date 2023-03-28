use serde::{Deserialize, Serialize};
use url::Url;

use crate::Pubkey;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelayInformation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubkey: Option<Pubkey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_nips: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limitation: Option<Limitation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limitation {
    pub max_message_length: u64,
    pub max_subscriptions: u64,
    pub max_filters: u64,
    pub max_limit: u64,
    pub max_subid_length: u64,
    pub min_prefix: u64,
    pub max_event_tags: u64,
    pub max_content_length: u64,
    pub min_pow_difficulty: u64,
    pub auth_required: bool,
    pub payment_required: bool,
}

impl Default for Limitation {
    fn default() -> Self {
        Self {
            max_message_length: 16384,
            max_subscriptions: 20,
            max_filters: 100,
            max_limit: 5000,
            max_subid_length: 100,
            min_prefix: 4,
            max_event_tags: 100,
            max_content_length: 8196,
            min_pow_difficulty: 30,
            auth_required: false,
            payment_required: false,
        }
    }
}
