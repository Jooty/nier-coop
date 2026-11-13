use nc_launcher_core::GameProfile;
use replicant_proto::PartnerBody;

use crate::app::Launcher;
use crate::app::pages::seat_ledger::Step;
use crate::lobby::{body_label, picker_choices};

/// Your current body's label and every body your stepper can land on, the current one first.
pub(super) struct OwnPick {
    pub label: &'static str,
    pub choices: Vec<PartnerBody>,
}

impl OwnPick {
    pub fn steppable(&self) -> bool {
        self.choices.len() > 1
    }

    pub fn step(&self, step: Step) -> Option<PartnerBody> {
        match step {
            Step::Forward => self.choices.get(1),
            Step::Back => self.choices.last().filter(|_| self.choices.len() > 1),
        }
        .copied()
    }
}

pub(super) fn body_pick(profile: &GameProfile, body: PartnerBody) -> OwnPick {
    let mut choices = picker_choices(profile, &[PartnerBody::Nier]);
    // The current pick goes first, so a step lands on its neighbour at either end.
    if let Some(at) = choices.iter().position(|each| *each == body) {
        choices.rotate_left(at);
    }
    OwnPick {
        label: body_label(body),
        choices,
    }
}

pub(super) fn apply_pick(launcher: &mut Launcher, body: PartnerBody) {
    launcher.set_body(body);
}
