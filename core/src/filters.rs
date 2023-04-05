use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::Filter;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Filters(Vec<Filter>);

impl Filters {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Deref for Filters {
    type Target = Vec<Filter>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Filters {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
