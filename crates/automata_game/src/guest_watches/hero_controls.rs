use crate::partner::BodyFields;

pub const HERO_INPUT_SOURCE_ON: i32 = 1;
pub const HERO_INPUT_SOURCE_OFF: i32 = 0;

/// The three input sources of the hero's controller: the only fields watching writes on the hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroInputSources {
    pub pad: i32,
    pub mouse: i32,
    pub keyboard: i32,
}

impl HeroInputSources {
    pub const ALL_ON: Self = Self {
        pad: HERO_INPUT_SOURCE_ON,
        mouse: HERO_INPUT_SOURCE_ON,
        keyboard: HERO_INPUT_SOURCE_ON,
    };

    pub fn of(fields: BodyFields) -> Self {
        Self {
            pad: fields.pad_enable,
            mouse: fields.mouse_enable,
            keyboard: fields.keyboard_enable,
        }
    }

    pub fn any_on(self) -> bool {
        self.pad != HERO_INPUT_SOURCE_OFF
            || self.mouse != HERO_INPUT_SOURCE_OFF
            || self.keyboard != HERO_INPUT_SOURCE_OFF
    }

    /// Kept for the give-back only when the game had at least one source on; a hero read with
    /// all three off (mid-load, or already cut) is given all three back on.
    pub fn worth_keeping(self) -> Option<Self> {
        self.any_on().then_some(self)
    }

    fn written_on(self, current: BodyFields) -> BodyFields {
        BodyFields {
            pad_enable: self.pad,
            mouse_enable: self.mouse,
            keyboard_enable: self.keyboard,
            ..current
        }
    }
}

/// What the watch drive writes on the hero each tick: the three sources off, every other field
/// as the game has it. The pad source opens alone while the host flies on a fed stick, since
/// the flight movement reads the stick only through it.
pub fn hero_fields_while_watched(current: BodyFields, flight_stick_is_fed: bool) -> BodyFields {
    HeroInputSources {
        pad: if flight_stick_is_fed {
            HERO_INPUT_SOURCE_ON
        } else {
            HERO_INPUT_SOURCE_OFF
        },
        mouse: HERO_INPUT_SOURCE_OFF,
        keyboard: HERO_INPUT_SOURCE_OFF,
    }
    .written_on(current)
}

/// What is written on the hero when watching ends: the sources it had, or all on.
pub fn hero_fields_given_back(current: BodyFields, kept: Option<HeroInputSources>) -> BodyFields {
    kept.unwrap_or(HeroInputSources::ALL_ON).written_on(current)
}
