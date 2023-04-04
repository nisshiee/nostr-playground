use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use k256::schnorr::SigningKey;
use serde::{Deserialize, Serialize};

use crate::{
    util::{bytes_to_string, string_to_bytes, BytesParseError, BytesVisitor},
    Pubkey,
};

#[derive(Clone, Copy, PartialEq, Eq)]

pub struct Seckey([u8; 32]);

impl Seckey {
    pub fn new(pubkey: [u8; 32]) -> Self {
        Self(pubkey)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_pubkey(&self) -> Pubkey {
        let signing_key = SigningKey::from_bytes(&self.0).unwrap();
        let verifying_key = signing_key.verifying_key();
        Pubkey::new(verifying_key.to_bytes().into())
    }
}

impl Display for Seckey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bytes_to_string(&self.0))
    }
}

impl Debug for Seckey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Seckey").field(&self.to_string()).finish()
    }
}

impl Serialize for Seckey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for Seckey {
    type Err = BytesParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        string_to_bytes(s).map(Self)
    }
}

impl<'de> Deserialize<'de> for Seckey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(BytesVisitor::<32>).map(Self)
    }
}
