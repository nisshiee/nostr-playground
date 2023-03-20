use crate::{Filter, RawEvent, SubscriptionId};

pub enum Request {
    Event(RawEvent),
    Req {
        subscription_id: SubscriptionId,
        filters: Vec<Filter>,
    },
    Close(SubscriptionId),
}
