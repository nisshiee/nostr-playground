use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::util::{bytes_to_string, string_to_bytes, BytesParseError, BytesVisitor};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EventSignature([u8; 64]);

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

impl Display for EventSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bytes_to_string(&self.0))
    }
}

impl Debug for EventSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EventSignature")
            .field(&self.to_string())
            .finish()
    }
}

impl Serialize for EventSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for EventSignature {
    type Err = BytesParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        string_to_bytes(s).map(Self)
    }
}

impl<'de> Deserialize<'de> for EventSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(BytesVisitor::<64>).map(Self)
    }
}
