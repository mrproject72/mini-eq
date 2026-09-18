//! Application instance guard (prevent multiple instances).

use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct InstanceGuard {
    pub running: Arc<Mutex<bool>>,
}

impl InstanceGuard {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(true)),
        }
    }

    pub fn is_running(&self) -> bool {
        match self.running.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }

    pub fn release(&self) {
        if let Ok(mut guard) = self.running.lock() {
            *guard = false;
        }
    }
}
