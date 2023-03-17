use k256::schnorr::SigningKey;
use serde::{Deserialize, Serialize};

use crate::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seckey(#[serde(with = "crate::serde::bytes")] [u8; 32]);

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
