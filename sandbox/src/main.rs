use std::time::Duration;

use futures_util::{pin_mut, SinkExt, StreamExt};
use nostr_core::{RawEvent, Seckey};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// https://github.com/snapview/tokio-tungstenite/blob/master/examples/client.rs
#[tokio::main]
async fn main() {
    let url = url::Url::parse("ws://localhost:8080").unwrap();
    // let url =
    //     url::Url::parse("ws://nostr-relay-1413080135.ap-northeast-1.elb.amazonaws.com").unwrap();

    let seckey = Seckey::new([
        0x23, 0xaf, 0x29, 0xe0, 0xf8, 0xed, 0xbd, 0x6b, 0xcd, 0x49, 0x8d, 0x00, 0xcb, 0xea, 0x1c,
        0x64, 0xbe, 0x6d, 0x2a, 0x08, 0xe0, 0x25, 0x37, 0xfb, 0xb9, 0x86, 0xd7, 0xa9, 0x7c, 0xf2,
        0x47, 0x0e,
    ]);
    let mut event = RawEvent::new(1, vec![], "test4".to_string());
    event.sign(&seckey);
    let event = [
        serde_json::Value::String("EVENT".to_string()),
        serde_json::to_value(&event).unwrap(),
    ];
    let event = serde_json::to_string(&event).unwrap();

    let req = json!([
            "REQ",
            "sub_id_test",
            {
            "kinds": [1]
        }
    ]);
    let req = serde_json::to_string(&req).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    tokio::task::spawn(async move {
        pin_mut!(read);
        while let Some(msg) = read.next().await {
            println!("Received a message: {:?}", msg);
        }
    });

    pin_mut!(write);

    let message: Message = event.into();
    write.send(message).await.unwrap();
    let message: Message = req.into();
    write.send(message).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    write.send(Message::Ping(vec![])).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    write.send(Message::Close(None)).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    write.close().await.unwrap();
}
// nsec1ywhjnc8cak7khn2f35qvh6suvjlx62sguqjn07aesmt6jl8jgu8q0mm3jv
// 23af29e0f8edbd6bcd498d00cbea1c64be6d2a08e02537fbb986d7a97cf2470e
