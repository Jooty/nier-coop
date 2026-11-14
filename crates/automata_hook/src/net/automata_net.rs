use std::time::{Duration, Instant};

use automata_game::character::Character;
use automata_game::enemy_lead::MOST_ENEMY_LEADS_WAITING;
use automata_game::exe::{ExeVariant, exe_difference_line};
use automata_game::load_gate::StoryPlace;
use automata_game::partner::{BodySnapshot, partner_character_note};
use automata_proto::{CharacterHello, LoadSignal, Message, PlayerState};
use nt_hook::diagnostics::log_line;
use nt_hook::net::{LinkAccess, NetGame, NetState, PlayerOf, RemotePlayer, net_clock_ms};
use nt_net::Channel;

use crate::net::attendance::local_unattended;
use crate::net::flight_report::local_flight_report;
use crate::net::link_state::AutomataLinkState;
use crate::net::load_signal::{HeardLoadSignal, is_snapshot_without_body, local_load_signal};
use crate::net::local_play::{ReadinessPacer, local_in_play, readiness_of};
use crate::net::partner_watch::PartnerWatch;
use crate::net::playing_event::local_playing_event;
use crate::net::story_place::local_story_place;
use crate::net::watching::local_watching;

#[derive(Debug)]
pub struct AutomataNet {
    partner_setting: Character,
    local_exe: ExeVariant,
    first_state_logged: bool,
    first_state_without_body_logged: bool,
    readiness: ReadinessPacer,
}

impl AutomataNet {
    pub fn new(partner_setting: Character, local_exe: ExeVariant) -> Self {
        Self {
            partner_setting,
            local_exe,
            first_state_logged: false,
            first_state_without_body_logged: false,
            readiness: ReadinessPacer::default(),
        }
    }

    fn on_partner_state(&mut self, link: &LinkAccess<'_, Self>, from: u8, state: PlayerState) {
        let heard = HeardLoadSignal {
            signal: state.load,
            heard_at_ms: net_clock_ms(),
        };
        if state.load.without_body {
            self.on_partner_state_without_body(link, from, heard, state.place);
            return;
        }
        let mut shared = link.shared();
        let Some(player) = shared.players.get_mut(&from) else {
            return;
        };
        player.watch.load = Some(heard);
        player.watch.place = state.place;
        player.watch.heard = true;
        player.latest = Some(RemotePlayer {
            name: player.name.clone(),
            slot: from,
            state,
            received_at: Instant::now(),
        });
        shared
            .extras
            .remote_inputs
            .receive(from, &state.to_snapshot());
        drop(shared);
        if !self.first_state_logged {
            self.first_state_logged = true;
            log_line(&format!(
                "net: first state from the other player (sequence {}, state {:#X})",
                state.sequence, state.state
            ));
        }
    }

    fn on_partner_state_without_body(
        &mut self,
        link: &LinkAccess<'_, Self>,
        from: u8,
        heard: HeardLoadSignal,
        place: StoryPlace,
    ) {
        {
            let mut shared = link.shared();
            let Some(player) = shared.players.get_mut(&from) else {
                return;
            };
            player.watch.load = Some(heard);
            player.watch.place = place;
            player.watch.heard = true;
        }
        if !self.first_state_without_body_logged {
            self.first_state_without_body_logged = true;
            log_line(&format!(
                "net: first word from the other player's loading screen (load done and waiting: {})",
                heard.signal.waiting
            ));
        }
    }

    fn on_partner_character(&self, link: &LinkAccess<'_, Self>, from: u8, hello: CharacterHello) {
        {
            let mut shared = link.shared();
            let Some(player) = shared.players.get_mut(&from) else {
                log_line(&format!(
                    "net: a character hello from slot {from}, which holds no player, was dropped"
                ));
                return;
            };
            player.watch.character = Some(hello.character);
            player.watch.heard = true;
        }
        log_line(&format!(
            "net: the other player plays {}",
            hello.character.launcher_id()
        ));
        if let Some(note) = partner_character_note(self.partner_setting, hello.character) {
            log_line(&note);
        }
        if let Some(line) = exe_difference_line(self.local_exe, hello.exe) {
            log_line(&line);
        }
    }

    fn tell_launcher_readiness(
        &mut self,
        link: &LinkAccess<'_, Self>,
        send: &mut dyn FnMut(&Message, Channel) -> bool,
    ) {
        if link.shared().state == NetState::WaitingForLauncher {
            return;
        }
        let current = readiness_of(local_in_play());
        let now_ms = net_clock_ms();
        if self.readiness.is_due(now_ms, current)
            && send(&Message::Readiness(current), Channel::ReliableOrdered)
        {
            if self.readiness.is_new(current) {
                log_line(&format!("readiness: this game {current:?}"));
            }
            self.readiness.mark_sent(now_ms, current);
        }
    }
}

impl NetGame for AutomataNet {
    type Message = Message;
    type Snapshot = BodySnapshot;
    type PeerState = PlayerState;
    type PlayerWatch = PartnerWatch;
    type Extras = AutomataLinkState;

    fn kind_name(message: &Message) -> &'static str {
        message.kind_name()
    }

    fn player_in_play(_watch: &PartnerWatch) -> bool {
        true
    }

    fn snapshot_message(&mut self, snapshot: &BodySnapshot, tick: u32) -> Message {
        Message::State(PlayerState {
            load: LoadSignal {
                without_body: is_snapshot_without_body(snapshot),
                ..local_load_signal()
            },
            place: local_story_place(),
            playing_event: local_playing_event(),
            flight: local_flight_report(),
            unattended: local_unattended(),
            watching: local_watching(),
            ..PlayerState::from_snapshot(snapshot, tick)
        })
    }

    fn on_message(&mut self, link: &LinkAccess<'_, Self>, from: u8, message: Message) {
        match message {
            Message::State(state) => self.on_partner_state(link, from, state),
            Message::Character(hello) => self.on_partner_character(link, from, hello),
            Message::PathInfo(info) => {
                let mut shared = link.shared();
                shared.rtt = Some(Duration::from_micros(u64::from(info.rtt_micros)));
                shared.direct_path = Some(info.direct);
            }
            // The game thread takes these from the inbox on its next tick.
            Message::Teleport(_)
            | Message::EnemyRoster(_)
            | Message::EnemyHits(_)
            | Message::EnemyDefeated(_)
            | Message::EnemySpawned(_)
            | Message::EnemyGone(_)
            | Message::GeneratorStates(_)
            | Message::LiveGenerators(_)
            | Message::Pickup(_)
            | Message::StoryMove(_)
            | Message::StoryAsk(_)
            | Message::StoryAskAnswer(_)
            | Message::EventStarted { .. }
            | Message::EventEnded { .. }
            | Message::EventAsk(_)
            | Message::HostSaved { .. }
            | Message::SkipAsk { .. }
            | Message::DefeatNote(_)
            | Message::StoryStepDone(_)
            | Message::StoryStepAsk(_)
            | Message::StoryStepAnswer(_)
            | Message::EventPause { .. } => link.push_relayed(from, message),
            // Kept apart from the inbox, so a flood of them never pushes a spawn message out.
            Message::EnemyLead(lead) => {
                let mut shared = link.shared();
                let leads = &mut shared.extras.enemy_leads;
                if leads.len() >= MOST_ENEMY_LEADS_WAITING {
                    leads.pop_front();
                }
                leads.push_back((from, lead));
            }
            _ => {}
        }
    }

    fn on_player_joined(&mut self, extras: &mut AutomataLinkState, player: &mut PlayerOf<Self>) {
        extras.player_joined(player.slot);
        self.first_state_logged = false;
        self.first_state_without_body_logged = false;
        log_line(&format!("net: connected to {}", player.name));
    }

    fn on_player_left(&mut self, extras: &mut AutomataLinkState, slot: u8) {
        extras.player_left(slot);
    }

    fn on_peer_cleared(&mut self, extras: &mut AutomataLinkState) {
        *extras = AutomataLinkState::default();
    }

    fn on_launcher_linked(&mut self) {
        self.readiness.forget();
        log_line("net: the launcher link is up");
    }

    fn on_idle(
        &mut self,
        link: &LinkAccess<'_, Self>,
        send: &mut dyn FnMut(&Message, Channel) -> bool,
    ) {
        self.tell_launcher_readiness(link, send);
    }

    fn announce(&self, text: &str) {
        log_line(&format!("net: {text}"));
    }
}
