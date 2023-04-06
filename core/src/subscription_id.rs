use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubscriptionId(String);

pub const SUBSCRIPTION_ID_MAX_LENGTH: usize = 64;

#[derive(Clone, Debug, thiserror::Error)]
pub enum SubscriptionIdParseError {
    #[error("invalid length")]
    InvalidLength,
}

impl FromStr for SubscriptionId {
    type Err = SubscriptionIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.len() > SUBSCRIPTION_ID_MAX_LENGTH {
            return Err(SubscriptionIdParseError::InvalidLength);
        }
        Ok(Self(s.to_string()))
    }
}
