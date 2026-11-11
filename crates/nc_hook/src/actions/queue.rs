use std::collections::VecDeque;
use std::fmt;
use std::sync::{Mutex, PoisonError};

#[derive(Debug)]
pub struct ActionQueue<A> {
    items: Mutex<VecDeque<A>>,
    capacity: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull {
    pub capacity: usize,
}

impl fmt::Display for QueueFull {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "action queue holds {} already", self.capacity)
    }
}

impl std::error::Error for QueueFull {}

impl<A> ActionQueue<A> {
    pub const fn new(capacity: usize) -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            capacity,
        }
    }

    pub fn enqueue(&self, action: A) -> Result<(), QueueFull> {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        if items.len() >= self.capacity {
            return Err(QueueFull {
                capacity: self.capacity,
            });
        }
        items.push_back(action);
        Ok(())
    }

    pub fn take_all(&self) -> Vec<A> {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        items.drain(..).collect()
    }
}
