use nc_launcher_core::GameProfile;
use replicant_proto::PartnerBody;

use crate::app::Launcher;
use crate::app::pages::seat_ledger::Step;
use crate::lobby::{body_label, character_label_of, picker_choices};

/// One choice your character or body stepper can land on.
pub(super) enum Pick {
    Character(&'static str),
    Body(PartnerBody),
}

/// Your current pick's label and every choice, the current one first.
pub(super) struct OwnPick {
    pub label: &'static str,
    pub choices: Vec<Pick>,
}

impl OwnPick {
    pub fn steppable(&self) -> bool {
        self.choices.len() > 1
    }

    pub fn step(&self, step: Step) -> Option<Pick> {
        let next = match step {
            Step::Forward => self.choices.get(1),
            Step::Back => self.choices.last().filter(|_| self.choices.len() > 1),
        }?;
        Some(match next {
            Pick::Character(id) => Pick::Character(id),
            Pick::Body(body) => Pick::Body(*body),
        })
    }
}

pub(super) fn character_pick(launcher: &Launcher, viewer_is_guest: bool) -> OwnPick {
    let own = launcher.own_character_id(viewer_is_guest);
    let choices = launcher
        .own_character_choices(viewer_is_guest)
        .into_iter()
        .map(|choice| Pick::Character(choice.id))
        .collect();
    OwnPick {
        label: character_label_of(launcher.profile, own),
        choices: current_first(
            choices,
            |pick| matches!(pick, Pick::Character(id) if *id == own),
        ),
    }
}

pub(super) fn body_pick(profile: &GameProfile, body: PartnerBody) -> OwnPick {
    let choices = picker_choices(profile, &[PartnerBody::Nier])
        .into_iter()
        .map(Pick::Body)
        .collect();
    OwnPick {
        label: body_label(body),
        choices: current_first(
            choices,
            |pick| matches!(pick, Pick::Body(each) if *each == body),
        ),
    }
}

pub(super) fn apply_pick(launcher: &mut Launcher, pick: Pick) {
    match pick {
        Pick::Character(id) => launcher.set_character_id(id),
        Pick::Body(body) => launcher.set_body(body),
    }
}

fn current_first(mut choices: Vec<Pick>, is_current: impl Fn(&Pick) -> bool) -> Vec<Pick> {
    // The current pick goes first, so a step lands on its neighbour at either end.
    if let Some(at) = choices.iter().position(is_current) {
        choices.rotate_left(at);
    }
    choices
}
