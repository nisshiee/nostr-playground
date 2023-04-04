use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{EventId, HexPrefix, Pubkey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub ids: Vec<HexPrefix>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub authors: Vec<HexPrefix>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub kinds: Vec<u32>,
    #[serde(rename = "#e", skip_serializing_if = "Vec::is_empty", default)]
    pub e_tags: Vec<EventId>,
    #[serde(rename = "#p", skip_serializing_if = "Vec::is_empty", default)]
    pub p_tags: Vec<Pubkey>,
    #[serde(
        with = "chrono::serde::ts_seconds_option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub since: Option<DateTime<Utc>>,
    #[serde(
        with = "chrono::serde::ts_seconds_option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<usize>,
}
