use crate::save_together::{
    GUEST_SAVE_NOT_TAKEN_AFTER_MS, GUEST_SAVE_WAIT_LIMIT_MS, GUEST_SAVE_WRITE_LIMIT_MS,
    MOST_HOST_SAVES_TOLD_PER_TICK, SAVE_WAITS_LOG_INTERVAL_MS, SaveReadings, SaveStep,
    guest_asks_line, guest_dropped_line, guest_gave_up_line, guest_heard_line,
    guest_save_failed_line, guest_save_not_taken_line, guest_save_timed_out_line, guest_saved_line,
    guest_waits_line, host_not_told_line, host_told_line,
};
use crate::travel::JumpRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WaitingSave {
    number: u32,
    since_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WritingSave {
    number: u32,
    since_ms: u64,
    seen_busy: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SaveMachine {
    last_number_told: u32,
    last_number_heard: Option<u32>,
    waiting: Option<WaitingSave>,
    writing: Option<WritingSave>,
    last_wait_log_ms: Option<u64>,
}

impl SaveMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn wants_gates(&self) -> bool {
        self.waiting.is_some() || self.writing.is_some()
    }

    pub fn save_requested(&mut self, busy_right_after: bool) {
        if let Some(writing) = self.writing.as_mut() {
            writing.seen_busy = busy_right_after;
        }
    }

    pub fn step(&mut self, readings: &SaveReadings) -> SaveStep {
        let mut step = SaveStep::default();
        if readings.role == JumpRole::Host {
            self.tell_the_guest(readings, &mut step);
        }
        if readings.role == JumpRole::Guest {
            self.hear_the_host(readings, &mut step);
        }
        self.drop_waiting_save_without_a_guest_link(readings, &mut step);
        self.watch_the_write(readings, &mut step);
        self.wait_for_the_gates(readings, &mut step);
        step
    }

    fn tell_the_guest(&mut self, readings: &SaveReadings, step: &mut SaveStep) {
        for _ in 0..readings.saves_asked_by_the_game {
            if !readings.both_in_play() {
                step.logs
                    .push(host_not_told_line(readings.why_the_host_cannot_tell()));
            } else if step.tell.len() as u32 >= MOST_HOST_SAVES_TOLD_PER_TICK {
                step.logs
                    .push(host_not_told_line("too many saves in one frame"));
            } else {
                self.last_number_told = self.last_number_told.wrapping_add(1);
                step.tell.push(self.last_number_told);
                step.logs.push(host_told_line(self.last_number_told));
            }
        }
    }

    fn hear_the_host(&mut self, readings: &SaveReadings, step: &mut SaveStep) {
        for &number in &readings.host_saves_heard {
            if self.last_number_heard == Some(number) {
                continue;
            }
            self.last_number_heard = Some(number);
            let replaced = self.waiting.map(|waiting| waiting.number);
            step.logs.push(guest_heard_line(number, replaced));
            self.waiting = Some(WaitingSave {
                number,
                since_ms: readings.now_ms,
            });
        }
    }

    fn drop_waiting_save_without_a_guest_link(
        &mut self,
        readings: &SaveReadings,
        step: &mut SaveStep,
    ) {
        let linked_guest = readings.role == JumpRole::Guest && readings.campaign_partner_present;
        if linked_guest {
            return;
        }
        if let Some(waiting) = self.waiting.take() {
            step.logs.push(guest_dropped_line(
                waiting.number,
                "this game is no longer a linked campaign guest",
            ));
        }
    }

    fn watch_the_write(&mut self, readings: &SaveReadings, step: &mut SaveStep) {
        let Some(mut writing) = self.writing else {
            return;
        };
        let Some(gates) = readings.gates.as_ref() else {
            return;
        };
        let took_ms = readings.now_ms.saturating_sub(writing.since_ms);
        if gates.save_busy == Some(true) {
            writing.seen_busy = true;
        }
        if writing.seen_busy && gates.save_busy == Some(false) {
            step.logs.push(if gates.save_failed == Some(true) {
                guest_save_failed_line(writing.number, took_ms)
            } else {
                guest_saved_line(writing.number, took_ms)
            });
            self.writing = None;
        } else if took_ms > GUEST_SAVE_WRITE_LIMIT_MS {
            step.logs
                .push(guest_save_timed_out_line(writing.number, took_ms));
            self.writing = None;
        } else if !writing.seen_busy && took_ms > GUEST_SAVE_NOT_TAKEN_AFTER_MS {
            step.logs
                .push(guest_save_not_taken_line(writing.number, took_ms));
            self.writing = None;
        } else {
            self.writing = Some(writing);
        }
    }

    fn wait_for_the_gates(&mut self, readings: &SaveReadings, step: &mut SaveStep) {
        let Some(waiting) = self.waiting else {
            return;
        };
        if self.writing.is_some() {
            return;
        }
        let waited_ms = readings.now_ms.saturating_sub(waiting.since_ms);
        let reason = match (readings.local_in_play, readings.gates.as_ref()) {
            (false, _) => Some("this game is not in play".to_owned()),
            (true, None) => Some("the player and the save manager were not read".to_owned()),
            (true, Some(gates)) => gates.why_not_ready(),
        };
        let Some(reason) = reason else {
            self.waiting = None;
            self.last_wait_log_ms = None;
            self.writing = Some(WritingSave {
                number: waiting.number,
                since_ms: readings.now_ms,
                seen_busy: false,
            });
            step.request_save = true;
            step.logs.push(guest_asks_line(waiting.number, waited_ms));
            return;
        };
        if waited_ms > GUEST_SAVE_WAIT_LIMIT_MS {
            self.waiting = None;
            self.last_wait_log_ms = None;
            step.logs
                .push(guest_gave_up_line(waiting.number, waited_ms, &reason));
            return;
        }
        let due = self
            .last_wait_log_ms
            .is_none_or(|last| readings.now_ms.saturating_sub(last) >= SAVE_WAITS_LOG_INTERVAL_MS);
        if due {
            self.last_wait_log_ms = Some(readings.now_ms);
            step.logs
                .push(guest_waits_line(waiting.number, waited_ms, &reason));
        }
    }
}
