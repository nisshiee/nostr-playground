use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize, Serializer,
};

use crate::{Filter, RawEvent, SubscriptionId};

#[derive(Debug, Clone)]
pub enum Request {
    Event(RawEvent),
    Req {
        subscription_id: SubscriptionId,
        filters: Vec<Filter>,
    },
    Close(SubscriptionId),
}

impl Request {
    pub fn type_str(&self) -> &'static str {
        match self {
            Request::Req { .. } => "REQ",
            Request::Event(_) => "EVENT",
            Request::Close(_) => "CLOSE",
        }
    }
}

impl Serialize for Request {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = match self {
            Request::Req { filters, .. } => filters.len() + 2,
            Request::Event(_) | Request::Close(_) => 2,
        };
        let mut seq = serializer.serialize_seq(Some(len))?;

        let r#type = self.type_str();
        seq.serialize_element(r#type)?;

        match self {
            Request::Event(event) => {
                seq.serialize_element(event)?;
            }
            Request::Req {
                subscription_id,
                filters,
            } => {
                seq.serialize_element(subscription_id)?;
                for filter in filters {
                    seq.serialize_element(filter)?;
                }
            }
            Request::Close(subscription_id) => {
                seq.serialize_element(subscription_id)?;
            }
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for Request {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;
        impl<'de> Visitor<'de> for SeqVisitor {
            type Value = Request;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("NIP-01: Communication from client to relay format")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let r#type = seq.next_element::<&str>()?;
                let Some(r#type) = r#type else {
                    return Err(de::Error::invalid_length(0, &self));
                };
                match r#type {
                    "REQ" => {
                        let Some(subscription_id) = seq.next_element::<SubscriptionId>()? else {
                            return Err(de::Error::invalid_length(1, &self));
                        };
                        let mut filters = Vec::new();
                        while let Some(filter) = seq.next_element::<Filter>()? {
                            filters.push(filter);
                        }
                        Ok(Request::Req {
                            subscription_id,
                            filters,
                        })
                    }
                    "EVENT" => {
                        let Some(event) = seq.next_element::<RawEvent>()? else {
                            return Err(de::Error::invalid_length(1, &self));
                        };
                        // Note: 一旦、不要な要素が配列に入っていてもスルー
                        Ok(Request::Event(event))
                    }
                    "CLOSE" => {
                        let Some(subscription_id) = seq.next_element::<SubscriptionId>()? else {
                            return Err(de::Error::invalid_length(1, &self));
                        };
                        // Note: 一旦、不要な要素が配列に入っていてもスルー
                        Ok(Request::Close(subscription_id))
                    }
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(r#type),
                        &"REQ, EVENT or CLOSE",
                    )),
                }
            }
        }

        deserializer.deserialize_seq(SeqVisitor)
    }
}
