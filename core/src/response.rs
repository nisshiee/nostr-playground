use crate::{RawEvent, SubscriptionId};

pub enum Response {
    Event {
        subscription_id: SubscriptionId,
        event: RawEvent,
    },
    Notice(String),
}
