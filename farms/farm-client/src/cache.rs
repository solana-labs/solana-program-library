//! Farm Client's cache to reduce the load on RPC endpoint.

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

pub const RELOAD_INTERVAL: Duration = Duration::from_secs(21600);

#[derive(Clone)]
pub struct Cache<T> {
    pub data: HashMap<String, T>,
    pub last_load: SystemTime,
    pub counter: u32,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self {
            data: HashMap::<String, T>::new(),
            last_load: SystemTime::now() - RELOAD_INTERVAL,
            counter: 0,
        }
    }
}

impl<T> Cache<T> {
    pub fn is_stale(&self) -> bool {
        if self.data.is_empty() {
            return true;
        }
        if let Ok(diff) = SystemTime::now().duration_since(self.last_load) {
            return diff >= RELOAD_INTERVAL;
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_updated(&self, counter: u32) -> bool {
        counter != self.counter
    }

    pub fn set(&mut self, data: HashMap<String, T>, counter: u32) {
        self.data = data;
        self.last_load = SystemTime::now();
        self.counter = counter;
    }

    pub fn reset(&mut self) {
        self.data = HashMap::<String, T>::new();
        self.last_load = SystemTime::now().checked_sub(RELOAD_INTERVAL).unwrap();
        self.counter = 0;
    }

    pub fn mark_not_stale(&mut self) {
        self.last_load = SystemTime::now();
    }
}
