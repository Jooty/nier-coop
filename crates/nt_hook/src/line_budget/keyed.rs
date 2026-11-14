use std::collections::HashMap;
use std::hash::Hash;

use crate::line_budget::LineBudget;

#[derive(Debug, Clone)]
pub struct KeyedLineBudget<K> {
    budgets: HashMap<K, LineBudget>,
}

impl<K> Default for KeyedLineBudget<K> {
    fn default() -> Self {
        Self {
            budgets: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash> KeyedLineBudget<K> {
    pub fn due(&mut self, key: K, now_ms: u64, gap_ms: u64) -> bool {
        self.budgets.entry(key).or_default().due(now_ms, gap_ms)
    }

    pub fn forget(&mut self, key: &K) {
        self.budgets.remove(key);
    }

    pub fn clear(&mut self) {
        self.budgets.clear();
    }
}
