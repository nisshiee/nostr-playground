use serde::de::Visitor;

pub(crate) fn bytes_to_string(bytes: &[u8]) -> String {
    let mut str = String::with_capacity(bytes.len() * 2);
    for byte in bytes.iter() {
        str.push_str(&format!("{:02x}", byte));
    }
    str
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BytesParseError {
    #[error("invalid length")]
    InvalidLength,
    #[error("invalid char")]
    InvalidChar,
}

pub(crate) fn string_to_bytes<const N: usize>(s: &str) -> Result<[u8; N], BytesParseError> {
    if !s
        .as_bytes()
        .iter()
        .all(|b| (0x30..=0x39).contains(b) || (0x61..=0x66).contains(b))
    {
        return Err(BytesParseError::InvalidChar);
    }

    if s.len() != N * 2 {
        return Err(BytesParseError::InvalidLength);
    }

    let mut bytes = [0u8; N];
    for (i, byte) in s.as_bytes().chunks(2).enumerate() {
        bytes[i] = u8::from_str_radix(std::str::from_utf8(byte).unwrap(), 16).unwrap();
    }
    Ok(bytes)
}

pub(crate) struct BytesVisitor<const N: usize>;

impl<'de, const N: usize> Visitor<'de> for BytesVisitor<N> {
    type Value = [u8; N];

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a lowercase hex string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        string_to_bytes(v).map_err(E::custom)
    }
}
