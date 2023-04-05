use std::ops::{Deref, DerefMut};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Filter, RawEvent};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Filters(Vec<Filter>);

impl Filters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_since(&self) -> Option<DateTime<Utc>> {
        self.iter().filter_map(|filter| filter.since).min()
    }

    pub fn max_until(&self) -> Option<DateTime<Utc>> {
        self.iter().filter_map(|filter| filter.until).max()
    }

    pub fn is_fit(&self, event: &RawEvent) -> bool {
        if self.is_empty() {
            return true;
        }
        self.iter().any(|filter| filter.is_fit(event))
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
