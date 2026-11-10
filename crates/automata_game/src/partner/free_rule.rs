use std::fmt;

use crate::partner::{EVENT_INPUT_OFF_BIT, SCENE_INPUT_OFF_BIT};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroGates {
    pub player_controlled: bool,
    pub computer_controlled: bool,
    pub input_blank_switch: i32,
    pub input_blank_seconds: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalGates {
    pub scene_input_off_word: u32,
    pub event_input_off_word: u32,
    pub controller_stopping_mode: i32,
}

/// What the game itself reads before it blanks a body's input for a cutscene or an event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeReadings {
    pub in_play: bool,
    pub hero: Option<HeroGates>,
    pub globals: Option<GlobalGates>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotFree {
    NotInPlay,
    NoHero,
    GatesUnreadable,
    HeroNotPlayerControlled,
    HeroComputerControlled,
    HeroInputBlankSwitch(i32),
    HeroInputBlankTimer,
    SceneInputOffBit,
    EventInputOffBit,
    ControllerStoppingMode(i32),
}

/// True from the first free moment of a stay in play until play ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReadyLatch {
    ready: bool,
}

impl FreeReadings {
    pub fn is_free(&self) -> bool {
        not_free_reason(self).is_none()
    }
}

impl fmt::Display for NotFree {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInPlay => write!(formatter, "the scene is not in play"),
            Self::NoHero => write!(formatter, "there is no hero"),
            Self::GatesUnreadable => write!(formatter, "the game's input gates cannot be read"),
            Self::HeroNotPlayerControlled => {
                write!(formatter, "the hero's player flag is 0")
            }
            Self::HeroComputerControlled => {
                write!(formatter, "the hero is computer controlled")
            }
            Self::HeroInputBlankSwitch(value) => {
                write!(formatter, "the hero's input blank switch reads {value}")
            }
            Self::HeroInputBlankTimer => {
                write!(formatter, "the hero's input blank timer runs")
            }
            Self::SceneInputOffBit => {
                write!(
                    formatter,
                    "the scene input-off bit {SCENE_INPUT_OFF_BIT:#X} is set"
                )
            }
            Self::EventInputOffBit => {
                write!(
                    formatter,
                    "the event input-off bit {EVENT_INPUT_OFF_BIT:#X} is set"
                )
            }
            Self::ControllerStoppingMode(mode) => {
                write!(formatter, "the game's mode word reads {mode}")
            }
        }
    }
}

impl ReadyLatch {
    pub fn observe(&mut self, in_play: bool, free: bool) -> bool {
        self.ready = in_play && (self.ready || free);
        self.ready
    }
}

/// Why the local player is not free; `None` when the scene is in play and nothing holds the hero.
pub fn not_free_reason(readings: &FreeReadings) -> Option<NotFree> {
    if !readings.in_play {
        return Some(NotFree::NotInPlay);
    }
    let Some(hero) = readings.hero else {
        return Some(NotFree::NoHero);
    };
    let Some(globals) = readings.globals else {
        return Some(NotFree::GatesUnreadable);
    };
    if !hero.player_controlled {
        return Some(NotFree::HeroNotPlayerControlled);
    }
    if hero.computer_controlled {
        return Some(NotFree::HeroComputerControlled);
    }
    if globals.scene_input_off_word & SCENE_INPUT_OFF_BIT != 0 {
        return Some(NotFree::SceneInputOffBit);
    }
    if globals.event_input_off_word & EVENT_INPUT_OFF_BIT != 0 {
        return Some(NotFree::EventInputOffBit);
    }
    if globals.controller_stopping_mode != 0 {
        return Some(NotFree::ControllerStoppingMode(
            globals.controller_stopping_mode,
        ));
    }
    if hero.input_blank_switch != 0 {
        return Some(NotFree::HeroInputBlankSwitch(hero.input_blank_switch));
    }
    if hero.input_blank_seconds > 0.0 {
        return Some(NotFree::HeroInputBlankTimer);
    }
    None
}
