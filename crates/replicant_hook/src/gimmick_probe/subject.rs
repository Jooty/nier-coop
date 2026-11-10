use replicant_game::cube_probe::{ActorLink, GrabWatch};
use replicant_game::offsets::BODY_MOTION_LAYER;
use replicant_game::puppet::{motion_name, read_layer_motion};

use crate::memory::InProcessMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeldChoice {
    Link,
    NearestGuess,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HeldCube {
    pub actor: u64,
    pub choice: HeldChoice,
}

pub(super) struct Subject {
    pub label: &'static str,
    pub actor: u64,
    pub watch: GrabWatch,
    pub held: Option<HeldCube>,
    pub last_links: Vec<ActorLink>,
    cached_name: Option<(i32, Option<String>)>,
}

impl HeldChoice {
    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Link => "from the grab link",
            Self::NearestGuess => "nearest actor, no link found",
        }
    }
}

impl Subject {
    pub(super) fn new(label: &'static str) -> Self {
        Self {
            label,
            actor: 0,
            watch: GrabWatch::default(),
            held: None,
            last_links: Vec::new(),
            cached_name: None,
        }
    }

    pub(super) fn follow_actor(&mut self, actor: u64) {
        if self.actor != actor {
            *self = Self::new(self.label);
            self.actor = actor;
        }
    }

    pub(super) fn body_motion_name(&mut self, memory: &InProcessMemory) -> Option<String> {
        let motion = read_layer_motion(memory, self.actor, BODY_MOTION_LAYER)
            .ok()
            .flatten()?;
        if let Some((index, name)) = &self.cached_name
            && *index == motion.index
        {
            return name.clone();
        }
        let name = motion_name(memory, self.actor, motion.index).ok().flatten();
        self.cached_name = Some((motion.index, name.clone()));
        name
    }
}
