use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::serde::bytes::to_string;

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(#[serde(with = "crate::serde::bytes")] [u8; 32]);

impl EventId {
    pub fn new(pubkey: [u8; 32]) -> Self {
        Self(pubkey)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EventId").field(&to_string(&self.0)).finish()
    }
}
