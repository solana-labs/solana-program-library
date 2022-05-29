//! Farm Client's cache to reduce the load on RPC endpoint.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub const RELOAD_INTERVAL: Duration = Duration::from_secs(86400);

#[derive(Clone)]
pub struct Cache<T> {
    pub data: HashMap<String, T>,
    pub last_load: Instant,
    pub counter: u32,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self {
            data: HashMap::<String, T>::new(),
            last_load: Instant::now() - RELOAD_INTERVAL,
            counter: 0,
        }
    }
}

impl<T> Cache<T> {
    pub fn is_stale(&self) -> bool {
        self.data.is_empty() || Instant::now() - self.last_load >= RELOAD_INTERVAL
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_updated(&self, counter: u32) -> bool {
        counter != self.counter
    }

    pub fn set(&mut self, data: HashMap<String, T>, counter: u32) {
        self.data = data;
        self.last_load = Instant::now();
        self.counter = counter;
    }

    pub fn reset(&mut self) {
        self.data = HashMap::<String, T>::new();
        self.last_load = Instant::now() - RELOAD_INTERVAL;
        self.counter = 0;
    }

    pub fn mark_not_stale(&mut self) {
        self.last_load = Instant::now();
    }
}
