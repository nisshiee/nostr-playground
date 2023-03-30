use crate::Connections;

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
}
