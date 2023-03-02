use chrono::{DateTime, Utc};
use ecdsa::signature::Verifier;
use k256::schnorr::{Signature, VerifyingKey};
// use ecdsa::{
//     elliptic_curve::{NonZeroScalar, PublicKey},
//     signature::Verifier,
//     Signature, VerifyingKey,
// };
// use k256::Secp256k1;
use serde::{Deserialize, Serialize};

use crate::{canonical_event, serde::bytes::to_string, CanonicalEvent, Pubkey};

mod tag;
pub use tag::Tag;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawEvent {
    #[serde(with = "crate::serde::bytes")]
    pub id: [u8; 32],
    pub pubkey: Pubkey,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub kind: u32,
    pub tags: Vec<Tag>,
    pub content: String,
    #[serde(with = "crate::serde::bytes")]
    pub sig: [u8; 64],
}

impl RawEvent {
    pub fn verify(self) -> bool {
        let Ok(verifying_key) = VerifyingKey::from_bytes(self.pubkey.as_slice()) else { return false };
        let Ok(signature) = Signature::try_from(self.sig.as_slice()) else { return false };

        let canonical_event = CanonicalEvent::from(self);
        let canonical_event = canonical_event.to_string();
        let canonical_event = canonical_event.as_bytes();

        verifying_key.verify(canonical_event, &signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_test::{assert_tokens, Token};

    use super::*;

    #[test]
    fn serde() {
        let raw_event = RawEvent {
            id: [
                0xd7, 0x69, 0x25, 0xda, 0xbc, 0x18, 0x1a, 0xb8, 0xae, 0x7a, 0x00, 0x15, 0x2c, 0xe3,
                0xff, 0x63, 0x45, 0x05, 0xf4, 0xcf, 0x7f, 0x38, 0x41, 0xd3, 0xe9, 0x47, 0x33, 0x37,
                0x37, 0x43, 0x73, 0x74,
            ],
            pubkey: Pubkey::new([
                0x73, 0x49, 0x15, 0x09, 0xb8, 0xe2, 0xd8, 0x08, 0x40, 0x87, 0x3b, 0x5a, 0x13, 0xba,
                0x98, 0xa5, 0xd1, 0xac, 0x3a, 0x16, 0xc9, 0x29, 0x2e, 0x10, 0x6b, 0x1f, 0x2e, 0xda,
                0x31, 0x15, 0x2c, 0x52,
            ]),
            created_at: Utc.timestamp_opt(1677538187, 0).unwrap(),
            kind: 1,
            tags: vec![Tag {
                name: "t".to_owned(),
                value: "#test".to_owned(),
                parameters: vec![],
            }],
            content: "おはノス".to_string(),
            sig: [
                0xf1, 0x73, 0xac, 0x92, 0x6d, 0x93, 0x61, 0x3b, 0xc1, 0xc8, 0x08, 0xe5, 0xe7, 0x76,
                0x2c, 0x88, 0xb1, 0x3a, 0x0f, 0x47, 0xa3, 0xca, 0x8b, 0x43, 0x7c, 0x2d, 0x76, 0xc9,
                0xaf, 0xaf, 0xfa, 0xc6, 0xfd, 0x72, 0xb0, 0x03, 0x17, 0xc7, 0x79, 0x9c, 0x6c, 0x54,
                0x43, 0x54, 0x4d, 0xad, 0x46, 0xe0, 0xd7, 0x7c, 0x1d, 0x23, 0x8f, 0xc0, 0x49, 0x66,
                0xdd, 0x56, 0x22, 0x30, 0xd8, 0xe7, 0x9c, 0x79,
            ],
        };
        let serialized = [
            Token::Struct { name: "RawEvent", len: 7 },
            Token::Str("id"),
            Token::Str("d76925dabc181ab8ae7a00152ce3ff634505f4cf7f3841d3e947333737437374"),
            Token::Str("pubkey"),
            Token::Str("73491509b8e2d80840873b5a13ba98a5d1ac3a16c9292e106b1f2eda31152c52"),
            Token::Str("created_at"),
            Token::I64(1677538187),
            Token::Str("kind"),
            Token::U32(1),
            Token::Str("tags"),
            Token::Seq { len: Some(1) },
            Token::Seq { len: Some(2) },
            Token::Str("t"),
            Token::Str("#test"),
            Token::SeqEnd,
            Token::SeqEnd,
            Token::Str("content"),
            Token::Str("おはノス"),
            Token::Str("sig"),
            Token::Str("f173ac926d93613bc1c808e5e7762c88b13a0f47a3ca8b437c2d76c9afaffac6fd72b00317c7799c6c5443544dad46e0d77c1d238fc04966dd562230d8e79c79"),
            Token::StructEnd
        ];

        assert_tokens(&raw_event, &serialized);
    }

    #[test]
    fn verify() {
        let raw_event = RawEvent {
            id: [
                0xb8, 0xe9, 0x21, 0x46, 0xc5, 0xd3, 0xc0, 0x06, 0xb2, 0xde, 0x7b, 0x2a, 0xbb, 0xdb,
                0x5f, 0xb7, 0xb5, 0xbc, 0x39, 0xde, 0xc4, 0x78, 0xa9, 0x73, 0x93, 0x36, 0x94, 0x99,
                0x95, 0x2e, 0xbb, 0x62,
            ],
            pubkey: Pubkey::new([
                0x73, 0x49, 0x15, 0x09, 0xb8, 0xe2, 0xd8, 0x08, 0x40, 0x87, 0x3b, 0x5a, 0x13, 0xba,
                0x98, 0xa5, 0xd1, 0xac, 0x3a, 0x16, 0xc9, 0x29, 0x2e, 0x10, 0x6b, 0x1f, 0x2e, 0xda,
                0x31, 0x15, 0x2c, 0x52,
            ]),
            created_at: Utc.timestamp_opt(1677711753, 0).unwrap(),
            kind: 1,
            tags: vec![],
            content: "おはのすー".to_string(),
            sig: [
                0x10, 0xc6, 0x15, 0x4f, 0xd6, 0x36, 0x54, 0xf7, 0xd7, 0xbd, 0x24, 0xa0, 0x9e, 0xf4,
                0x05, 0x42, 0xdb, 0x81, 0x1f, 0xf0, 0x64, 0x1a, 0x94, 0x2f, 0x50, 0x78, 0xf1, 0x56,
                0xcc, 0x23, 0x72, 0xb7, 0xd5, 0x33, 0xae, 0x5e, 0x66, 0xe6, 0x10, 0x13, 0x9a, 0xd3,
                0xdb, 0xfe, 0xed, 0x38, 0x7e, 0x86, 0xbf, 0xaf, 0x09, 0x25, 0x03, 0x6b, 0xce, 0x8c,
                0x56, 0xca, 0x75, 0x5a, 0x57, 0x60, 0xb1, 0x74,
            ],
        };

        assert!(raw_event.verify());
    }
}
