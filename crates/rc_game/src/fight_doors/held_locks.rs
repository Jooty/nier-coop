use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeldLocks {
    doors: BTreeSet<u16>,
}

impl HeldLocks {
    pub fn hold(&mut self, layout_id: u16) {
        self.doors.insert(layout_id);
    }

    pub fn drop_hold(&mut self, layout_id: u16) -> bool {
        self.doors.remove(&layout_id)
    }

    pub fn is_holding(&self, layout_id: u16) -> bool {
        self.doors.contains(&layout_id)
    }

    pub fn clear(&mut self) {
        self.doors.clear();
    }
}
