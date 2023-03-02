use chrono::{DateTime, Utc};
use serde::{ser::SerializeSeq, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    raw_event::{RawEvent, Tag},
    Pubkey,
};

pub struct CanonicalEvent {
    pub pubkey: Pubkey,
    pub created_at: DateTime<Utc>,
    pub kind: u32,
    pub tags: Vec<Tag>,
    pub content: String,
}

impl CanonicalEvent {
    pub fn to_sha256(&self) -> [u8; 32] {
        let canonical_event = self.to_string();
        let mut hasher = Sha256::new();
        hasher.update(canonical_event.as_bytes());
        let mut hash = [0; 32];
        hash.copy_from_slice(hasher.finalize().as_slice());
        hash
    }
}

impl ToString for CanonicalEvent {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

impl From<RawEvent> for CanonicalEvent {
    fn from(raw_event: RawEvent) -> Self {
        CanonicalEvent {
            pubkey: raw_event.pubkey,
            created_at: raw_event.created_at,
            kind: raw_event.kind,
            tags: raw_event.tags,
            content: raw_event.content,
        }
    }
}

impl Serialize for CanonicalEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(6))?;
        seq.serialize_element(&0)?;
        seq.serialize_element(&self.pubkey)?;
        let created_at = self.created_at.timestamp();
        seq.serialize_element(&created_at)?;
        seq.serialize_element(&self.kind)?;
        seq.serialize_element(&self.tags)?;
        seq.serialize_element(&self.content)?;
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn to_json() {
        let canonical_event = CanonicalEvent {
            pubkey: Pubkey::new([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x12, 0x1a, 0xa0, 0xff, 0x01, 0x02,
                0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x12, 0x1a, 0xa0, 0xff, 0x01, 0x02, 0x03, 0x04,
                0x05, 0x06, 0x07, 0x08,
            ]),
            created_at: Utc.timestamp_opt(1677538187, 0).unwrap(),
            kind: 1,
            tags: vec![],
            content: "content".to_string(),
        };

        let got = serde_json::to_string(&canonical_event).unwrap();
        assert_eq!(
            got,
            r#"[0,"0102030405060708121aa0ff0102030405060708121aa0ff0102030405060708",1677538187,1,[],"content"]"#
        )
    }

    #[test]
    fn to_sha256() {
        let canonical_event = CanonicalEvent {
            pubkey: Pubkey::new([
                0x73, 0x49, 0x15, 0x09, 0xb8, 0xe2, 0xd8, 0x08, 0x40, 0x87, 0x3b, 0x5a, 0x13, 0xba,
                0x98, 0xa5, 0xd1, 0xac, 0x3a, 0x16, 0xc9, 0x29, 0x2e, 0x10, 0x6b, 0x1f, 0x2e, 0xda,
                0x31, 0x15, 0x2c, 0x52,
            ]),
            created_at: Utc.timestamp_opt(1677711753, 0).unwrap(),
            kind: 1,
            tags: vec![],
            content: "おはのすー".to_string(),
        };

        let got = canonical_event.to_sha256();
        let want: [u8; 32] = [
            0xb8, 0xe9, 0x21, 0x46, 0xc5, 0xd3, 0xc0, 0x06, 0xb2, 0xde, 0x7b, 0x2a, 0xbb, 0xdb,
            0x5f, 0xb7, 0xb5, 0xbc, 0x39, 0xde, 0xc4, 0x78, 0xa9, 0x73, 0x93, 0x36, 0x94, 0x99,
            0x95, 0x2e, 0xbb, 0x62,
        ];

        assert_eq!(got, want);
    }
}
