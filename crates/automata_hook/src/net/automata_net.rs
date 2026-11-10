use std::time::{Duration, Instant};

use automata_game::character::Character;
use automata_game::partner::{BodySnapshot, partner_character_note};
use automata_proto::{CharacterHello, Message, PlayerState};
use nc_hook::diagnostics::log_line;
use nc_hook::net::{LinkAccess, NetGame, RemotePlayer};
use nc_net::Channel;

use crate::net::link_state::AutomataLinkState;

#[derive(Debug)]
pub struct AutomataNet {
    partner_setting: Character,
    first_state_logged: bool,
}

impl AutomataNet {
    pub fn new(partner_setting: Character) -> Self {
        Self {
            partner_setting,
            first_state_logged: false,
        }
    }

    fn on_partner_state(&mut self, link: &LinkAccess<'_, Self>, state: PlayerState) {
        let mut shared = link.shared();
        shared.extras.remote_input.receive(&state.to_snapshot());
        let name = shared.peer_name.clone().unwrap_or_default();
        let slot = u8::from(shared.local_slot == Some(0));
        shared.remote = Some(RemotePlayer {
            name,
            slot,
            state,
            received_at: Instant::now(),
        });
        drop(shared);
        if !self.first_state_logged {
            self.first_state_logged = true;
            log_line(&format!(
                "net: first state from the other player (sequence {}, state {:#X})",
                state.sequence, state.state
            ));
        }
    }

    fn on_partner_character(&self, link: &LinkAccess<'_, Self>, hello: CharacterHello) {
        link.shared().extras.partner_character = Some(hello.character);
        log_line(&format!(
            "net: the other player plays {}",
            hello.character.launcher_id()
        ));
        if let Some(note) = partner_character_note(self.partner_setting, hello.character) {
            log_line(&note);
        }
    }
}

impl NetGame for AutomataNet {
    type Message = Message;
    type Snapshot = BodySnapshot;
    type PeerState = PlayerState;
    type Extras = AutomataLinkState;

    fn kind_name(message: &Message) -> &'static str {
        message.kind_name()
    }

    fn peer_in_play(_extras: &AutomataLinkState) -> bool {
        true
    }

    fn snapshot_message(&mut self, snapshot: &BodySnapshot, tick: u32) -> Message {
        Message::State(PlayerState::from_snapshot(snapshot, tick))
    }

    fn on_message(&mut self, link: &LinkAccess<'_, Self>, message: Message) {
        match message {
            Message::State(state) => self.on_partner_state(link, state),
            Message::Character(hello) => self.on_partner_character(link, hello),
            Message::PathInfo(info) => {
                let mut shared = link.shared();
                shared.rtt = Some(Duration::from_micros(u64::from(info.rtt_micros)));
                shared.direct_path = Some(info.direct);
            }
            _ => {}
        }
    }

    fn on_connected(&mut self, extras: &mut AutomataLinkState, peer_name: &str) {
        *extras = AutomataLinkState::default();
        self.first_state_logged = false;
        log_line(&format!("net: connected to {peer_name}"));
    }

    fn on_peer_cleared(&mut self, extras: &mut AutomataLinkState) {
        *extras = AutomataLinkState::default();
    }

    fn on_launcher_linked(&mut self) {
        log_line("net: the launcher link is up");
    }

    fn on_idle(
        &mut self,
        _link: &LinkAccess<'_, Self>,
        _send: &mut dyn FnMut(&Message, Channel) -> bool,
    ) {
    }

    fn announce(&self, text: &str) {
        log_line(&format!("net: {text}"));
    }
}
