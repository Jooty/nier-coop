use crate::partner::{BODY_PUT_BACK_RETRY_MS, BODY_PUT_BACK_TRIES_PER_FALL, SlotChange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotFall {
    pub change: SlotChange,
    pub given_back: bool,
    pub body_resolves: bool,
    pub body_state: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyMade {
    Spawned,
    TakenOver,
}

/// Why the body this copy held before is not the one it holds next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldBodyFate {
    NoneKnown,
    Kept { handle: u32 },
    Gone { handle: u32 },
    OtherCharacter { handle: u32, object_id: u32 },
    OtherHero { handle: u32 },
    IsTheHeroNow { handle: u32 },
    OutOfTries { handle: u32 },
    SlotHeldAnother { handle: u32 },
}

impl OldBodyFate {
    pub fn describe(&self) -> String {
        match *self {
            Self::NoneKnown => {
                "no old one is known (none was held, or it was removed or let go for good)"
                    .to_owned()
            }
            Self::Kept { handle } => {
                format!("the old one (handle {handle:#X}) is the body in the slot, kept")
            }
            Self::Gone { handle } => {
                format!("the old one (handle {handle:#X}) no longer resolves")
            }
            Self::OtherCharacter { handle, object_id } => format!(
                "the old one (handle {handle:#X}) is object {object_id:#X}, not the wanted character"
            ),
            Self::OtherHero { handle } => format!(
                "the old one (handle {handle:#X}) was the partner of another hero (a load or a hero change)"
            ),
            Self::IsTheHeroNow { handle } => format!(
                "the old one (handle {handle:#X}) is the hero now (the game switched to it)"
            ),
            Self::OutOfTries { handle } => format!(
                "the old one (handle {handle:#X}) did not stay in the slot after {BODY_PUT_BACK_TRIES_PER_FALL} put-backs"
            ),
            Self::SlotHeldAnother { handle } => format!(
                "the game put another body into the slot, the old one (handle {handle:#X}) is left where it is"
            ),
        }
    }
}

pub fn slot_fall_line(fall: &SlotFall) -> String {
    let change = &fall.change;
    let state = fall
        .body_state
        .map_or("unreadable".to_owned(), |state| format!("{state:#X}"));
    let since = change
        .since_place_change_ms
        .map_or("no story-place change seen yet".to_owned(), |since_ms| {
            format!("{since_ms} ms since the story place last changed")
        });
    format!(
        "partner: slot fall: the hero's partner slot went from {:#X} to {:#X}; the {} body {:#X} resolves {}, state {state}; story place {} -> {}, {since}",
        change.slot_before,
        change.slot_after,
        if fall.given_back {
            "given-back"
        } else {
            "held"
        },
        change.body_handle,
        fall.body_resolves,
        change.place_before.describe(),
        change.place_now.describe(),
    )
}

pub fn body_put_back_line(handle: u32, number: u8) -> String {
    format!(
        "partner: the body (handle {handle:#X}) fell out of the hero's partner slot and is still alive, put it back (try {number} of {BODY_PUT_BACK_TRIES_PER_FALL}, {BODY_PUT_BACK_RETRY_MS} ms apart)"
    )
}

pub fn body_put_back_gave_up_line(handle: u32) -> String {
    format!(
        "partner: the body (handle {handle:#X}) did not stay in the hero's partner slot after {BODY_PUT_BACK_TRIES_PER_FALL} put-backs {BODY_PUT_BACK_RETRY_MS} ms apart, giving it up and starting over"
    )
}

pub fn slot_refilled_line(ours: u32, slot_holder: u32, slot_object_id: Option<u32>) -> String {
    let holder = slot_object_id.map_or_else(
        || format!("handle {slot_holder:#X}, not a player body"),
        |object_id| format!("handle {slot_holder:#X}, object {object_id:#X}"),
    );
    format!(
        "partner: the game put another body into the hero's partner slot ({holder}) in place of ours (handle {ours:#X}); not fighting it, letting ours go"
    )
}

pub fn given_back_body_put_back_line(handle: u32, number: u8) -> String {
    format!(
        "partner: put back the body given back to the game (handle {handle:#X}), which left the hero's partner slot while given back (try {number} of {BODY_PUT_BACK_TRIES_PER_FALL}, {BODY_PUT_BACK_RETRY_MS} ms apart)"
    )
}

pub fn given_back_body_not_put_back_line(fate: OldBodyFate) -> String {
    format!(
        "partner: the body given back to the game is not put back: {}",
        fate.describe()
    )
}

pub fn slot_took_the_body_line(handle: u32, took: bool) -> String {
    if took {
        format!("partner: the hero's partner slot holds the body (handle {handle:#X}) again")
    } else {
        format!(
            "partner: the hero's partner slot did not take the body (handle {handle:#X}) this time"
        )
    }
}

pub fn old_body_line(made: BodyMade, fate: OldBodyFate) -> String {
    format!(
        "partner: {}: {}",
        match made {
            BodyMade::Spawned => "making a new body",
            BodyMade::TakenOver => "taking over the body in the slot",
        },
        fate.describe()
    )
}
