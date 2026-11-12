use crate::address::{
    GameAddress, USE_ITEM_RETURN_IN_AUTO_ITEM_USE_CHIP_CHECK, USE_ITEM_RETURN_IN_MENU_ITEM_LIST,
    USE_ITEM_RETURN_IN_QUICK_ITEM_MENU, USE_ITEM_RETURN_IN_SCRIPT_ITEM_COMMAND,
    USE_ITEM_RETURNS_IN_MENU_ITEM_SCREENS,
};
use crate::downed::{
    AUTO_ITEM_USE_COOL_DOWN_WHILE_HEALTH_IS_HIGH, DownedPhase, ITEM_IDS_THAT_HEAL_THE_HERO,
};
use crate::exe::ExeVariant;

/// The game code that asked the use-item function for an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemUseAsker {
    AutoUseItemChip,
    QuickItemMenu,
    MenuItemList,
    MenuItemScreen,
    ScriptCommand,
    Unknown,
}

/// What the held automatic-use check reads of the hero once per call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoItemUseReading {
    /// -1 when the Auto-Use Item chip is not fitted.
    pub chip_level: i32,
    pub health: i32,
    pub max_health: i32,
    /// The game's own cool-down, in the units of the body's step.
    pub game_cool_down: f32,
    pub step: f32,
}

/// Keeps the game's own cool-down of the automatic item use running while its check is held,
/// so a held use is logged where the game would have used an item.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AutoItemUseHold {
    cool_down_left: Option<f32>,
}

impl ItemUseAsker {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AutoUseItemChip => "the Auto-Use Item chip",
            Self::QuickItemMenu => "the quick item menu",
            Self::MenuItemList => "the menu's item list",
            Self::MenuItemScreen => "a menu item screen",
            Self::ScriptCommand => "a script command",
            Self::Unknown => "unknown game code",
        }
    }
}

impl AutoItemUseHold {
    pub const fn new() -> Self {
        Self {
            cool_down_left: None,
        }
    }

    pub fn forget(&mut self) {
        self.cool_down_left = None;
    }

    /// One held call of the game's check; true when the game would have used an item now, if
    /// one is in stock.
    pub fn held_check_would_use(&mut self, reading: AutoItemUseReading, health_share: f32) -> bool {
        if reading.chip_level < 0 || reading.max_health <= 0 {
            self.forget();
            return false;
        }
        if reading.health as f32 / reading.max_health as f32 >= health_share {
            self.cool_down_left = Some(AUTO_ITEM_USE_COOL_DOWN_WHILE_HEALTH_IS_HIGH);
            return false;
        }
        let left = self.cool_down_left.unwrap_or(reading.game_cool_down) - reading.step;
        if left <= 0.0 {
            self.cool_down_left = Some(AUTO_ITEM_USE_COOL_DOWN_WHILE_HEALTH_IS_HIGH);
            return true;
        }
        self.cool_down_left = Some(left);
        false
    }
}

/// The hero's own game uses no item on it while it is down and the downed rules are on.
pub fn own_item_use_is_held(downed_revive_on: bool, local: DownedPhase) -> bool {
    downed_revive_on && local == DownedPhase::Downed
}

/// Only an item whose use heals the hero is held; other items stay usable while down.
pub fn item_use_is_held(hold_on: bool, item_id: i32) -> bool {
    hold_on && ITEM_IDS_THAT_HEAL_THE_HERO.contains(&item_id)
}

/// Who called the use-item function, from the return address the call left on the stack.
pub fn item_use_asker(variant: ExeVariant, return_rva: u32) -> ItemUseAsker {
    let is = |address: GameAddress| address.rva(variant) == return_rva;
    if is(USE_ITEM_RETURN_IN_AUTO_ITEM_USE_CHIP_CHECK) {
        ItemUseAsker::AutoUseItemChip
    } else if is(USE_ITEM_RETURN_IN_QUICK_ITEM_MENU) {
        ItemUseAsker::QuickItemMenu
    } else if is(USE_ITEM_RETURN_IN_MENU_ITEM_LIST) {
        ItemUseAsker::MenuItemList
    } else if USE_ITEM_RETURNS_IN_MENU_ITEM_SCREENS.into_iter().any(is) {
        ItemUseAsker::MenuItemScreen
    } else if is(USE_ITEM_RETURN_IN_SCRIPT_ITEM_COMMAND) {
        ItemUseAsker::ScriptCommand
    } else {
        ItemUseAsker::Unknown
    }
}
