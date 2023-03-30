



use crate::{Connections};

#[derive(Clone)]
pub struct Context {
    pub connections: Connections,
}

impl Context {
    pub fn new() -> Self {
        Self {
            connections: Connections::new(),
        }
    }

    // pub async fn append_connection(&self, connection: Connection) {
    //     let mut connections = self.connections.lock().await;
    //     connections.insert(connection.addr(), connection);
    // }

    // pub async fn remove_connection(&self, addr: SocketAddr) {
    //     let mut connections = self.connections.lock().await;
    //     connections.remove(&addr);
    // }
}
