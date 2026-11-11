use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

#[derive(Debug)]
pub struct RelayInbox<M> {
    items: Mutex<VecDeque<M>>,
    capacity: usize,
}

impl<M> RelayInbox<M> {
    pub const fn new(capacity: usize) -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            capacity,
        }
    }

    pub fn push(&self, message: M) {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        if items.len() >= self.capacity {
            items.pop_front();
        }
        items.push_back(message);
    }

    pub fn take_matching(&self, wanted: impl Fn(&M) -> bool) -> Vec<M> {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        let (taken, kept): (VecDeque<M>, VecDeque<M>) =
            items.drain(..).partition(|message| wanted(message));
        *items = kept;
        taken.into()
    }

    pub fn clear(&self) {
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    pub fn len(&self) -> usize {
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
