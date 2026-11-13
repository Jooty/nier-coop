use crate::load_gate::StoryPlace;
use crate::story_follow::guest_side::GuestSide;
use crate::story_follow::host_side::HostSide;
use crate::story_follow::{
    FollowStep, GameRequestVerdict, RequestClass, StoryAsk, StoryAskAnswer, StoryMove, StoryRequest,
};
use crate::travel::JumpRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowReadings {
    pub now_ms: u64,
    pub role: JumpRole,
    pub partner_present: bool,
    pub can_take_a_request: bool,
    pub current_place: StoryPlace,
    pub host_place: StoryPlace,
    pub host_report_heard_at_ms: u64,
    pub host_in_play: bool,
    pub host_waiting_at_load_end: bool,
    pub moves: Vec<StoryMove>,
    pub asks: Vec<StoryAsk>,
    pub answers: Vec<StoryAskAnswer>,
}

#[derive(Debug, Clone, Default)]
pub struct FollowMachine {
    role: JumpRole,
    host: HostSide,
    guest: GuestSide,
    logs_before_the_next_step: Vec<String>,
}

impl FollowMachine {
    pub fn role(&self) -> JumpRole {
        self.role
    }

    pub fn on_game_request(
        &mut self,
        request: &StoryRequest,
        current: &StoryPlace,
        now_ms: u64,
    ) -> GameRequestVerdict {
        if !RequestClass::of(request, current).is_for_the_story() {
            return GameRequestVerdict::Pass;
        }
        match self.role {
            JumpRole::Solo => GameRequestVerdict::Pass,
            JumpRole::Host => {
                self.host
                    .game_request(request, current, &mut self.logs_before_the_next_step);
                GameRequestVerdict::Pass
            }
            JumpRole::Guest => self.guest.game_request(
                request,
                current,
                now_ms,
                &mut self.logs_before_the_next_step,
            ),
        }
    }

    pub fn step(&mut self, readings: &FollowReadings) -> FollowStep {
        let role = if readings.partner_present {
            readings.role
        } else {
            JumpRole::Solo
        };
        let mut step = FollowStep {
            logs: std::mem::take(&mut self.logs_before_the_next_step),
            ..FollowStep::default()
        };
        if role != self.role {
            self.change_role(role, readings.now_ms, &mut step);
        }
        match role {
            JumpRole::Solo => self.guest.step_alone(readings, &mut step),
            JumpRole::Host => self.host.step(
                readings.now_ms,
                readings.can_take_a_request,
                &readings.current_place,
                &readings.asks,
                &mut step,
            ),
            JumpRole::Guest => self.guest.step(readings, &mut step),
        }
        step
    }

    fn change_role(&mut self, role: JumpRole, now_ms: u64, step: &mut FollowStep) {
        if role == JumpRole::Solo && self.role == JumpRole::Guest {
            self.guest.partner_gone(now_ms, step);
        }
        if self.role == JumpRole::Host {
            self.host = HostSide::default();
        }
        self.role = role;
    }
}
