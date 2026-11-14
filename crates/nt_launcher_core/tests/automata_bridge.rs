use std::sync::Arc;
use std::thread;
use std::time::Duration;

use automata_game::build::{COMPAT_PE_IDENTITY, MAIN_PE_IDENTITY};
use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_proto::{CharacterHello, Member, Message, PROTOCOL_VERSION, PlayerState};
use nt_launcher_core::{
    AUTOMATA, LauncherConfig, LauncherCore, LauncherEvent, LauncherNet, NetState, REPLICANT,
    TransportFactory, version_difference_text,
};
use nt_net::{
    Channel, Invite, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER, LocalLinkTransport, LoopbackNetwork,
    Transport, TransportEvent,
};
use nt_wire::{
    BuildGate, Envelope, GameReadiness, Hello, LinkPayload, Reject, RejectReason, STORY_MODE,
    SessionRules, Welcome, decode_enveloped_ignoring_trailing, encode_enveloped,
};
use replicant_game::build::PeIdentity;
use replicant_proto::{CampaignCard, SaveSummary};

use crate::bridge::wait_until;

const MAIN_BUILD: BuildGate = build_gate_of(MAIN_PE_IDENTITY);
const COMPAT_BUILD: BuildGate = build_gate_of(COMPAT_PE_IDENTITY);
const TWO_PLAYER_RULES: SessionRules = SessionRules {
    mode: STORY_MODE,
    max_players: 2,
    map: 0,
};
const OWN_GAME_ENVELOPE: Envelope = Envelope { from: 0, to: None };

struct FakeHook {
    link: LocalLinkTransport,
    received: Vec<Message>,
    envelopes: Vec<Envelope>,
    frames: Vec<Vec<u8>>,
}

struct Rig {
    host: LauncherCore,
    guest: LauncherCore,
    host_hook: FakeHook,
    guest_hook: FakeHook,
}

impl FakeHook {
    fn attach(core: &LauncherCore, build: BuildGate) -> Self {
        let mut link = LocalLinkTransport::game(build);
        let invite =
            Invite::new(format!("{LINK_INVITE_PREFIX}{}", core.link_port())).expect("invite");
        link.join(&invite).expect("dials");
        Self {
            link,
            received: Vec::new(),
            envelopes: Vec::new(),
            frames: Vec::new(),
        }
    }

    fn send(&mut self, message: &Message) {
        let bytes = encode_enveloped(&OWN_GAME_ENVELOPE, message).expect("encodes");
        let channel = if message.is_latest_wins() {
            Channel::UnreliableLatest
        } else {
            Channel::ReliableOrdered
        };
        let _ = self.link.send(LINK_LAUNCHER_PEER, channel, &bytes);
    }

    fn send_frame(&mut self, frame: &[u8]) {
        let _ = self
            .link
            .send(LINK_LAUNCHER_PEER, Channel::UnreliableLatest, frame);
    }

    fn pump(&mut self) {
        let mut events = Vec::new();
        self.link.poll(&mut events);
        for event in events {
            if let TransportEvent::Message { payload, .. } = event {
                let (envelope, message, _) = decode_enveloped_ignoring_trailing(&payload)
                    .expect("the launcher sends this game's messages, enveloped");
                self.frames.push(payload);
                self.envelopes.push(envelope);
                self.received.push(message);
            }
        }
    }

    fn saw(&mut self, matcher: impl Fn(&Message) -> bool) -> bool {
        self.pump();
        self.received.iter().any(matcher)
    }

    fn sender_of(&mut self, matcher: impl Fn(&Message) -> bool) -> Option<u8> {
        self.pump();
        let index = self.received.iter().position(matcher)?;
        Some(self.envelopes[index].from)
    }

    fn position_of(&mut self, matcher: impl Fn(&Message) -> bool) -> Option<usize> {
        self.pump();
        self.received.iter().position(matcher)
    }
}

const fn build_gate_of(identity: PeIdentity) -> BuildGate {
    BuildGate {
        time_date_stamp: identity.time_date_stamp,
        size_of_image: identity.size_of_image,
    }
}

fn loopback_factory(network: &LoopbackNetwork) -> TransportFactory {
    let network = network.clone();
    Arc::new(move || Ok(Box::new(network.endpoint()) as Box<dyn Transport>))
}

fn automata_core(network: &LoopbackNetwork, name: &str) -> LauncherCore {
    LauncherCore::for_game(&AUTOMATA, name, loopback_factory(network)).expect("core")
}

fn replicant_core(network: &LoopbackNetwork, name: &str) -> LauncherCore {
    LauncherCore::new(LauncherConfig::new(name), loopback_factory(network)).expect("core")
}

fn connected_rig() -> Rig {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let guest = automata_core(&network, "Guesty");
    let host_hook = FakeHook::attach(&host, MAIN_BUILD);
    let guest_hook = FakeHook::attach(&guest, COMPAT_BUILD);
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    let mut rig = Rig {
        host,
        guest,
        host_hook,
        guest_hook,
    };
    assert!(wait_until(|| rig.host_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Guesty" && w.slot == 0
    ))));
    assert!(wait_until(|| rig.guest_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Hosty" && w.slot == 1
    ))));
    rig
}

fn state_at(sequence: u32, x: f32) -> PlayerState {
    PlayerState {
        sequence,
        held: 1,
        pressed: 1,
        speed: 1.0,
        world_facing: 0.5,
        position: [x, 0.0, 0.0],
        body_facing: 0.25,
        state: 3,
        last_start: None,
        health: 100,
        max_health: 100,
        free: true,
        phase_id: 0x100,
        paused: false,
        jump_pending: false,
        load: Default::default(),
        pod_aim: None,
        downed: Default::default(),
        place: Default::default(),
        playing_event: None,
        flight: Default::default(),
        unattended: false,
        watching: false,
    }
}

fn saw_event(core: &LauncherCore, wanted: impl Fn(&LauncherEvent) -> bool) -> bool {
    wait_until(|| core.drain_events().iter().any(&wanted))
}

fn failure_text(core: &LauncherCore) -> Option<String> {
    match core.status().state {
        NetState::Failed(text) => Some(text),
        _ => None,
    }
}

#[test]
fn both_hooks_are_welcomed_with_this_games_protocol_version() {
    let mut rig = connected_rig();
    assert!(rig.host_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.version == PROTOCOL_VERSION
    )));
    assert!(rig.guest_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.version == PROTOCOL_VERSION
    )));
    assert!(wait_until(|| rig.host.link_status().game_linked));
}

#[test]
fn player_state_crosses_both_ways_between_the_hooks() {
    let mut rig = connected_rig();
    assert!(wait_until(|| {
        rig.host_hook.send(&Message::State(state_at(5, 11.0)));
        rig.guest_hook
            .saw(|m| matches!(m, Message::State(s) if s.position[0] == 11.0))
    }));
    assert!(wait_until(|| {
        rig.guest_hook.send(&Message::State(state_at(9, 22.0)));
        rig.host_hook
            .saw(|m| matches!(m, Message::State(s) if s.position[0] == 22.0))
    }));
}

#[test]
fn the_character_hello_crosses_both_ways_between_the_hooks() {
    let mut rig = connected_rig();
    rig.host_hook.send(&Message::Character(CharacterHello {
        character: Character::TwoB,
        exe: ExeVariant::Main,
    }));
    rig.guest_hook.send(&Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    }));
    assert!(wait_until(|| rig.guest_hook.saw(|m| matches!(
        m,
        Message::Character(hello) if hello.character == Character::TwoB
    ))));
    assert!(wait_until(|| rig.host_hook.saw(|m| matches!(
        m,
        Message::Character(hello) if hello.character == Character::A2
    ))));
}

#[test]
fn the_ping_of_the_session_reaches_both_hooks() {
    let mut rig = connected_rig();
    assert!(wait_until(|| rig
        .host_hook
        .saw(|m| matches!(m, Message::PathInfo(info) if info.direct))));
    assert!(wait_until(|| rig
        .guest_hook
        .saw(|m| matches!(m, Message::PathInfo(info) if info.direct))));
}

#[test]
fn the_lobby_character_pick_reaches_the_other_launcher_and_no_hook() {
    let mut rig = connected_rig();
    rig.host.send_lobby_character(0);
    rig.guest.send_lobby_character(2);
    assert!(wait_until(
        || rig.host.link_status().partner_character() == Some(2)
    ));
    assert!(wait_until(
        || rig.guest.link_status().partner_character() == Some(0)
    ));
    thread::sleep(Duration::from_millis(200));
    assert!(
        !rig.host_hook
            .saw(|m| matches!(m, Message::LobbyCharacter(_)))
    );
    assert!(
        !rig.guest_hook
            .saw(|m| matches!(m, Message::LobbyCharacter(_)))
    );
}

#[test]
fn a_character_set_before_joining_is_sent_when_the_guest_arrives() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let guest = automata_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.send_lobby_character(2);
    guest.join(&ticket).expect("join starts");
    assert!(wait_until(
        || host.link_status().partner_character() == Some(2)
    ));
}

#[test]
fn the_campaign_card_and_a_seat_never_leave_a_launcher_of_this_game() {
    let rig = connected_rig();
    rig.host.send_campaign_card(CampaignCard {
        campaign_id: "0123456789abcdef0123456789abcdef".to_owned(),
        host: SaveSummary {
            name: "Hosty".to_owned(),
            level: 1,
            play_seconds: 60.0,
        },
        second_seat: None,
    });
    rig.host.send_seat(1, vec![7; 4096]);
    thread::sleep(Duration::from_millis(300));
    let save_work = |event: &LauncherEvent| {
        matches!(
            event,
            LauncherEvent::CampaignCardReceived { .. }
                | LauncherEvent::SeatReceived { .. }
                | LauncherEvent::SeatProgress { .. }
                | LauncherEvent::SeatTransferRefused { .. }
        )
    };
    assert!(!rig.guest.drain_events().iter().any(save_work));
    assert!(!rig.host.drain_events().iter().any(save_work));
}

#[test]
fn the_guest_launcher_hears_that_the_hosts_game_is_linked() {
    let rig = connected_rig();
    assert!(wait_until(
        || rig.guest.link_status().partner_readiness() == GameReadiness::Loading
    ));
}

#[test]
fn the_lobby_start_reaches_the_guests_launcher() {
    let rig = connected_rig();
    assert!(wait_until(|| {
        rig.host.start_lobby(0);
        rig.guest
            .drain_events()
            .iter()
            .any(|event| matches!(event, LauncherEvent::LobbyStartReceived { mode: 0 }))
    }));
}

#[test]
fn a_hook_of_the_other_game_cannot_attach() {
    let network = LoopbackNetwork::new();
    let core = automata_core(&network, "Hosty");
    let _hook = FakeHook::attach(&core, LauncherConfig::new("x").build);
    thread::sleep(Duration::from_millis(700));
    assert!(!core.link_status().game_linked);
}

#[test]
fn a_launcher_changed_to_this_game_takes_its_hook_and_talks_to_its_launchers() {
    let network = LoopbackNetwork::new();
    let host = replicant_core(&network, "Hosty");
    host.set_game(&AUTOMATA);
    let guest = automata_core(&network, "Guesty");
    let mut host_hook = FakeHook::attach(&host, COMPAT_BUILD);
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(wait_until(|| host_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Guesty" && w.version == PROTOCOL_VERSION
    ))));
}

#[test]
fn a_character_picked_under_this_game_is_never_sent_under_the_other_game() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    host.send_lobby_character(1);
    host.set_game(&REPLICANT);
    let guest = replicant_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(wait_until(|| guest
        .link_status()
        .partner()
        .map(|partner| partner.name.as_str())
        == Some("Hosty")));
    host.send_lobby_character(2);
    thread::sleep(Duration::from_millis(300));
    assert_eq!(guest.link_status().partner_character(), None);
    assert_eq!(failure_text(&guest), None);
}

#[test]
fn a_host_on_this_game_refuses_a_guest_on_the_other_game_and_says_why() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let guest = replicant_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(saw_event(&host, |event| matches!(
        event,
        LauncherEvent::PeerOnOtherVersion {
            ours: PROTOCOL_VERSION,
            theirs: replicant_proto::PROTOCOL_VERSION
        }
    )));
    assert!(wait_until(|| failure_text(&guest).is_some()));
    let text = failure_text(&guest).expect("the guest shows why");
    assert!(text.contains("another game open"), "{text}");
}

#[test]
fn a_host_on_the_other_game_refuses_a_guest_on_this_game_and_says_why() {
    let network = LoopbackNetwork::new();
    let host = replicant_core(&network, "Hosty");
    let guest = automata_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(saw_event(&host, |event| matches!(
        event,
        LauncherEvent::PeerOnOtherVersion {
            ours: replicant_proto::PROTOCOL_VERSION,
            theirs: PROTOCOL_VERSION
        }
    )));
    assert!(wait_until(|| failure_text(&guest).is_some()));
    let text = failure_text(&guest).expect("the guest shows why");
    assert!(text.contains("another game open"), "{text}");
}

#[test]
fn the_refusal_line_names_both_games_or_both_versions() {
    let replicant_version = replicant_proto::PROTOCOL_VERSION;
    let games = version_difference_text(replicant_version, PROTOCOL_VERSION);
    assert!(games.contains(AUTOMATA.display_name), "{games}");
    assert!(games.contains(REPLICANT.display_name), "{games}");
    let older_version = replicant_version - 1;
    let releases = version_difference_text(replicant_version, older_version);
    assert!(
        releases.contains(&format!("protocol {older_version}")),
        "{releases}"
    );
    assert!(
        releases.contains(&replicant_version.to_string()),
        "{releases}"
    );
}

#[test]
fn the_handshake_bytes_are_the_same_under_both_games() {
    let payloads = [
        LinkPayload::Hello(Hello {
            version: replicant_proto::PROTOCOL_VERSION,
            build: MAIN_BUILD,
            name: "Jay".to_owned(),
        }),
        LinkPayload::Welcome(Welcome {
            version: PROTOCOL_VERSION,
            slot: 1,
            host_name: "Host".to_owned(),
            rules: TWO_PLAYER_RULES,
        }),
        LinkPayload::Reject(Reject {
            reason: RejectReason::VersionMismatch,
        }),
    ];
    for payload in payloads {
        let this_game: Message = nt_wire::GameMessage::from_link(payload.clone());
        let other_game: replicant_proto::Message = nt_wire::GameMessage::from_link(payload);
        assert_eq!(
            automata_proto::encode(&this_game).expect("encodes"),
            replicant_proto::encode(&other_game).expect("encodes")
        );
    }
}

#[test]
fn each_hook_hears_the_other_player_as_a_member_right_after_its_welcome() {
    let mut rig = connected_rig();
    let guest_member = Message::Member(Member {
        slot: 1,
        name: "Guesty".to_owned(),
    });
    let host_member = Message::Member(Member {
        slot: 0,
        name: "Hosty".to_owned(),
    });
    assert!(wait_until(|| rig.host_hook.saw(|m| *m == guest_member)));
    assert!(wait_until(|| rig.guest_hook.saw(|m| *m == host_member)));
    for (hook, member) in [
        (&mut rig.host_hook, &guest_member),
        (&mut rig.guest_hook, &host_member),
    ] {
        let welcome = hook
            .position_of(|m| matches!(m, Message::Welcome(_)))
            .expect("welcomed");
        let announced = hook.position_of(|m| m == member).expect("announced");
        assert_eq!(announced, welcome + 1);
    }
}

#[test]
fn a_partners_frames_reach_the_hook_with_the_partners_slot() {
    let mut rig = connected_rig();
    let mut sequence = 0;
    assert!(wait_until(|| {
        sequence += 1;
        rig.host_hook.send(&Message::State(state_at(sequence, 7.0)));
        rig.guest_hook
            .sender_of(|m| matches!(m, Message::State(s) if s.position[0] == 7.0))
            == Some(0)
    }));
    assert!(wait_until(|| {
        sequence += 1;
        rig.guest_hook
            .send(&Message::State(state_at(sequence, 9.0)));
        rig.host_hook
            .sender_of(|m| matches!(m, Message::State(s) if s.position[0] == 9.0))
            == Some(1)
    }));
}

#[test]
fn the_host_hook_hears_member_left_when_the_guest_goes() {
    let mut rig = connected_rig();
    rig.guest.shutdown();
    assert!(wait_until(|| rig
        .host_hook
        .saw(|m| *m == Message::MemberLeft { slot: 1 })));
    assert!(!rig.host_hook.saw(|m| *m == Message::Bye));
}

#[test]
fn the_guest_hook_hears_bye_when_the_host_goes() {
    let mut rig = connected_rig();
    rig.host.shutdown();
    assert!(wait_until(|| rig.guest_hook.saw(|m| *m == Message::Bye)));
}

const FREE_ROAM_RULES: SessionRules = SessionRules {
    mode: 1,
    max_players: 8,
    map: 0,
};

fn told_of_member(hook: &mut FakeHook, slot: u8) -> bool {
    hook.saw(|m| matches!(m, Message::Member(member) if member.slot == slot))
}

fn welcomed_slot(hook: &mut FakeHook) -> Option<u8> {
    hook.pump();
    hook.received.iter().find_map(|m| match m {
        Message::Welcome(welcome) => Some(welcome.slot),
        _ => None,
    })
}

fn slot_once_welcomed(hook: &mut FakeHook) -> u8 {
    let mut slot = None;
    assert!(wait_until(|| {
        slot = welcomed_slot(hook);
        slot.is_some()
    }));
    slot.expect("welcomed")
}

#[test]
fn a_free_roam_session_of_three_reaches_every_guest() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let first = automata_core(&network, "Firsty");
    let second = automata_core(&network, "Secondy");
    let mut host_hook = FakeHook::attach(&host, MAIN_BUILD);
    let mut first_hook = FakeHook::attach(&first, MAIN_BUILD);
    let mut second_hook = FakeHook::attach(&second, MAIN_BUILD);
    host.set_rules(FREE_ROAM_RULES);
    let ticket = host.host().expect("ticket");
    first.join(&ticket).expect("first joins");
    assert!(wait_until(|| told_of_member(&mut host_hook, 1)));
    second.join(&ticket).expect("second joins");
    assert!(wait_until(|| told_of_member(&mut host_hook, 2)));
    assert!(wait_until(
        || told_of_member(&mut first_hook, 0) && told_of_member(&mut first_hook, 2)
    ));
    assert!(wait_until(
        || told_of_member(&mut second_hook, 0) && told_of_member(&mut second_hook, 1)
    ));
    for hook in [&mut first_hook, &mut second_hook] {
        assert!(hook.saw(|m| matches!(
            m,
            Message::Welcome(w) if w.rules == FREE_ROAM_RULES
        )));
    }
    assert_eq!(host.link_status().rules, FREE_ROAM_RULES);
    assert_eq!(first.link_status().rules, FREE_ROAM_RULES);
    assert_eq!(second.link_status().rules, FREE_ROAM_RULES);
    assert_eq!(host.link_status().members.len(), 2);
    assert_eq!(first.link_status().members.len(), 2);
    assert_eq!(second.link_status().members.len(), 2);
    assert!(host.link_status().partner().is_none());

    let from_first = |m: &Message| matches!(m, Message::State(s) if s.position[0] == 31.0);
    assert!(wait_until(|| {
        first_hook.send(&Message::State(state_at(5, 31.0)));
        host_hook.saw(from_first) && second_hook.saw(from_first)
    }));
    assert_eq!(host_hook.sender_of(from_first), Some(1));
    assert_eq!(second_hook.sender_of(from_first), Some(1));
    assert!(!first_hook.saw(from_first));

    let from_second = |m: &Message| matches!(m, Message::State(s) if s.position[0] == 62.0);
    assert!(wait_until(|| {
        second_hook.send(&Message::State(state_at(9, 62.0)));
        host_hook.saw(from_second) && first_hook.saw(from_second)
    }));
    assert_eq!(host_hook.sender_of(from_second), Some(2));
    assert_eq!(first_hook.sender_of(from_second), Some(2));
    assert!(!second_hook.saw(from_second));
}

#[test]
fn the_hosts_map_reaches_every_guest_as_the_host_changes_it() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let first = automata_core(&network, "Firsty");
    let second = automata_core(&network, "Secondy");
    host.set_rules(FREE_ROAM_RULES);
    let ticket = host.host().expect("ticket");
    first.join(&ticket).expect("first joins");
    second.join(&ticket).expect("second joins");
    assert!(wait_until(|| first.link_status().rules == FREE_ROAM_RULES
        && second.link_status().rules == FREE_ROAM_RULES));
    for map in [3, 1] {
        let picked = SessionRules {
            map,
            ..FREE_ROAM_RULES
        };
        host.set_rules(picked);
        assert!(wait_until(
            || first.link_status().rules.map == map && second.link_status().rules.map == map
        ));
        assert_eq!(first.link_status().rules, picked);
    }
}

#[test]
fn a_guest_leaving_a_session_of_three_is_reported_to_the_others_and_the_session_goes_on() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let first = automata_core(&network, "Firsty");
    let second = automata_core(&network, "Secondy");
    let mut host_hook = FakeHook::attach(&host, MAIN_BUILD);
    let mut first_hook = FakeHook::attach(&first, MAIN_BUILD);
    let mut second_hook = FakeHook::attach(&second, MAIN_BUILD);
    host.set_rules(FREE_ROAM_RULES);
    let ticket = host.host().expect("ticket");
    first.join(&ticket).expect("first joins");
    second.join(&ticket).expect("second joins");
    let first_slot = slot_once_welcomed(&mut first_hook);
    let second_slot = slot_once_welcomed(&mut second_hook);
    assert!(wait_until(|| told_of_member(&mut first_hook, second_slot)));
    second.leave();
    let left = |m: &Message| matches!(m, Message::MemberLeft { slot } if *slot == second_slot);
    assert!(wait_until(|| host_hook.saw(left) && first_hook.saw(left)));
    assert!(wait_until(|| host.link_status().members.len() == 1));
    assert_eq!(
        host.link_status().partner().map(|p| p.slot),
        Some(first_slot)
    );
    assert!(!first_hook.saw(|m| matches!(m, Message::Bye)));
    assert!(wait_until(|| {
        first_hook.send(&Message::State(state_at(12, 77.0)));
        host_hook.saw(|m| matches!(m, Message::State(s) if s.position[0] == 77.0))
    }));
}

#[test]
fn the_guest_in_the_lower_slot_leaving_a_session_of_three_leaves_the_higher_slot_playing() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let earlier = automata_core(&network, "Earlier");
    let later = automata_core(&network, "Later");
    let mut host_hook = FakeHook::attach(&host, MAIN_BUILD);
    let mut earlier_hook = FakeHook::attach(&earlier, MAIN_BUILD);
    let mut later_hook = FakeHook::attach(&later, MAIN_BUILD);
    host.set_rules(FREE_ROAM_RULES);
    let ticket = host.host().expect("ticket");
    earlier.join(&ticket).expect("earlier joins");
    assert_eq!(slot_once_welcomed(&mut earlier_hook), 1);
    later.join(&ticket).expect("later joins");
    assert_eq!(slot_once_welcomed(&mut later_hook), 2);
    assert!(wait_until(|| told_of_member(&mut later_hook, 1)));
    earlier.leave();
    let left = |m: &Message| matches!(m, Message::MemberLeft { slot: 1 });
    assert!(wait_until(|| host_hook.saw(left) && later_hook.saw(left)));
    assert!(wait_until(|| host.link_status().members.len() == 1));
    assert_eq!(host.link_status().partner().map(|p| p.slot), Some(2));
    assert!(!later_hook.saw(|m| matches!(m, Message::Bye)));
    assert!(wait_until(|| {
        later_hook.send(&Message::State(state_at(13, 55.0)));
        host_hook.sender_of(|m| matches!(m, Message::State(s) if s.position[0] == 55.0)) == Some(2)
    }));
}

fn every_member_has_a_ping(core: &LauncherCore, expected_members: usize) -> bool {
    let members = core.link_status().members;
    members.len() == expected_members && members.iter().all(|member| member.rtt_ms.is_some())
}

#[test]
fn three_launchers_each_list_a_ping_for_every_member() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let first = automata_core(&network, "Firsty");
    let second = automata_core(&network, "Secondy");
    let _host_hook = FakeHook::attach(&host, MAIN_BUILD);
    let _first_hook = FakeHook::attach(&first, MAIN_BUILD);
    let _second_hook = FakeHook::attach(&second, MAIN_BUILD);
    host.set_rules(FREE_ROAM_RULES);
    let ticket = host.host().expect("ticket");
    first.join(&ticket).expect("first joins");
    second.join(&ticket).expect("second joins");
    assert!(wait_until(|| every_member_has_a_ping(&host, 2)));
    assert!(wait_until(|| every_member_has_a_ping(&first, 2)));
    assert!(wait_until(|| every_member_has_a_ping(&second, 2)));
}

#[test]
fn the_host_removes_one_of_two_guests_and_the_session_goes_on() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let first = automata_core(&network, "Firsty");
    let second = automata_core(&network, "Secondy");
    let mut host_hook = FakeHook::attach(&host, MAIN_BUILD);
    let mut first_hook = FakeHook::attach(&first, MAIN_BUILD);
    let mut second_hook = FakeHook::attach(&second, MAIN_BUILD);
    host.set_rules(FREE_ROAM_RULES);
    let ticket = host.host().expect("ticket");
    first.join(&ticket).expect("first joins");
    second.join(&ticket).expect("second joins");
    let first_slot = slot_once_welcomed(&mut first_hook);
    let second_slot = slot_once_welcomed(&mut second_hook);
    assert!(wait_until(|| told_of_member(&mut first_hook, second_slot)));
    host.remove_member(second_slot);
    assert!(wait_until(
        || failure_text(&second).as_deref() == Some("The host removed you from the lobby.")
    ));
    let left = |m: &Message| matches!(m, Message::MemberLeft { slot } if *slot == second_slot);
    assert!(wait_until(|| host_hook.saw(left) && first_hook.saw(left)));
    assert!(wait_until(|| host.link_status().members.len() == 1));
    assert_eq!(
        host.link_status().partner().map(|p| p.slot),
        Some(first_slot)
    );
    assert!(wait_until(|| {
        first_hook.send(&Message::State(state_at(14, 88.0)));
        host_hook.saw(|m| matches!(m, Message::State(s) if s.position[0] == 88.0))
    }));
}

fn state_frame_with_extra_byte(sequence: u32, x: f32) -> (Vec<u8>, Vec<u8>) {
    let mut frame = encode_enveloped(&OWN_GAME_ENVELOPE, &Message::State(state_at(sequence, x)))
        .expect("encodes");
    let exact_body = frame[2..].to_vec();
    frame.push(0xAB);
    (frame, exact_body)
}

fn trailing_line_of(core: &LauncherCore) -> bool {
    saw_event(core, |event| {
        matches!(event, LauncherEvent::BuildsDiffer { text }
            if text.starts_with("relayed a game message with 1 trailing bytes")
                && text.contains("restart the launcher"))
    })
}

#[test]
fn a_state_with_a_byte_more_than_the_launcher_knows_reaches_the_partner_game_unchanged() {
    let mut rig = connected_rig();
    let (frame, exact_body) = state_frame_with_extra_byte(40, 77.0);
    let mut expected_body = exact_body;
    expected_body.push(0xAB);
    assert!(wait_until(|| {
        rig.host_hook.send_frame(&frame);
        rig.guest_hook.pump();
        rig.guest_hook
            .frames
            .iter()
            .any(|received| received[2..] == expected_body[..])
    }));
    assert!(
        rig.guest_hook
            .saw(|m| matches!(m, Message::State(s) if s.position[0] == 77.0))
    );
}

#[test]
fn the_first_message_with_extra_bytes_is_said_once_by_the_launcher_of_the_sending_game() {
    let mut rig = connected_rig();
    let (frame, _) = state_frame_with_extra_byte(50, 5.0);
    rig.host_hook.send_frame(&frame);
    assert!(trailing_line_of(&rig.host));
    let (again, _) = state_frame_with_extra_byte(51, 6.0);
    rig.host_hook.send_frame(&again);
    thread::sleep(Duration::from_millis(200));
    let repeats = rig
        .host
        .drain_events()
        .iter()
        .filter(|event| matches!(event, LauncherEvent::BuildsDiffer { .. }))
        .count();
    assert_eq!(repeats, 0);
    assert!(!rig.guest.drain_events().iter().any(
        |event| matches!(event, LauncherEvent::BuildsDiffer { text } if text.contains("trailing"))
    ));
}

#[test]
fn a_message_cut_short_is_still_refused_and_not_relayed() {
    let mut rig = connected_rig();
    let (frame, _) = state_frame_with_extra_byte(60, 1.0);
    rig.host_hook.send_frame(&frame[..frame.len() / 2]);
    assert!(saw_event(&rig.host, |event| matches!(
        event,
        LauncherEvent::MessageRefused { reason, .. } if reason.contains("ends early")
    )));
}

#[test]
fn a_game_on_another_protocol_is_named_once_with_both_numbers() {
    let mut rig = connected_rig();
    let hello = Message::Hello(Hello {
        version: PROTOCOL_VERSION + 1,
        build: MAIN_BUILD,
        name: "game".to_owned(),
    });
    rig.host_hook.send(&hello);
    rig.host_hook.send(&hello);
    let wanted = format!(
        "the game speaks protocol {}, this launcher {PROTOCOL_VERSION}: restart the launcher after a build update",
        PROTOCOL_VERSION + 1
    );
    assert!(saw_event(&rig.host, |event| matches!(
        event,
        LauncherEvent::BuildsDiffer { text } if *text == wanted
    )));
    thread::sleep(Duration::from_millis(200));
    assert!(
        !rig.host
            .drain_events()
            .iter()
            .any(|event| matches!(event, LauncherEvent::BuildsDiffer { text } if *text == wanted))
    );
}

#[test]
fn a_game_on_this_protocol_is_not_named() {
    let mut rig = connected_rig();
    rig.host_hook.send(&Message::Hello(Hello {
        version: PROTOCOL_VERSION,
        build: MAIN_BUILD,
        name: "game".to_owned(),
    }));
    thread::sleep(Duration::from_millis(300));
    assert!(
        !rig.host
            .drain_events()
            .iter()
            .any(|event| matches!(event, LauncherEvent::BuildsDiffer { .. }))
    );
}
