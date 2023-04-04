use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::util::{bytes_to_string, string_to_bytes, BytesParseError, BytesVisitor};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    pub fn new(pubkey: [u8; 32]) -> Self {
        Self(pubkey)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bytes_to_string(&self.0))
    }
}

impl Debug for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Pubkey").field(&self.to_string()).finish()
    }
}

impl Serialize for Pubkey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for Pubkey {
    type Err = BytesParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        string_to_bytes(s).map(Self)
    }
}

impl<'de> Deserialize<'de> for Pubkey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(BytesVisitor::<32>).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn serde() {
        let obj = Pubkey([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x12, 0x1a, 0xa0, 0xff, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x12, 0x1a, 0xa0, 0xff, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08,
        ]);
        assert_tokens(
            &obj,
            &[Token::String(
                "0102030405060708121aa0ff0102030405060708121aa0ff0102030405060708",
            )],
        )
    }
}
