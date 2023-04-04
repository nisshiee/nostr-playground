use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{EventId, HexPrefix, Pubkey, RawEvent};

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

impl Filter {
    pub fn is_fit(&self, event: &RawEvent) -> bool {
        if !self.ids.is_empty()
            && !self
                .ids
                .iter()
                .any(|prefix| prefix.is_fit(event.id.as_slice()))
        {
            return false;
        }

        if !self.authors.is_empty()
            && !self
                .authors
                .iter()
                .any(|prefix| prefix.is_fit(event.pubkey.as_slice()))
        {
            return false;
        }

        if !self.kinds.is_empty() && !self.kinds.contains(&event.kind) {
            return false;
        }

        if !self.e_tags.is_empty()
            && !self.e_tags.iter().any(|filter| {
                event
                    .tags
                    .iter()
                    .any(|tag| tag.name == "e" && tag.value == filter.to_string())
            })
        {
            return false;
        }

        if !self.p_tags.is_empty()
            && !self.p_tags.iter().any(|filter| {
                event
                    .tags
                    .iter()
                    .any(|tag| tag.name == "p" && tag.value == filter.to_string())
            })
        {
            return false;
        }

        if let Some(since) = self.since {
            if event.created_at < since {
                return false;
            }
        }

        if let Some(until) = self.until {
            if event.created_at > until {
                return false;
            }
        }

        true
    }
}
