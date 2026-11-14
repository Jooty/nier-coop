use crate::app::Launcher;
use crate::lobby::character_label_of;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Back,
    Forward,
}

/// Your current character's label and every character your stepper can land on, the current one first.
pub(super) struct OwnPick {
    pub label: &'static str,
    pub choices: Vec<&'static str>,
}

impl OwnPick {
    pub fn steppable(&self) -> bool {
        self.choices.len() > 1
    }

    pub fn step(&self, step: Step) -> Option<&'static str> {
        match step {
            Step::Forward => self.choices.get(1),
            Step::Back => self.choices.last().filter(|_| self.choices.len() > 1),
        }
        .copied()
    }
}

pub(super) fn character_pick(launcher: &Launcher, viewer_is_guest: bool) -> OwnPick {
    let own = launcher.own_character_id(viewer_is_guest);
    let mut choices: Vec<&'static str> = launcher
        .own_character_choices()
        .into_iter()
        .map(|choice| choice.id)
        .collect();
    // The current pick goes first, so a step lands on its neighbour at either end.
    if let Some(at) = choices.iter().position(|id| *id == own) {
        choices.rotate_left(at);
    }
    OwnPick {
        label: character_label_of(launcher.profile, own),
        choices,
    }
}

pub(super) fn apply_pick(launcher: &mut Launcher, id: &'static str) {
    launcher.set_character_id(id);
}
