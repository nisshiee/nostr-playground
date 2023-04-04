use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::serde::bytes::to_string;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventSignature(#[serde(with = "crate::serde::bytes")] [u8; 64]);

impl EventSignature {
    pub fn new(pubkey: [u8; 64]) -> Self {
        Self(pubkey)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Default for EventSignature {
    fn default() -> Self {
        Self([0; 64])
    }
}

impl Debug for EventSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EventSignature")
            .field(&to_string(&self.0))
            .finish()
    }
}
