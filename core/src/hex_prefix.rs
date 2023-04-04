use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use serde::{de::Visitor, Deserialize, Serialize};

#[derive(Clone, Copy)]
pub struct HexPrefix {
    body: [u8; 32],
    len: usize,
}

impl HexPrefix {
    pub fn is_fit(&self, bytes: &[u8]) -> bool {
        if self.len > bytes.len() * 2 {
            return false;
        }
        if self.body[0..self.len / 2] != bytes[0..self.len / 2] {
            return false;
        }
        if self.len % 2 == 0 {
            return true;
        }
        self.body[self.len / 2] >> 4 == bytes[self.len / 2] >> 4
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HexPrefixParseError {
    #[error("invalid length")]
    InvalidLength,
    #[error("invalid char")]
    InvalidChar,
}

impl PartialEq for HexPrefix {
    fn eq(&self, other: &Self) -> bool {
        self.body[0..self.len] == other.body[0..self.len]
    }
}

impl Eq for HexPrefix {}

impl FromStr for HexPrefix {
    type Err = HexPrefixParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s
            .as_bytes()
            .iter()
            .all(|b| (0x30..=0x39).contains(b) || (0x61..=0x66).contains(b))
        {
            return Err(HexPrefixParseError::InvalidChar);
        }
        if s.is_empty() || s.len() > 64 {
            return Err(HexPrefixParseError::InvalidLength);
        }

        let mut body = [0; 32];
        for (i, c) in s.as_bytes().iter().enumerate() {
            let mut mask = if c < &0x40 { c - 0x30 } else { c - 0x60 + 9 };
            if i % 2 == 0 {
                mask <<= 4;
            }
            body[i / 2] |= mask;
        }

        Ok(Self { body, len: s.len() })
    }
}

impl Display for HexPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.len {
            if i % 2 == 0 {
                write!(f, "{:1x}", self.body[i / 2] >> 4)?;
            } else {
                write!(f, "{:1x}", self.body[i / 2] & 0x0f)?;
            }
        }
        Ok(())
    }
}

impl Serialize for HexPrefix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HexPrefix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct HexPrefixVisitor;

        impl Visitor<'_> for HexPrefixVisitor {
            type Value = HexPrefix;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("hex prefix")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(HexPrefixVisitor)
    }
}

impl Debug for HexPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HexPrefix").field(&self.to_string()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_conversion() {
        let hex_prefix = HexPrefix::from_str("1234567890abcdef").unwrap();
        assert_eq!(hex_prefix.to_string(), "1234567890abcdef");
    }

    #[test]
    fn str_conversion_odd_len() {
        let hex_prefix = HexPrefix::from_str("1234567890abcde").unwrap();
        assert_eq!(hex_prefix.to_string(), "1234567890abcde");
    }
}
