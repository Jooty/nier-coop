use crate::downed::{DOWNED_HEALTH, OWN_DEFEAT_FLOOR_MODE_THAT_FLOORS};

/// What the damage function's hook does with hits on the local hero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeroGuard {
    #[default]
    Off,
    /// A hit that would defeat the hero leaves it at 1 health instead.
    HoldOffDefeat,
    /// The hero is down: no hit reaches its health.
    Shield,
}

/// What the hook reads before one hit on the local hero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeroHit {
    pub caller_asks_for_no_defeat: bool,
    pub berserk_switch: i32,
    pub own_floor_switch: i32,
    pub own_floor_mode: i32,
    pub flight_flags: [i32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroHitTreatment {
    Untouched,
    KeptOff,
    Floored,
}

impl HeroGuard {
    pub const fn as_number(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::HoldOffDefeat => 1,
            Self::Shield => 2,
        }
    }

    pub const fn from_number(number: u8) -> Self {
        match number {
            1 => Self::HoldOffDefeat,
            2 => Self::Shield,
            _ => Self::Off,
        }
    }
}

impl HeroHit {
    pub fn game_floors_it_itself(&self) -> bool {
        self.caller_asks_for_no_defeat
            || self.berserk_switch != 0
            || self.own_floor_switch != 0
            || self.own_floor_mode == OWN_DEFEAT_FLOOR_MODE_THAT_FLOORS
    }

    pub fn hero_flies(&self) -> bool {
        self.flight_flags.iter().any(|flag| *flag != 0)
    }
}

/// A flying hero keeps the game's own defeat: the downed pose is a state of a hero on foot.
pub fn hero_hit_treatment(guard: HeroGuard, hit: HeroHit) -> HeroHitTreatment {
    match guard {
        HeroGuard::Off => HeroHitTreatment::Untouched,
        HeroGuard::Shield => HeroHitTreatment::KeptOff,
        HeroGuard::HoldOffDefeat if hit.hero_flies() => HeroHitTreatment::Untouched,
        HeroGuard::HoldOffDefeat => HeroHitTreatment::Floored,
    }
}

/// True when a floored hit left the hero where only the mod's floor kept it from a defeat.
/// A hit that honestly leaves 1 health counts too: the two cannot be told apart.
pub fn hit_put_the_hero_down(hit: HeroHit, health_before: i32, health_after: i32) -> bool {
    !hit.game_floors_it_itself() && health_before > 0 && health_after <= DOWNED_HEALTH
}
