use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize,
};

use crate::{RawEvent, SubscriptionId};

pub enum Response {
    Event {
        subscription_id: SubscriptionId,
        event: RawEvent,
    },
    Notice(String),
    Eose(SubscriptionId),
}

impl Response {
    pub fn type_str(&self) -> &'static str {
        match self {
            Response::Event { .. } => "EVENT",
            Response::Notice(_) => "NOTICE",
            Response::Eose(_) => "EOSE",
        }
    }
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let len = match self {
            Response::Event { .. } => 3,
            Response::Notice(_) => 2,
            Response::Eose(_) => 2,
        };
        let mut seq = serializer.serialize_seq(Some(len))?;

        let r#type = self.type_str();
        seq.serialize_element(r#type)?;

        match self {
            Response::Event {
                subscription_id,
                event,
            } => {
                seq.serialize_element(subscription_id)?;
                seq.serialize_element(event)?;
            }
            Response::Notice(notice) => {
                seq.serialize_element(notice)?;
            }
            Response::Eose(subscription_id) => {
                seq.serialize_element(subscription_id)?;
            }
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;
        impl<'de> Visitor<'de> for SeqVisitor {
            type Value = Response;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("NIP-01: Communication from relay to client format")
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
                    "EVENT" => {
                        let Some(subscription_id) = seq.next_element::<SubscriptionId>()? else {
                            return Err(de::Error::invalid_length(1, &self));
                        };
                        let Some(event) = seq.next_element::<RawEvent>()? else {
                            return Err(de::Error::invalid_length(2, &self));
                        };
                        Ok(Response::Event {
                            subscription_id,
                            event,
                        })
                    }
                    "NOTICE" => {
                        let Some(notice) = seq.next_element::<String>()? else {
                            return Err(de::Error::invalid_length(1, &self));
                        };
                        Ok(Response::Notice(notice))
                    }
                    "EOSE" => {
                        let Some(subscription_id) = seq.next_element::<SubscriptionId>()? else {
                            return Err(de::Error::invalid_length(1, &self));
                        };
                        Ok(Response::Eose(subscription_id))
                    }
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(r#type),
                        &"EVENT, NOTICE or EOSE",
                    )),
                }
            }
        }

        deserializer.deserialize_seq(SeqVisitor)
    }
}
