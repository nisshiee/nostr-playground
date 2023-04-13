use std::num::NonZeroUsize;

use lru::LruCache;
use nostr_core::RawEvent;
use tokio::sync::{broadcast, mpsc};

#[derive(Clone)]
pub struct EventBroadcaster {
    sender: mpsc::UnboundedSender<RawEvent>,
    broadcaster: broadcast::Sender<RawEvent>,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        // broadcasterの準備
        let (broadcaster, mut rx) = broadcast::channel(1000);
        // プロセスが生きてる間、受信側を常に起動しておく
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Err(broadcast::error::RecvError::Closed) => break,
                    _ => {} // noop
                }
            }
        });

        // senderの準備
        let (sender, mut rx) = mpsc::unbounded_channel::<RawEvent>();

        // 重複排除
        let outgoing = broadcaster.clone();
        tokio::spawn(async move {
            let mut recent = LruCache::new(NonZeroUsize::new(100).unwrap());
            while let Some(event) = rx.recv().await {
                if recent.contains(&event.id) {
                    continue;
                }

                recent.put(event.id, true);
                outgoing.send(event).ok();
            }
        });

        Self {
            sender,
            broadcaster,
        }
    }

    pub fn send(&self, event: RawEvent) {
        self.sender.send(event).unwrap();
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RawEvent> {
        self.broadcaster.subscribe()
    }
}
