use serde::{ser::SerializeSeq, Serialize};

use crate::{RawEvent, SubscriptionId};

pub enum Response {
    Event {
        subscription_id: SubscriptionId,
        event: RawEvent,
    },
    Notice(String),
}

impl Response {
    pub fn type_str(&self) -> &'static str {
        match self {
            Response::Event { .. } => "EVENT",
            Response::Notice(_) => "NOTICE",
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
        }

        seq.end()
    }
}
