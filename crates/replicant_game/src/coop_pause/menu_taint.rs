use crate::coop_pause::menu_added_bits;

const MENU_TAINT_TICKS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuTaint {
    slot_zero_without_menu: u32,
    menu_ticks: u32,
    bits: u32,
}

impl MenuTaint {
    pub const fn new() -> Self {
        Self {
            slot_zero_without_menu: 0,
            menu_ticks: 0,
            bits: 0,
        }
    }

    pub fn tick(&mut self, menu_open: bool, slot_zero_now: u32, other_writer_ran: bool) {
        if !menu_open {
            self.slot_zero_without_menu = slot_zero_now;
            self.menu_ticks = 0;
            self.bits = 0;
            return;
        }
        self.menu_ticks = self.menu_ticks.saturating_add(1);
        if other_writer_ran {
            self.bits &= slot_zero_now;
            self.slot_zero_without_menu |= slot_zero_now & !self.bits;
            return;
        }
        if self.menu_ticks <= MENU_TAINT_TICKS {
            self.bits |= menu_added_bits(self.slot_zero_without_menu, slot_zero_now);
        }
    }

    pub const fn bits(&self) -> u32 {
        self.bits
    }
}

#[derive(Debug, Default)]
pub struct TalkWindowTaints {
    entries: Vec<(u64, u32)>,
}

impl TalkWindowTaints {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, window: u64, menu_bits: u32) {
        self.entries.retain(|(known, _)| *known != window);
        if menu_bits != 0 {
            self.entries.push((window, menu_bits));
        }
    }

    pub fn take(&mut self, window: u64) -> u32 {
        let Some(index) = self.entries.iter().position(|(known, _)| *known == window) else {
            return 0;
        };
        self.entries.swap_remove(index).1
    }
}
