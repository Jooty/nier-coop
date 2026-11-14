use std::thread;
use std::time::{Duration, Instant};

use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_game::load_gate::{
    LocalLoadSignal, PARTNER_LOAD_SIGNAL_STALE_MS, PartnerLoad, PlaceRelation, StoryPlace,
};
use automata_game::partner::BodySnapshot;
use automata_hook::net::{
    AutomataNet, HeardLoadSignal, NetShared, NetSharedExt, PartnerWatch, ReadinessPacer,
    co_op_status_of, is_snapshot_without_body, link_not_started_status, local_load_signal,
    partner_load_of, player_load_of, readiness_of, set_local_load_signal, set_local_story_place,
    snapshot_without_body,
};
use automata_proto::{
    CharacterHello, GameReadiness, LoadSignal, Member, Message, PROTOCOL_VERSION, PathInfo,
    PlayerState, TeleportRequest, Welcome,
};
use nc_hook::net::{LinkPeer, NetState, NetThread, Player, RemotePlayer, net_clock_ms};
use nc_net::{LoopbackNetwork, Transport};

const WAIT_LIMIT: Duration = Duration::from_secs(20);
const MIRA_SLOT: u8 = 0;
const ODO_SLOT: u8 = 1;

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + WAIT_LIMIT;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(2));
    }
    false
}

fn welcome(slot: u8, host_name: &str) -> Message {
    Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot,
        host_name: host_name.to_owned(),
        rules: nc_wire::SessionRules {
            mode: nc_wire::STORY_MODE,
            max_players: 2,
            map: 0,
        },
    })
}

fn member(slot: u8, name: &str) -> Message {
    Message::Member(Member {
        slot,
        name: name.to_owned(),
    })
}

fn connected_pair() -> (NetThread<AutomataNet>, NetThread<AutomataNet>) {
    let network = LoopbackNetwork::new();
    let a_transport = network.endpoint();
    let mut b_transport = network.endpoint();
    let a_peer = a_transport.local_peer();
    let b_peer = b_transport.local_peer();
    let b_invite = b_transport.host().expect("b hosts");
    let a = NetThread::spawn(
        AutomataNet::new(Character::NineS, ExeVariant::Main),
        a_transport,
        LinkPeer {
            invite: Some(b_invite),
            peer: b_peer,
            port: 1,
        },
    )
    .expect("a starts");
    let b = NetThread::spawn(
        AutomataNet::new(Character::TwoB, ExeVariant::Compat),
        b_transport,
        LinkPeer {
            invite: None,
            peer: a_peer,
            port: 2,
        },
    )
    .expect("b starts");
    assert!(wait_until(|| {
        a.view().state == NetState::LauncherLinked && b.view().state == NetState::LauncherLinked
    }));
    a.send_to_launcher(welcome(ODO_SLOT, "Mira"));
    a.send_to_launcher(member(MIRA_SLOT, "Mira"));
    b.send_to_launcher(welcome(MIRA_SLOT, "Odo"));
    b.send_to_launcher(member(ODO_SLOT, "Odo"));
    assert!(wait_until(|| {
        a.view().state == NetState::Connected
            && b.view().state == NetState::Connected
            && a.view().partner().is_some()
            && b.view().partner().is_some()
    }));
    (a, b)
}

fn snapshot() -> BodySnapshot {
    BodySnapshot {
        held: 0b100,
        pressed: 0b001,
        speed: 1.0,
        world_facing: 1.5,
        position: [10.0, 2.0, -30.0],
        body_facing: 0.5,
        state: 0x44,
        last_start: None,
        health: 4200,
        max_health: 5000,
        free: true,
        phase_id: 0x100,
        paused: false,
        jump_pending: false,
        pod_aim: None,
        downed: Default::default(),
    }
}

#[test]
fn a_published_snapshot_arrives_as_the_partners_state_and_input() {
    let (a, b) = connected_pair();
    assert!(a.wants_snapshots());
    a.publish(snapshot());
    assert!(wait_until(|| b.view().partner_state().is_some()));
    let remote = b
        .view()
        .partner_state()
        .cloned()
        .expect("the state arrived");
    assert_eq!(remote.state.to_snapshot(), snapshot());
    assert_eq!(remote.name, "Mira");
    let feed = b.update(|shared| shared.extras.remote_inputs.take_feed(MIRA_SLOT));
    assert_eq!(feed.held, 0b100);
    assert_eq!(feed.pressed, 0b101);
    assert_eq!(
        b.update(|shared| shared.extras.remote_inputs.take_feed(MIRA_SLOT).pressed),
        0
    );
}

#[test]
fn the_character_message_reaches_the_other_side_reliably() {
    let (a, b) = connected_pair();
    a.send_relayed(Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    }));
    assert!(wait_until(|| {
        b.view().partner_watch().character == Some(Character::A2)
    }));
    assert_eq!(a.view().partner_watch().character, None);
}

#[test]
fn the_other_game_counts_as_heard_after_its_hello_or_its_first_state() {
    let (a, b) = connected_pair();
    assert!(!a.view().partner_watch().heard);
    assert!(!b.view().partner_watch().heard);
    a.send_relayed(Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    }));
    assert!(wait_until(|| b.view().partner_watch().heard));
    b.publish(snapshot());
    assert!(wait_until(|| a.view().partner_watch().heard));
}

#[test]
fn a_teleport_request_waits_in_the_inbox_for_the_game_thread() {
    let (a, b) = connected_pair();
    let request = Message::Teleport(TeleportRequest::FollowJump { point: 8 });
    a.send_relayed(request.clone());
    let mut taken = Vec::new();
    assert!(wait_until(|| {
        taken = b.take_relayed(|message| matches!(message, Message::Teleport(_)));
        !taken.is_empty()
    }));
    assert_eq!(taken, vec![request]);
    assert!(
        a.take_relayed(|message| matches!(message, Message::Teleport(_)))
            .is_empty()
    );
}

#[test]
fn the_path_report_sets_the_ping() {
    let (a, b) = connected_pair();
    a.send_to_launcher(Message::PathInfo(PathInfo {
        rtt_micros: 42_000,
        direct: true,
    }));
    assert!(wait_until(|| b.view().rtt.is_some()));
    let status = co_op_status_of(&b.view(), Character::TwoB);
    assert_eq!(status.ping_ms, Some(42));
    assert_eq!(status.link, "connected");
}

fn shared(state: NetState) -> NetShared {
    let mut shared = NetShared {
        state,
        ..NetShared::default()
    };
    shared
        .players
        .insert(ODO_SLOT, Player::joined(ODO_SLOT, "Mira".to_owned()));
    shared
}

fn partner_of(
    shared: &mut NetShared,
) -> &mut Player<PlayerState, automata_hook::net::PartnerWatch> {
    shared
        .players
        .get_mut(&ODO_SLOT)
        .expect("the partner is in the session")
}

#[test]
fn the_co_op_section_is_empty_until_a_partner_is_connected() {
    let status = co_op_status_of(&shared(NetState::LauncherLinked), Character::NineS);
    assert_eq!(status.link, "launcher linked, no other player");
    assert!(status.players.is_empty());
    assert_eq!(status.ping_ms, None);
    let off = link_not_started_status();
    assert!(off.link.starts_with("not started"));
    assert!(off.players.is_empty());
}

#[test]
fn a_connected_partner_shows_the_link_setting_until_it_names_its_character() {
    let mut connected = shared(NetState::Connected);
    let status = co_op_status_of(&connected, Character::NineS);
    assert_eq!(status.players.len(), 1);
    assert_eq!(status.players[0].name, "Mira");
    assert!(status.players[0].detail.starts_with("9S (the link setting"));
    partner_of(&mut connected).watch.character = Some(Character::A2);
    partner_of(&mut connected).latest = Some(RemotePlayer {
        name: "Mira".to_owned(),
        slot: 1,
        state: PlayerState::from_snapshot(&snapshot(), 1),
        received_at: Instant::now(),
    });
    let status = co_op_status_of(&connected, Character::NineS);
    assert_eq!(status.players[0].detail, "A2, health 4200 of 5000");
    if let Some(remote) = partner_of(&mut connected).latest.as_mut() {
        remote.state.load.without_body = true;
    }
    let status = co_op_status_of(&connected, Character::NineS);
    assert_eq!(status.players[0].detail, "A2");
}

#[test]
fn the_launcher_hears_in_play_only_once_the_start_reached_the_world() {
    assert_eq!(readiness_of(false), GameReadiness::Loading);
    assert_eq!(readiness_of(true), GameReadiness::InPlay);
}

#[test]
fn readiness_is_told_at_once_on_a_change_and_again_every_second() {
    let mut pacer = ReadinessPacer::default();
    assert!(pacer.is_due(0, GameReadiness::Loading));
    assert!(pacer.is_new(GameReadiness::Loading));
    pacer.mark_sent(0, GameReadiness::Loading);
    assert!(!pacer.is_due(500, GameReadiness::Loading));
    assert!(pacer.is_due(1_000, GameReadiness::Loading));
    assert!(!pacer.is_new(GameReadiness::Loading));
    assert!(pacer.is_due(10, GameReadiness::InPlay));
    assert!(pacer.is_new(GameReadiness::InPlay));
    pacer.mark_sent(10, GameReadiness::InPlay);
    assert!(pacer.is_due(20, GameReadiness::Loading));
    pacer.forget();
    assert!(pacer.is_due(30, GameReadiness::InPlay));
    assert!(pacer.is_new(GameReadiness::InPlay));
}

#[test]
fn a_state_without_a_body_tells_of_the_loading_screen_and_moves_no_partner() {
    let (a, b) = connected_pair();
    set_local_load_signal(LocalLoadSignal {
        without_body: false,
        waiting: true,
        release_at_ms: Some(net_clock_ms() + 500),
    });
    a.publish(snapshot_without_body());
    assert!(wait_until(|| b.view().partner_watch().load.is_some()));
    set_local_load_signal(LocalLoadSignal::default());
    let view = b.view();
    let heard = view.partner_watch().load.expect("the word arrived");
    assert!(heard.signal.without_body);
    assert!(heard.signal.waiting);
    let release_in_ms = heard
        .signal
        .release_in_ms
        .expect("a release moment was named");
    assert!((0..=500).contains(&release_in_ms));
    assert!(view.partner_watch().heard);
    assert!(view.partner_state().is_none());
    assert_eq!(
        b.update(|shared| shared.extras.remote_inputs.take_feed(MIRA_SLOT).held),
        0
    );
    assert_eq!(local_load_signal(), LoadSignal::default());
}

#[test]
fn a_state_with_a_body_says_the_partner_is_past_its_loading_screen() {
    let (a, b) = connected_pair();
    a.publish(snapshot());
    assert!(wait_until(|| b.view().partner_state().is_some()));
    let heard = b.view().partner_watch().load.expect("the word arrived");
    assert!(!heard.signal.without_body);
    assert!(!is_snapshot_without_body(&snapshot()));
    assert!(is_snapshot_without_body(&snapshot_without_body()));
}

#[test]
fn the_partners_word_counts_as_waiting_or_in_play_only_while_it_is_fresh() {
    let mut linked = shared(NetState::Connected);
    assert_eq!(
        partner_load_of(&linked, 5_000),
        PartnerLoad {
            never_heard: true,
            ..PartnerLoad::default()
        }
    );
    linked.rtt = Some(Duration::from_millis(80));
    partner_of(&mut linked).watch.load = Some(HeardLoadSignal {
        signal: LoadSignal {
            without_body: true,
            waiting: true,
            release_in_ms: None,
        },
        heard_at_ms: 5_000,
    });
    let fresh = partner_load_of(&linked, 5_000 + PARTNER_LOAD_SIGNAL_STALE_MS);
    assert!(fresh.waiting);
    assert!(!fresh.in_play);
    assert_eq!(fresh.round_trip_ms, Some(80));
    let stale = partner_load_of(&linked, 5_001 + PARTNER_LOAD_SIGNAL_STALE_MS);
    assert!(!stale.waiting);
    partner_of(&mut linked).watch.load = Some(HeardLoadSignal {
        signal: LoadSignal {
            without_body: false,
            waiting: false,
            release_in_ms: Some(77),
        },
        heard_at_ms: 6_000,
    });
    assert!(partner_load_of(&linked, 6_100).in_play);
    let long_after = partner_load_of(&linked, 600_000);
    assert!(!long_after.in_play);
    // A named release time stays known however old the message is.
    assert_eq!(long_after.release_in_ms, Some(77));
    assert_eq!(long_after.heard_at_ms, 6_000);
}

#[test]
fn a_partner_who_leaves_takes_their_character_and_state_along() {
    let (a, b) = connected_pair();
    a.send_relayed(Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    }));
    a.publish(snapshot());
    assert!(wait_until(|| {
        let view = b.view();
        view.partner_watch().character == Some(Character::A2) && view.partner_state().is_some()
    }));
    a.send_to_launcher(Message::MemberLeft { slot: MIRA_SLOT });
    assert!(wait_until(|| b.view().partner().is_none()));
    let view = b.view();
    assert_eq!(view.state, NetState::LauncherLinked);
    assert_eq!(view.partner_watch(), Default::default());
    assert!(view.partner_state().is_none());
    assert_eq!(co_op_status_of(&view, Character::TwoB).players, Vec::new());
}

#[test]
fn a_leaving_player_resets_only_its_own_extras() {
    use automata_game::enemy_lead::EnemyLead;
    use automata_hook::net::AutomataLinkState;

    let heard = BodySnapshot {
        held: 1,
        pressed: 1,
        speed: 2.0,
        ..BodySnapshot::default()
    };
    let lead = EnemyLead { poses: Vec::new() };
    let mut extras = AutomataLinkState::default();
    extras.remote_inputs.receive(2, &heard);
    extras.remote_inputs.receive(3, &heard);
    extras.enemy_leads.push_back((2, lead.clone()));
    extras.enemy_leads.push_back((3, lead));
    extras.player_left(2);
    assert!(!extras.remote_inputs.holds_input_of(2));
    assert!(extras.remote_inputs.holds_input_of(3));
    assert_eq!(extras.remote_inputs.take_feed(3).held, 1);
    assert_eq!(extras.enemy_leads.len(), 1);
    assert_eq!(extras.enemy_leads[0].0, 3);
    extras.remote_inputs.receive(3, &heard);
    extras.player_joined(4);
    assert!(extras.remote_inputs.holds_input_of(3));
    extras.player_joined(3);
    assert!(!extras.remote_inputs.holds_input_of(3));
    assert!(extras.enemy_leads.is_empty());
}

#[test]
fn a_players_watch_and_load_are_read_by_slot() {
    let (a, b) = connected_pair();
    a.publish(snapshot());
    assert!(wait_until(|| b.view().player_watch(MIRA_SLOT).heard));
    let view = b.view();
    assert!(view.player_watch(MIRA_SLOT).load.is_some());
    assert_eq!(view.player_watch(ODO_SLOT), Default::default());
    assert!(!player_load_of(&view, ODO_SLOT, net_clock_ms()).in_play);
    assert!(player_load_of(&view, MIRA_SLOT, net_clock_ms()).in_play);
}

#[test]
fn relayed_messages_keep_going_to_a_silent_player() {
    use nc_hook::net::NetGame;

    let silent = PartnerWatch {
        heard: true,
        load: Some(HeardLoadSignal {
            signal: local_load_signal(),
            heard_at_ms: 0,
        }),
        ..PartnerWatch::default()
    };
    assert!(<AutomataNet as NetGame>::player_in_play(&silent));
    assert!(<AutomataNet as NetGame>::player_in_play(
        &PartnerWatch::default()
    ));
}

#[test]
fn the_story_place_rides_every_state_and_is_compared_with_this_games_own() {
    let here = StoryPlace::new(0x100, "00_30_A_Place");
    let there = StoryPlace::new(0x200, "10_00_B_Place");
    set_local_story_place(here);
    let (a, b) = connected_pair();
    a.publish(snapshot());
    assert!(wait_until(|| b.view().partner_watch().place == here));
    set_local_story_place(there);
    a.publish(snapshot_without_body());
    assert!(wait_until(|| {
        let watch = b.view().partner_watch();
        watch.place == there && watch.load.is_some_and(|heard| heard.signal.without_body)
    }));

    let mut linked = shared(NetState::Connected);
    set_local_story_place(here);
    partner_of(&mut linked).watch.place = here;
    assert_eq!(partner_load_of(&linked, 0).place, PlaceRelation::Same);
    partner_of(&mut linked).watch.place = there;
    assert_eq!(partner_load_of(&linked, 0).place, PlaceRelation::Other);
    partner_of(&mut linked).watch.place = StoryPlace::default();
    assert_eq!(partner_load_of(&linked, 0).place, PlaceRelation::Unknown);
    set_local_story_place(StoryPlace::default());
    partner_of(&mut linked).watch.place = there;
    assert_eq!(partner_load_of(&linked, 0).place, PlaceRelation::Unknown);
}
