use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pubkey(#[serde(with = "crate::serde::bytes")] [u8; 32]);

impl Pubkey {
    pub fn new(pubkey: [u8; 32]) -> Self {
        Self(pubkey)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
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
