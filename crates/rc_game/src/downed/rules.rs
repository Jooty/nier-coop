use rc_proto::CombatEvent;

pub const REVIVE_RANGE_UNITS: f32 = 3.0;
pub const REVIVE_HOLD_SECONDS: f32 = 3.0;
pub const DOWNED_HEALTH: i32 = 1;
pub const HEARTBEAT_SECONDS: f32 = 1.0;
pub const BLEED_OUT_SECONDS: f32 = 60.0;
pub const REVIVED_HEALTH_DIVISOR: i32 = 4;
pub const MIN_REVIVED_HEALTH: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownedState {
    Up,
    Downed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownedEffect {
    Send(CombatEvent),
    EnterDowned,
    LeaveDowned,
    ForceKill,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepInput {
    pub dt_seconds: f32,
    pub session_active: bool,
    pub local_health: i32,
    pub lethal_hit_seen: bool,
    pub partner_distance: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownedRules {
    local: DownedState,
    partner: DownedState,
    revive_seconds: f32,
    heartbeat_seconds: f32,
    bleed_seconds: f32,
    last_health: Option<i32>,
    game_over: bool,
}

impl Default for DownedRules {
    fn default() -> Self {
        Self::new()
    }
}

impl DownedRules {
    pub const fn new() -> Self {
        Self {
            local: DownedState::Up,
            partner: DownedState::Up,
            revive_seconds: 0.0,
            heartbeat_seconds: 0.0,
            bleed_seconds: 0.0,
            last_health: None,
            game_over: false,
        }
    }

    pub fn local(&self) -> DownedState {
        self.local
    }

    pub fn partner(&self) -> DownedState {
        self.partner
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn revive_progress(&self) -> f32 {
        (self.revive_seconds / REVIVE_HOLD_SECONDS).clamp(0.0, 1.0)
    }

    pub fn step(&mut self, input: StepInput) -> Vec<DownedEffect> {
        if !input.session_active {
            return self.end_session();
        }
        if self.game_over {
            if input.local_health <= DOWNED_HEALTH {
                return Vec::new();
            }
            *self = Self::new();
        }
        let mut effects = Vec::new();
        if self.local == DownedState::Up && self.took_a_lethal_hit(input) {
            self.go_down(&mut effects);
        }
        self.last_health = Some(input.local_health);
        if self.local == DownedState::Up && self.partner == DownedState::Downed {
            self.hold_revive(input, &mut effects);
        } else {
            self.revive_seconds = 0.0;
        }
        self.end_the_fight_if_both_down(&mut effects);
        self.beat_and_bleed(input.dt_seconds, &mut effects);
        effects
    }

    pub fn bleed_out_progress(&self) -> f32 {
        (self.bleed_seconds / BLEED_OUT_SECONDS).clamp(0.0, 1.0)
    }

    pub fn on_partner_event(&mut self, event: CombatEvent) -> Vec<DownedEffect> {
        let mut effects = Vec::new();
        match event {
            CombatEvent::Downed => {
                if self.partner != DownedState::Downed {
                    self.partner = DownedState::Downed;
                    self.revive_seconds = 0.0;
                }
                self.end_the_fight_if_both_down(&mut effects);
            }
            CombatEvent::Up => {
                if self.partner == DownedState::Downed {
                    self.partner = DownedState::Up;
                    self.revive_seconds = 0.0;
                }
            }
            CombatEvent::Revived => {
                self.partner = DownedState::Up;
                if self.local == DownedState::Downed && !self.game_over {
                    self.local = DownedState::Up;
                    self.bleed_seconds = 0.0;
                    self.last_health = None;
                    effects.push(DownedEffect::LeaveDowned);
                }
            }
            CombatEvent::GameOver => {
                if !self.game_over {
                    self.game_over = true;
                    effects.push(DownedEffect::ForceKill);
                }
            }
        }
        effects
    }

    pub fn request_go_down(&mut self) -> Vec<DownedEffect> {
        let mut effects = Vec::new();
        if self.local == DownedState::Up && !self.game_over {
            self.go_down(&mut effects);
            self.end_the_fight_if_both_down(&mut effects);
        }
        effects
    }

    pub fn request_revive_partner(&mut self) -> Vec<DownedEffect> {
        let mut effects = Vec::new();
        if self.local == DownedState::Up && self.partner == DownedState::Downed {
            self.finish_revive(&mut effects);
        }
        effects
    }

    fn took_a_lethal_hit(&self, input: StepInput) -> bool {
        input.lethal_hit_seen
            && input.local_health == DOWNED_HEALTH
            && self.last_health.is_some_and(|last| last > DOWNED_HEALTH)
    }

    fn beat_and_bleed(&mut self, dt_seconds: f32, effects: &mut Vec<DownedEffect>) {
        if self.game_over {
            return;
        }
        let dt = dt_seconds.max(0.0);
        self.heartbeat_seconds += dt;
        if self.heartbeat_seconds >= HEARTBEAT_SECONDS {
            self.heartbeat_seconds = 0.0;
            effects.push(DownedEffect::Send(if self.local == DownedState::Downed {
                CombatEvent::Downed
            } else {
                CombatEvent::Up
            }));
        }
        if self.local == DownedState::Downed {
            self.bleed_seconds += dt;
            if self.bleed_seconds >= BLEED_OUT_SECONDS {
                self.game_over = true;
                effects.push(DownedEffect::ForceKill);
            }
        }
    }

    fn go_down(&mut self, effects: &mut Vec<DownedEffect>) {
        self.local = DownedState::Downed;
        self.revive_seconds = 0.0;
        self.bleed_seconds = 0.0;
        self.heartbeat_seconds = 0.0;
        effects.push(DownedEffect::EnterDowned);
        effects.push(DownedEffect::Send(CombatEvent::Downed));
    }

    fn hold_revive(&mut self, input: StepInput, effects: &mut Vec<DownedEffect>) {
        let close = input
            .partner_distance
            .is_some_and(|distance| distance <= REVIVE_RANGE_UNITS);
        if !close {
            self.revive_seconds = 0.0;
            return;
        }
        self.revive_seconds += input.dt_seconds.max(0.0);
        if self.revive_seconds >= REVIVE_HOLD_SECONDS {
            self.finish_revive(effects);
        }
    }

    fn finish_revive(&mut self, effects: &mut Vec<DownedEffect>) {
        self.partner = DownedState::Up;
        self.revive_seconds = 0.0;
        effects.push(DownedEffect::Send(CombatEvent::Revived));
    }

    fn end_the_fight_if_both_down(&mut self, effects: &mut Vec<DownedEffect>) {
        let both_down = self.local == DownedState::Downed && self.partner == DownedState::Downed;
        if both_down && !self.game_over {
            self.game_over = true;
            effects.push(DownedEffect::Send(CombatEvent::GameOver));
            effects.push(DownedEffect::ForceKill);
        }
    }

    fn end_session(&mut self) -> Vec<DownedEffect> {
        let leave = self.local == DownedState::Downed && !self.game_over;
        *self = Self::new();
        if leave {
            vec![DownedEffect::LeaveDowned]
        } else {
            Vec::new()
        }
    }
}

pub fn revived_health(max_health: i32) -> i32 {
    (max_health / REVIVED_HEALTH_DIVISOR).max(MIN_REVIVED_HEALTH)
}
