use crate::downed::{
    BLEED_OUT_SECONDS, DOWNED_HEALTH, DownedPhase, DownedSignal,
    HIGHEST_REVIVE_PERCENT_BEFORE_DONE, HeroGuard, LONGEST_STEP_SECONDS, MIN_REVIVED_HEALTH,
    REVIVE_HOLD_SECONDS, REVIVE_PERCENT_WHEN_DONE, REVIVE_PROGRESS_LOG_STEP_PERCENT,
    REVIVE_RANGE_UNITS, REVIVED_HEALTH_DIVISOR, REVIVED_SHIELD_SECONDS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GotUpCause {
    Revived,
    PartnerGone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallBackCause {
    BothDown,
    PartnerDefeated,
    GaveUp,
    DefeatedByTheGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownedEffect {
    WentDown,
    GotUp(GotUpCause),
    /// The game's own defeat takes over; the hook lets the hero be defeated.
    FellBack(FallBackCause),
    DefeatedWhileUp,
    BackFromTheGamesDefeat,
    BeingRevived(u8),
    PartnerWentDown,
    PartnerGotUp,
    PartnerFellBack,
    ReviveStarted,
    ReviveProgress(u8),
    ReviveBroken,
    ReviveDone,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepInput {
    pub dt_seconds: f32,
    /// The setting is on and everything the downed state needs in this game was found.
    pub enabled: bool,
    /// The other player's signal while that player is in play with a hero.
    pub partner: Option<DownedSignal>,
    /// None when the two heroes are not in one place.
    pub partner_distance: Option<f32>,
    pub local_health: i32,
    pub lethal_hit_seen: bool,
    pub defeated_by_the_game: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Revive {
    down: u8,
    seconds: f32,
    done: bool,
    logged_percent: u8,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DownedRules {
    local: DownedPhase,
    times_downed: u8,
    bleed_seconds: f32,
    incoming_percent: u8,
    incoming_logged_percent: u8,
    partner: Option<DownedPhase>,
    revive: Option<Revive>,
    give_up_asked: bool,
    revived_shield_seconds: f32,
}

impl Revive {
    fn percent(&self) -> u8 {
        if self.done {
            return REVIVE_PERCENT_WHEN_DONE;
        }
        let percent = self.seconds / REVIVE_HOLD_SECONDS * f32::from(REVIVE_PERCENT_WHEN_DONE);
        (percent as u8).min(HIGHEST_REVIVE_PERCENT_BEFORE_DONE)
    }

    fn is_under_way(&self) -> bool {
        self.seconds > 0.0 && !self.done
    }
}

impl DownedRules {
    pub fn local(&self) -> DownedPhase {
        self.local
    }

    /// The other player's phase while that player can take part; None otherwise.
    pub fn partner(&self) -> Option<DownedPhase> {
        self.partner
    }

    pub fn bleed_out_progress(&self) -> f32 {
        (self.bleed_seconds / BLEED_OUT_SECONDS).clamp(0.0, 1.0)
    }

    pub fn being_revived_progress(&self) -> f32 {
        if self.local == DownedPhase::Downed {
            f32::from(self.incoming_percent) / f32::from(REVIVE_PERCENT_WHEN_DONE)
        } else {
            0.0
        }
    }

    pub fn reviving_progress(&self) -> f32 {
        self.revive.map_or(0.0, |revive| {
            f32::from(revive.percent()) / f32::from(REVIVE_PERCENT_WHEN_DONE)
        })
    }

    /// The downed player asked for the game's own defeat instead of waiting.
    pub fn ask_to_give_up(&mut self) {
        self.give_up_asked = true;
    }

    pub fn signal(&self, enabled: bool) -> DownedSignal {
        DownedSignal {
            revives: enabled,
            phase: self.local,
            times_downed: self.times_downed,
            reviving_down: self.revive.map_or(0, |revive| revive.down),
            reviving_percent: self.revive.map_or(0, |revive| revive.percent()),
        }
    }

    /// The hero's defeat is held off only while the other player is up and can revive.
    pub fn hero_guard(&self) -> HeroGuard {
        match (self.local, self.partner) {
            (DownedPhase::Downed, _) => HeroGuard::Shield,
            (DownedPhase::Up, _) if self.revived_shield_seconds > 0.0 => HeroGuard::Shield,
            (DownedPhase::Up, Some(DownedPhase::Up)) => HeroGuard::HoldOffDefeat,
            _ => HeroGuard::Off,
        }
    }

    pub fn step(&mut self, input: StepInput) -> Vec<DownedEffect> {
        let mut effects = Vec::new();
        let dt_seconds = input.dt_seconds.clamp(0.0, LONGEST_STEP_SECONDS);
        let partner = input
            .partner
            .filter(|partner| input.enabled && partner.revives && partner.is_valid());
        self.note_the_partner(partner, &mut effects);
        self.revived_shield_seconds = (self.revived_shield_seconds - dt_seconds).max(0.0);
        self.step_local(input, dt_seconds, partner, &mut effects);
        self.step_revive(input, dt_seconds, partner, &mut effects);
        self.give_up_asked = false;
        effects
    }

    fn note_the_partner(&mut self, partner: Option<DownedSignal>, effects: &mut Vec<DownedEffect>) {
        let now = partner.map(|partner| partner.phase);
        match (self.partner, now) {
            (Some(DownedPhase::Up), Some(DownedPhase::Downed)) => {
                effects.push(DownedEffect::PartnerWentDown);
            }
            (Some(DownedPhase::Downed), Some(DownedPhase::Up)) => {
                effects.push(DownedEffect::PartnerGotUp);
            }
            (Some(DownedPhase::Downed), Some(DownedPhase::Defeated)) => {
                effects.push(DownedEffect::PartnerFellBack);
            }
            _ => {}
        }
        self.partner = now;
    }

    fn step_local(
        &mut self,
        input: StepInput,
        dt_seconds: f32,
        partner: Option<DownedSignal>,
        effects: &mut Vec<DownedEffect>,
    ) {
        match self.local {
            DownedPhase::Defeated => {
                if input.local_health > DOWNED_HEALTH && !input.defeated_by_the_game {
                    self.local = DownedPhase::Up;
                    if partner.is_some() {
                        effects.push(DownedEffect::BackFromTheGamesDefeat);
                    }
                }
            }
            DownedPhase::Up => {
                if input.defeated_by_the_game {
                    self.local = DownedPhase::Defeated;
                    if partner.is_some() {
                        effects.push(DownedEffect::DefeatedWhileUp);
                    }
                } else if partner.is_some_and(|partner| partner.phase == DownedPhase::Up)
                    && input.lethal_hit_seen
                    && input.local_health <= DOWNED_HEALTH
                {
                    self.go_down(effects);
                }
            }
            DownedPhase::Downed => self.step_downed(input, dt_seconds, partner, effects),
        }
    }

    fn step_downed(
        &mut self,
        input: StepInput,
        dt_seconds: f32,
        partner: Option<DownedSignal>,
        effects: &mut Vec<DownedEffect>,
    ) {
        if input.defeated_by_the_game {
            return self.fall_back(FallBackCause::DefeatedByTheGame, effects);
        }
        let Some(partner) = partner else {
            return self.get_up(GotUpCause::PartnerGone, effects);
        };
        if self.give_up_asked {
            return self.fall_back(FallBackCause::GaveUp, effects);
        }
        match partner.phase {
            DownedPhase::Downed => return self.fall_back(FallBackCause::BothDown, effects),
            DownedPhase::Defeated => {
                return self.fall_back(FallBackCause::PartnerDefeated, effects);
            }
            DownedPhase::Up => {}
        }
        let for_this_down = partner.reviving_down == self.times_downed;
        if for_this_down && partner.reviving_percent >= REVIVE_PERCENT_WHEN_DONE {
            self.revived_shield_seconds = REVIVED_SHIELD_SECONDS;
            return self.get_up(GotUpCause::Revived, effects);
        }
        self.bleed_seconds = (self.bleed_seconds + dt_seconds).min(BLEED_OUT_SECONDS);
        self.incoming_percent = if for_this_down {
            partner.reviving_percent
        } else {
            0
        };
        let step = log_step_of(self.incoming_percent);
        if step > self.incoming_logged_percent {
            effects.push(DownedEffect::BeingRevived(step));
        }
        self.incoming_logged_percent = step;
    }

    fn step_revive(
        &mut self,
        input: StepInput,
        dt_seconds: f32,
        partner: Option<DownedSignal>,
        effects: &mut Vec<DownedEffect>,
    ) {
        let downed_partner = partner
            .filter(|partner| partner.phase == DownedPhase::Downed)
            .filter(|_| self.local == DownedPhase::Up);
        let Some(partner) = downed_partner else {
            if self.revive.is_some_and(|revive| revive.is_under_way()) {
                effects.push(DownedEffect::ReviveBroken);
            }
            self.revive = None;
            return;
        };
        let mut revive = self
            .revive
            .filter(|revive| revive.down == partner.times_downed)
            .unwrap_or(Revive {
                down: partner.times_downed,
                seconds: 0.0,
                done: false,
                logged_percent: 0,
            });
        let close = input
            .partner_distance
            .is_some_and(|distance| distance <= REVIVE_RANGE_UNITS);
        if !revive.done && close {
            if revive.seconds == 0.0 {
                effects.push(DownedEffect::ReviveStarted);
            }
            revive.seconds += dt_seconds;
            if revive.seconds >= REVIVE_HOLD_SECONDS {
                revive.done = true;
                effects.push(DownedEffect::ReviveDone);
            } else {
                let step = log_step_of(revive.percent());
                if step > revive.logged_percent {
                    revive.logged_percent = step;
                    effects.push(DownedEffect::ReviveProgress(step));
                }
            }
        } else if !revive.done {
            if revive.is_under_way() {
                effects.push(DownedEffect::ReviveBroken);
            }
            revive.seconds = 0.0;
            revive.logged_percent = 0;
        }
        self.revive = Some(revive);
    }

    fn go_down(&mut self, effects: &mut Vec<DownedEffect>) {
        self.local = DownedPhase::Downed;
        self.times_downed = self.times_downed.wrapping_add(1);
        self.forget_the_down();
        effects.push(DownedEffect::WentDown);
    }

    fn get_up(&mut self, cause: GotUpCause, effects: &mut Vec<DownedEffect>) {
        self.local = DownedPhase::Up;
        self.forget_the_down();
        effects.push(DownedEffect::GotUp(cause));
    }

    fn fall_back(&mut self, cause: FallBackCause, effects: &mut Vec<DownedEffect>) {
        self.local = DownedPhase::Defeated;
        self.forget_the_down();
        effects.push(DownedEffect::FellBack(cause));
    }

    fn forget_the_down(&mut self) {
        self.bleed_seconds = 0.0;
        self.incoming_percent = 0;
        self.incoming_logged_percent = 0;
    }
}

/// A hero that is up is defeated once the game starts its defeat state or flow; a downed one
/// also when its health was taken below the hold by a way the hold does not cover.
pub fn hero_is_defeated_by_the_game(
    local: DownedPhase,
    health: i32,
    in_a_defeat_state: bool,
    defeat_flow_runs: bool,
) -> bool {
    in_a_defeat_state
        || defeat_flow_runs
        || (local == DownedPhase::Downed && health < DOWNED_HEALTH)
}

pub fn revived_health(max_health: i32) -> i32 {
    (max_health / REVIVED_HEALTH_DIVISOR).max(MIN_REVIVED_HEALTH)
}

/// The health a hero gets up with: the revived share, or what it has when that is more.
pub fn health_after_getting_up(current_health: i32, max_health: i32) -> i32 {
    revived_health(max_health)
        .min(max_health)
        .max(current_health)
        .max(DOWNED_HEALTH)
}

/// None when a coordinate is not a number.
pub fn distance_between(a: [f32; 3], b: [f32; 3]) -> Option<f32> {
    let squared: f32 = a.iter().zip(b).map(|(a, b)| (a - b) * (a - b)).sum();
    squared.is_finite().then(|| squared.sqrt())
}

fn log_step_of(percent: u8) -> u8 {
    percent / REVIVE_PROGRESS_LOG_STEP_PERCENT * REVIVE_PROGRESS_LOG_STEP_PERCENT
}
