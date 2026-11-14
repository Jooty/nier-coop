use replicant_game::rule_rooms::{
    GuestBreakVerdict, RULE_ROOMS, RuleAct, RulePlayer, RulePop, RulePopFacts, RuleRoomCall,
    RuleRoomWatch, RuleState, armed_line, guest_break_line, kept_message_line, rule_room_call,
};
use replicant_game::story_calls::{StoryCallKind, name_hash};
use replicant_game::story_relay::relay_kind;

const TEMPLE: &str = "A_DESERT_TEMPLE_01";

fn facts(function: &str, player: RulePlayer, own_fire: bool) -> RulePopFacts<'_> {
    RulePopFacts {
        pack: TEMPLE,
        function,
        player,
        shared: true,
        own_fire,
    }
}

fn armed(player_watch: &mut RuleRoomWatch, handler: &str) {
    assert!(player_watch.registered(TEMPLE, handler).is_some());
}

#[test]
fn the_temple_has_eight_rule_rooms_each_with_its_clear_flag_and_a_hit_rule() {
    let rooms: Vec<_> = RULE_ROOMS
        .iter()
        .filter(|room| room.pack == TEMPLE)
        .collect();
    assert_eq!(rooms.len(), 8);
    for (index, room) in rooms.iter().enumerate() {
        assert_eq!(usize::from(room.room), index + 1);
        assert_eq!(room.clear_flag, 448 + u32::try_from(index).unwrap());
        assert!(
            room.breaks
                .iter()
                .any(|rule_break| rule_break.act == RuleAct::HitByTheRoom)
        );
    }
}

#[test]
fn the_first_room_breaks_on_a_jump_and_its_calls_are_known() {
    let (room, call) = rule_room_call(TEMPLE, "Jump_has_Done").unwrap();
    assert_eq!((room.room, call), (1, RuleRoomCall::Break(RuleAct::Jump)));
    assert_eq!(
        rule_room_call(TEMPLE, "Start_Law_Game_01").map(|(room, call)| (room.room, call)),
        Some((1, RuleRoomCall::Start))
    );
    assert_eq!(
        rule_room_call(TEMPLE, "Restart_Law_Game_01").map(|(room, call)| (room.room, call)),
        Some((1, RuleRoomCall::Restart))
    );
    assert_eq!(
        rule_room_call(TEMPLE, "Clear_Law_Game_01").map(|(room, call)| (room.room, call)),
        Some((1, RuleRoomCall::Clear))
    );
    assert!(rule_room_call("A_DESERT_TEMPLE_02", "Jump_has_Done").is_none());
    assert!(room.has_break_message(219));
    assert!(room.has_break_message(422));
    assert!(!room.has_break_message(216));
}

#[test]
fn room_starts_and_move_and_timer_breaks_are_the_hosts_story_calls() {
    let pack = name_hash(TEMPLE);
    for function in [
        "Start_Law_Game_01",
        "Start_Law_Game_08",
        "Run_has_Done",
        "Move_has_Done",
        "Time_has_Done",
        "Restart_Law_Game_01",
        "Clear_Law_Game_01",
    ] {
        assert_eq!(
            relay_kind(pack, name_hash(function)),
            Some(StoryCallKind::Story),
            "{function}"
        );
    }
    assert_eq!(
        relay_kind(pack, name_hash("Jump_has_Done")),
        Some(StoryCallKind::Local)
    );
}

#[test]
fn the_arming_line_names_the_rooms_breaks_and_inhibits() {
    let mut watch = RuleRoomWatch::new();
    let room = watch.registered(TEMPLE, "Jump_has_Done").unwrap();
    assert_eq!(
        armed_line(room, RulePlayer::Guest, "Jump_has_Done"),
        "rule room: room 1 of A_DESERT_TEMPLE_01 armed on the guest side by Jump_has_Done (breaks: jump Jump_has_Done, hit Hit_has_Done01; inhibits: weapon, magic, jump, lock-on)"
    );
    assert_eq!(watch.registered(TEMPLE, "Hit_has_Done01"), None);
    assert_eq!(watch.registered(TEMPLE, "LIB_GameOver"), None);
}

#[test]
fn the_guests_own_jump_is_asked_of_the_host_once_and_not_run_there() {
    // 02:49:30.324 in run-1114c: the guest's own Jump_has_Done was held and never asked.
    let mut guest = RuleRoomWatch::new();
    armed(&mut guest, "Jump_has_Done");
    let own_jump = guest.pop(&facts("Jump_has_Done", RulePlayer::Guest, true));
    assert!(matches!(
        own_jump,
        RulePop::GuestAsksTheHost {
            act: RuleAct::Jump,
            ..
        }
    ));
    assert!(own_jump.drops());
    assert_eq!(
        own_jump.log_line("Jump_has_Done").unwrap(),
        "rule room: room 1 of A_DESERT_TEMPLE_01 broken by the guest (jump, Jump_has_Done), not run here, asked of the host"
    );
    let second = guest.pop(&facts("Jump_has_Done", RulePlayer::Guest, true));
    assert!(matches!(
        second,
        RulePop::DropAlreadyBroken {
            player: RulePlayer::Guest,
            ..
        }
    ));
    let hosts_copy = guest.pop(&facts("Jump_has_Done", RulePlayer::Guest, false));
    assert!(matches!(hosts_copy, RulePop::HostsBreakRunsHere { .. }));
    assert!(!hosts_copy.drops());
    armed(&mut guest, "Jump_has_Done");
    assert!(matches!(
        guest.pop(&facts("Hit_has_Done01", RulePlayer::Guest, true)),
        RulePop::GuestAsksTheHost {
            act: RuleAct::HitByTheRoom,
            ..
        }
    ));
}

#[test]
fn the_host_runs_the_guests_break_once_and_drops_its_own_late_copy() {
    let mut host = RuleRoomWatch::new();
    armed(&mut host, "Jump_has_Done");
    let verdict = host.accept_guest_break(TEMPLE, "Jump_has_Done");
    assert!(matches!(
        verdict,
        GuestBreakVerdict::RunsOnce {
            act: RuleAct::Jump,
            ..
        }
    ));
    assert_eq!(
        guest_break_line(&verdict, "Jump_has_Done"),
        "rule room: room 1 of A_DESERT_TEMPLE_01: the guest broke the rule (jump, Jump_has_Done), it runs here once"
    );
    let again = host.accept_guest_break(TEMPLE, "Hit_has_Done01");
    assert!(matches!(
        again,
        GuestBreakVerdict::NotArmed {
            state: Some(RuleState::AskedOfTheHost),
            ..
        }
    ));
    assert!(
        host.pop(&facts("Jump_has_Done", RulePlayer::Host, true))
            .drops()
    );
    assert!(matches!(
        host.pop(&facts("Jump_has_Done", RulePlayer::Host, false)),
        RulePop::GuestsBreakRunsHere { .. }
    ));
    assert!(matches!(
        host.accept_guest_break(TEMPLE, "Jump_has_Done"),
        GuestBreakVerdict::NotArmed {
            state: Some(RuleState::Broken),
            ..
        }
    ));
}

#[test]
fn the_hosts_own_jump_breaks_once_and_a_late_guest_ask_is_refused() {
    // 02:49:15.607 in run-1114c: the host's jump reset both; 02:49:22.2 a second host jump ran the
    // guest's handler twice (the mirrored 219 and the relayed copy).
    let mut host = RuleRoomWatch::new();
    armed(&mut host, "Jump_has_Done");
    assert_eq!(
        host.keeps_message_here(TEMPLE, 219).map(|room| room.room),
        Some(1)
    );
    assert_eq!(host.keeps_message_here(TEMPLE, 216), None);
    let jump = host.pop(&facts("Jump_has_Done", RulePlayer::Host, true));
    assert!(matches!(jump, RulePop::HostBreaks { .. }));
    assert!(!jump.drops());
    assert_eq!(host.keeps_message_here(TEMPLE, 219), None);
    assert!(matches!(
        host.accept_guest_break(TEMPLE, "Jump_has_Done"),
        GuestBreakVerdict::NotArmed { .. }
    ));
    assert!(
        host.pop(&facts("Hit_has_Done01", RulePlayer::Host, true))
            .drops()
    );
    assert!(host.registered(TEMPLE, "Jump_has_Done").is_some());
    assert!(matches!(
        host.pop(&facts("Jump_has_Done", RulePlayer::Host, true)),
        RulePop::HostBreaks { .. }
    ));
}

#[test]
fn the_kept_mirror_line_says_why() {
    let mut host = RuleRoomWatch::new();
    let room = host.registered(TEMPLE, "Jump_has_Done").unwrap();
    assert_eq!(
        kept_message_line(room, 219),
        "boss message: host raised 219, kept here: it breaks the rule of room 1 of A_DESERT_TEMPLE_01, the host's run reaches the guest"
    );
}

#[test]
fn a_room_clears_from_the_host_and_the_guests_own_clear_is_dropped() {
    let mut guest = RuleRoomWatch::new();
    armed(&mut guest, "Jump_has_Done");
    assert!(
        guest
            .pop(&facts("Clear_Law_Game_01", RulePlayer::Guest, true))
            .drops()
    );
    let hosts_clear = guest.pop(&facts("Clear_Law_Game_01", RulePlayer::Guest, false));
    assert_eq!(
        hosts_clear.log_line("Clear_Law_Game_01").unwrap(),
        "rule room: room 1 of A_DESERT_TEMPLE_01 cleared (Clear_Law_Game_01 on the guest side, flag 448)"
    );
    let mut host = RuleRoomWatch::new();
    armed(&mut host, "Jump_has_Done");
    assert!(matches!(
        host.pop(&facts("Clear_Law_Game_01", RulePlayer::Host, true)),
        RulePop::Cleared {
            player: RulePlayer::Host,
            ..
        }
    ));
    assert_eq!(host.keeps_message_here(TEMPLE, 219), None);
}

#[test]
fn the_guests_own_entry_runs_there_and_its_own_clock_never_breaks_the_room() {
    let mut guest = RuleRoomWatch::new();
    let start = guest.pop(&facts("Start_Law_Game_01", RulePlayer::Guest, true));
    assert!(start.skips_the_story_relay());
    assert!(!start.drops());
    assert_eq!(
        guest.pop(&facts("Start_Law_Game_01", RulePlayer::Guest, false)),
        RulePop::PassesOn
    );
    armed(&mut guest, "Hit_has_Done08");
    assert!(matches!(
        guest.pop(&facts("Time_has_Done", RulePlayer::Guest, true)),
        RulePop::DropGuestTimeUp { .. }
    ));
    assert!(matches!(
        guest.accept_guest_break(TEMPLE, "Time_has_Done"),
        GuestBreakVerdict::NotARuleBreak
    ));
}

#[test]
fn a_break_without_the_partner_in_the_phase_runs_and_resets_only_here() {
    let mut guest = RuleRoomWatch::new();
    armed(&mut guest, "Jump_has_Done");
    let alone = guest.pop(&RulePopFacts {
        shared: false,
        ..facts("Jump_has_Done", RulePlayer::Guest, true)
    });
    assert!(matches!(alone, RulePop::BrokenAlone { .. }));
    assert!(!alone.drops());
    assert_eq!(
        guest.pop(&facts("Law_Door01_Open", RulePlayer::Guest, true)),
        RulePop::NotARuleRoomCall
    );
}
