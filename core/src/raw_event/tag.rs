use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "Vec<String>", try_from = "Vec<String>")]
pub struct Tag {
    pub name: String,
    pub value: String,
    pub parameters: Vec<String>,
}

impl From<Tag> for Vec<String> {
    fn from(tag: Tag) -> Self {
        let mut ret = vec![tag.name, tag.value];
        ret.extend(tag.parameters);
        ret
    }
}

impl TryFrom<Vec<String>> for Tag {
    type Error = anyhow::Error;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        if value.len() < 2 {
            anyhow::bail!("Tag must have at least 2 elements");
        }

        let mut parameters = value.clone();
        let name = parameters.remove(0);
        let value = parameters.remove(0);

        Ok(Tag {
            name,
            value,
            parameters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn serde() {
        let tag = Tag {
            name: "name".to_string(),
            value: "value".to_string(),
            parameters: vec!["param1".to_string(), "param2".to_string()],
        };

        assert_tokens(
            &tag,
            &[
                Token::Seq { len: Some(4) },
                Token::String("name"),
                Token::String("value"),
                Token::String("param1"),
                Token::String("param2"),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn serde_no_params() {
        let tag = Tag {
            name: "name".to_string(),
            value: "value".to_string(),
            parameters: vec![],
        };

        assert_tokens(
            &tag,
            &[
                Token::Seq { len: Some(2) },
                Token::String("name"),
                Token::String("value"),
                Token::SeqEnd,
            ],
        );
    }
}
