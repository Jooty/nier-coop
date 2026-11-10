#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HelloRule {
    was_connected: bool,
    hero_handle_told_for: Option<u32>,
}

impl HelloRule {
    /// True when the character must be told: on connect, and when a load made a new hero.
    pub fn should_send(&mut self, connected: bool, hero_handle: Option<u32>) -> bool {
        if !connected {
            *self = Self::default();
            return false;
        }
        let just_connected = !self.was_connected;
        self.was_connected = true;
        let hero_changed = hero_handle.is_some()
            && self.hero_handle_told_for.is_some()
            && hero_handle != self.hero_handle_told_for;
        if hero_handle.is_some() {
            self.hero_handle_told_for = hero_handle;
        }
        just_connected || hero_changed
    }
}
