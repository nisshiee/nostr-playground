use std::pin::Pin;

use url::Url;

pub struct Connection {
    url: Url,
    sink: Pin,
}
