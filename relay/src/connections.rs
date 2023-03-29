use std::{
    collections::HashMap,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use tokio::sync::{Mutex, MutexGuard};
use tokio_tungstenite::tungstenite::Message;

use crate::Connection;

#[derive(Clone)]
pub struct Connections {
    inner: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
}

impl Connections {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn insert(&self, connection: Connection) {
        let mut inner = self.inner.lock().await;
        inner.insert(connection.addr(), connection);
    }

    pub async fn get_connection_mut(&self, addr: SocketAddr) -> Option<ConnectionGuard<'_>> {
        let inner = self.inner.lock().await;
        inner
            .contains_key(&addr)
            .then(|| ConnectionGuard { inner, addr })
    }

    pub async fn close_all(&self) {
        let mut inner = self.inner.lock().await;
        for (_, connection) in inner.iter_mut() {
            connection.close();
        }
    }
}

struct ConnectionGuard<'a> {
    inner: MutexGuard<'a, HashMap<SocketAddr, Connection>>,
    addr: SocketAddr,
}

impl ConnectionGuard<'_> {
    pub fn remove(mut self) {
        self.inner.remove(&self.addr);
    }
}

impl Deref for ConnectionGuard<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.inner.get(&self.addr).unwrap()
    }
}

impl DerefMut for ConnectionGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.get_mut(&self.addr).unwrap()
    }
}
