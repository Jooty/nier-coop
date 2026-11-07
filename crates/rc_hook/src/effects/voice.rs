use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::effects::{
    FirstTimeNames, VOICE_SWITCH_GROUP, VoiceCharacterSchedule, read_character_state,
};
use rc_proto::{Message, VoiceCharacter};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::effects::calls::set_switch;
use crate::memory::InProcessMemory;
use crate::net;

static VOICE: LazyLock<Mutex<Voice>> = LazyLock::new(|| Mutex::new(Voice::new()));

struct Voice {
    partner_character: Option<String>,
    schedule: VoiceCharacterSchedule,
    played_names: FirstTimeNames,
}

impl Voice {
    fn new() -> Self {
        Self {
            partner_character: None,
            schedule: VoiceCharacterSchedule::new(),
            played_names: FirstTimeNames::new(),
        }
    }
}

pub(super) fn send_local_character(build: &VerifiedBuild, now_ms: u64) {
    let Some(net) = net::handle() else {
        return;
    };
    let Some(state) = Replicant::new(InProcessMemory::new(build))
        .playable_actor()
        .ok()
        .and_then(|actor| read_character_state(&InProcessMemory::new(build), actor).ok())
        .flatten()
    else {
        return;
    };
    let mut voice = VOICE.lock().unwrap_or_else(PoisonError::into_inner);
    if voice.schedule.is_due(&state, now_ms) {
        log_line(&format!("voice: sending my character {state}"));
        net.send_relayed(Message::VoiceCharacter(VoiceCharacter { state }));
    }
}

pub(super) fn remember_partner_character(arrived: &VoiceCharacter) {
    let mut voice = VOICE.lock().unwrap_or_else(PoisonError::into_inner);
    if voice.partner_character.as_deref() != Some(arrived.state.as_str()) {
        log_line(&format!("voice: partner character is {}", arrived.state));
        voice.partner_character = Some(arrived.state.clone());
    }
}

pub(super) fn forget_sent_character() {
    VOICE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .schedule
        .reset();
}

pub(super) fn play_as_partner_character(build: &VerifiedBuild, puppet: u64, sound: &str) {
    let mut voice = VOICE.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(character) = voice.partner_character.clone() else {
        return;
    };
    let switched = set_switch(build, puppet, VOICE_SWITCH_GROUP, &character);
    if voice.played_names.is_first(sound) {
        log_line(&format!(
            "voice: partner {sound} played on puppet {puppet:#x} as {character} (switch set: {switched})"
        ));
    }
}
