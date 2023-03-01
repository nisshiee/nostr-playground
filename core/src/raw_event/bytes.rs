use serde::{Deserializer, Serializer};

pub fn serialize<S, const N: usize>(value: &[u8; N], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut str = String::with_capacity(N * 2);
    for byte in value.iter() {
        str.push_str(&format!("{:02x}", byte));
    }
    serializer.serialize_str(&str)
}

struct BytesVisitor<const N: usize>;

impl<const N: usize> serde::de::Visitor<'_> for BytesVisitor<N> {
    type Value = [u8; N];

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(&format!("{N} bytes hex string"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut bytes = [0u8; N];
        for (i, byte) in value.as_bytes().chunks(2).enumerate() {
            bytes[i] = u8::from_str_radix(std::str::from_utf8(byte).unwrap(), 16).unwrap();
        }
        Ok(bytes)
    }
}

pub fn deserialize<'de, D, const N: usize>(deserializer: D) -> Result<[u8; N], D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_string(BytesVisitor::<N>)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_tokens, Token};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Test {
        #[serde(with = "super")]
        bytes: [u8; 12],
    }

    #[test]
    fn serde() {
        let test = Test {
            bytes: [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x12, 0x1a, 0xa0, 0xff,
            ],
        };
        assert_tokens(
            &test,
            &[
                Token::Struct {
                    name: "Test",
                    len: 1,
                },
                Token::Str("bytes"),
                Token::Str("0102030405060708121aa0ff"),
                Token::StructEnd,
            ],
        );
    }
}
