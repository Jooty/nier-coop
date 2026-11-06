use rc_proto::GameReadiness;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocalGameReadiness {
    linked: bool,
    reported: GameReadiness,
}

impl LocalGameReadiness {
    pub const fn new() -> Self {
        Self {
            linked: false,
            reported: GameReadiness::NotLinked,
        }
    }

    pub fn game_linked(&mut self) {
        self.linked = true;
        self.reported = GameReadiness::Loading;
    }

    pub fn game_unlinked(&mut self) {
        self.linked = false;
        self.reported = GameReadiness::NotLinked;
    }

    pub fn game_reported(&mut self, readiness: GameReadiness) {
        if self.linked {
            self.reported = match readiness {
                GameReadiness::NotLinked => GameReadiness::Loading,
                other => other,
            };
        }
    }

    pub fn current(self) -> GameReadiness {
        self.reported
    }
}
