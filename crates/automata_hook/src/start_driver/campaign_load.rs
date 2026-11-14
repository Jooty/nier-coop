use automata_game::address::{
    CURRENT_PHASE_ID_VALUE, CURRENT_SUB_PHASE_NAME_VALUE, SCENE_STATE_VALUE,
};
use automata_game::load_gate::{
    LoadGate, LoadWatchReadings, LocalLoadSignal, StoryPlace, campaign_load_began_line,
    campaign_load_ended_line, campaign_load_link_lost_line, campaign_load_not_held_line,
};
use automata_game::partner::PartnerDataWait;
use automata_game::start::{SUB_PHASE_NAME_BUFFER_LEN, SubPhaseReading};
use nt_hook::diagnostics::log_line;
use nt_hook::net::net_clock_ms;

use crate::game_hooks::{reset_load_end_hold, set_load_end_hold_wanted};
use crate::net::{local_story_place, set_local_load_signal, set_local_story_place};
use crate::start_driver::driver_state::Driver;
use crate::start_driver::link_reading::link_reading;

impl Driver {
    pub(super) fn publish_story_place(&self) {
        let phase_id = self.calls.read_i32(CURRENT_PHASE_ID_VALUE).ok();
        let sub_phase = self
            .calls
            .read_bytes(CURRENT_SUB_PHASE_NAME_VALUE, SUB_PHASE_NAME_BUFFER_LEN)
            .map_or(SubPhaseReading::NotAName, |bytes| {
                SubPhaseReading::of_bytes(&bytes)
            });
        let place = match (phase_id, sub_phase) {
            (Some(phase_id), SubPhaseReading::Name(name)) => StoryPlace::new(phase_id, &name),
            (Some(phase_id), SubPhaseReading::NotAName) => StoryPlace::new(phase_id, ""),
            (None, _) => StoryPlace::default(),
        };
        set_local_story_place(place);
    }

    pub(super) fn drop_a_campaign_load(&mut self) {
        if self.load_watch.is_loading() {
            self.gate = LoadGate::unused();
        }
        self.load_watch.reset();
    }

    pub(super) fn step_campaign_load(&mut self) {
        if !self.campaign_loads_hooked {
            return;
        }
        let now_ms = self.started.elapsed().as_millis() as u64;
        let link = link_reading();
        let step = self.load_watch.step(&LoadWatchReadings {
            scene_state: self.calls.read_i32(SCENE_STATE_VALUE).ok(),
            phase_id: self.calls.read_i32(CURRENT_PHASE_ID_VALUE).ok(),
            campaign_link_up: link.campaign_link_up,
        });
        if step.began {
            self.begin_a_campaign_load();
        }
        if step.link_lost {
            log_line(&campaign_load_link_lost_line());
            for line in self.gate.link_went_down(net_clock_ms()) {
                log_line(&line);
            }
        }
        if step.ended {
            self.end_a_campaign_load(step.link_lost);
            return;
        }
        if !self.load_watch.is_loading() {
            return;
        }
        if step.phase_is_a_system_phase && self.gate.wants_hold() {
            log_line(&campaign_load_not_held_line(
                self.calls.read_i32(CURRENT_PHASE_ID_VALUE).unwrap_or(0),
            ));
            self.gate = LoadGate::unused();
        }
        set_load_end_hold_wanted(self.gate.wants_hold());
        self.run_held_load(now_ms, true);
    }

    fn begin_a_campaign_load(&mut self) {
        reset_load_end_hold();
        self.gate = LoadGate::for_a_campaign_load();
        self.partner_data = PartnerDataWait::default();
        self.state_without_body_sent_at_ms = None;
        self.loading_screen_logged = None;
        self.loading_screen_lines = 0;
        self.hold_waiting_since_ms = None;
        self.hold_waiting_logged_at_ms = None;
        log_line(&campaign_load_began_line(&local_story_place()));
    }

    fn end_a_campaign_load(&mut self, link_lost: bool) {
        set_load_end_hold_wanted(false);
        self.gate = LoadGate::unused();
        set_local_load_signal(LocalLoadSignal::default());
        self.state_without_body_sent_at_ms = None;
        if !link_lost {
            log_line(&campaign_load_ended_line(&local_story_place()));
        }
    }
}
