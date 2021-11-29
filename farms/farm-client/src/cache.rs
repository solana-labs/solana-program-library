//! Farm Client's cache to reduce the load on RPC endpoint.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub const RELOAD_INTERVAL: Duration = Duration::from_secs(604800);

#[derive(Clone)]
pub struct Cache<T> {
    pub data: HashMap<String, T>,
    pub last_load: Instant,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self {
            data: HashMap::<String, T>::new(),
            last_load: Instant::now() - RELOAD_INTERVAL,
        }
    }
}

impl<T> Cache<T> {
    pub fn is_stale(&self) -> bool {
        self.data.is_empty() || Instant::now() - self.last_load >= RELOAD_INTERVAL
    }

    pub fn set(&mut self, data: HashMap<String, T>) {
        self.data = data;
        self.last_load = Instant::now();
    }

    pub fn reset(&mut self) {
        self.data = HashMap::<String, T>::new();
        self.last_load = Instant::now() - RELOAD_INTERVAL;
    }
}
