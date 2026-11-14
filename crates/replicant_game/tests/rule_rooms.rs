use replicant_game::rule_rooms::{
    BreakerReturn, RETURN_FADE_OUT_MS, RETURN_SPOTS, RULE_ROOMS, RoomClock, RoomResetVerdict,
    RuleAct, RulePlayer, RulePop, RulePopFacts, RuleRoomCall, RuleRoomWatch, RuleState, TakeIn,
    armed_line, guest_break_line, hosts_reset_line, kept_message_line, return_spot_of,
    returned_line, rule_room_call,
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
        queued_by_the_mod: !own_fire,
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
        rule_room_call(TEMPLE, "Set_Law_Game_01").map(|(room, call)| (room.room, call)),
        Some((1, RuleRoomCall::Arm))
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
fn only_a_room_start_is_the_hosts_story_call_and_a_break_belongs_to_its_own_game() {
    let pack = name_hash(TEMPLE);
    for function in [
        "Start_Law_Game_01",
        "Start_Law_Game_08",
        "Restart_Law_Game_01",
        "Clear_Law_Game_01",
    ] {
        assert_eq!(
            relay_kind(pack, name_hash(function)),
            Some(StoryCallKind::Story),
            "{function}"
        );
    }
    for function in [
        "Run_has_Done",
        "Move_has_Done",
        "Time_has_Done",
        "Jump_has_Done",
    ] {
        assert_ne!(
            relay_kind(pack, name_hash(function)),
            Some(StoryCallKind::Story),
            "{function}"
        );
    }
}

#[test]
fn every_room_puts_the_breaker_back_where_its_restart_placed_the_player() {
    let restart_spots = [
        (1, [20.0, 12.0, 74.0], 90.0),
        (2, [-23.0, 12.0, 74.0], -90.0),
        (3, [-55.0, 8.0, 42.13], 90.0),
        (4, [22.0, 8.0, 42.0], 90.0),
        (5, [20.0, 4.0, 6.0], 90.0),
        (6, [-22.0, 4.0, 6.0], -90.0),
        (7, [-23.85, 0.0, -30.0], 90.0),
        (8, [19.15, -0.05, -22.0], 90.0),
    ];
    assert_eq!(RETURN_SPOTS.len(), restart_spots.len());
    for (room, position, heading) in restart_spots {
        let rule_room = RULE_ROOMS.iter().find(|row| row.room == room).unwrap();
        let spot = return_spot_of(rule_room).unwrap();
        assert_eq!((spot.position, spot.heading_degrees), (position, heading));
    }
}

#[test]
fn every_room_arms_its_breaks_again_without_the_restart_or_the_cube_placement() {
    for room in &RULE_ROOMS {
        assert!(!room.arms.is_empty(), "room {}", room.room);
        for arm in room.arms {
            assert!(arm.starts_with("Set_Law_Game_"), "{arm}");
            assert!(!arm.contains("Box"), "{arm}");
        }
        assert!(room.removed_before_arming.contains(&5101));
        assert!(room.removed_before_arming.contains(&5102));
    }
    let second = RULE_ROOMS.iter().find(|room| room.room == 2).unwrap();
    assert_eq!(second.arms, ["Set_Law_Game_02", "Set_Law_Game_02_02"]);
    assert!(second.removed_before_arming.contains(&5104));
    let seventh = RULE_ROOMS.iter().find(|room| room.room == 7).unwrap();
    assert!(!seventh.removed_before_arming.contains(&5103));
    let eighth = RULE_ROOMS.iter().find(|room| room.room == 8).unwrap();
    assert_eq!(eighth.arms, ["Set_Law_Game_08"]);
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
fn the_guests_jump_sends_back_only_the_guest_and_never_runs_the_hosts_copy() {
    // 13:19:12.866 in run-1114h: the guest's own game posted 219; the old flow asked the host,
    // which ran the handler and reset both players and the cubes.
    let mut guest = RuleRoomWatch::new();
    armed(&mut guest, "Jump_has_Done");
    let own_jump = guest.pop(&facts("Jump_has_Done", RulePlayer::Guest, true));
    assert!(matches!(
        own_jump,
        RulePop::BreakerSentBack {
            act: RuleAct::Jump,
            player: RulePlayer::Guest,
            ..
        }
    ));
    assert!(own_jump.drops());
    assert_eq!(
        own_jump.log_line("Jump_has_Done").unwrap(),
        "rule room: room 1 of A_DESERT_TEMPLE_01: the guest broke the rule (jump, Jump_has_Done), only the guest goes back to the start, room not reset"
    );
    let second = guest.pop(&facts("Hit_has_Done01", RulePlayer::Guest, true));
    assert!(matches!(second, RulePop::DropAlreadyBroken { .. }));
    assert!(second.drops());
    let hosts_copy = guest.pop(&facts("Jump_has_Done", RulePlayer::Guest, false));
    assert!(matches!(hosts_copy, RulePop::DropPartnersBreak { .. }));
    assert!(hosts_copy.drops());
    armed(&mut guest, "Jump_has_Done");
    assert!(matches!(
        guest.pop(&facts("Hit_has_Done01", RulePlayer::Guest, true)),
        RulePop::BreakerSentBack {
            act: RuleAct::HitByTheRoom,
            ..
        }
    ));
}

#[test]
fn the_hosts_jump_sends_back_only_the_host_and_stays_out_of_the_mirror_until_the_room_clears() {
    // 13:19:05.410 in run-1114h: host raised 219; the old flow ran the handler on both games.
    let mut host = RuleRoomWatch::new();
    armed(&mut host, "Jump_has_Done");
    assert_eq!(
        host.keeps_message_here(TEMPLE, 219).map(|room| room.room),
        Some(1)
    );
    assert_eq!(host.keeps_message_here(TEMPLE, 216), None);
    let jump = host.pop(&facts("Jump_has_Done", RulePlayer::Host, true));
    assert!(matches!(
        jump,
        RulePop::BreakerSentBack {
            player: RulePlayer::Host,
            ..
        }
    ));
    assert!(jump.drops());
    assert_eq!(host.state_of(jump_room(&jump)), Some(RuleState::Broken));
    assert_eq!(
        host.keeps_message_here(TEMPLE, 219).map(|room| room.room),
        Some(1)
    );
    assert!(host.registered(TEMPLE, "Jump_has_Done").is_some());
    assert!(matches!(
        host.pop(&facts("Jump_has_Done", RulePlayer::Host, true)),
        RulePop::BreakerSentBack { .. }
    ));
    assert!(matches!(
        host.pop(&facts("Clear_Law_Game_01", RulePlayer::Host, true)),
        RulePop::Cleared { .. }
    ));
    assert_eq!(host.keeps_message_here(TEMPLE, 219), None);
}

fn jump_room(pop: &RulePop) -> &'static replicant_game::rule_rooms::RuleRoom {
    let RulePop::BreakerSentBack { room, .. } = *pop else {
        panic!("not a break");
    };
    room
}

#[test]
fn the_kept_mirror_line_says_why() {
    let mut host = RuleRoomWatch::new();
    let room = host.registered(TEMPLE, "Jump_has_Done").unwrap();
    assert_eq!(
        kept_message_line(room, 219),
        "boss message: host raised 219, kept here: it breaks the rule of room 1 of A_DESERT_TEMPLE_01, it only sends the host back"
    );
}

#[test]
fn a_restart_never_runs_while_the_partner_is_in_the_room() {
    let mut guest = RuleRoomWatch::new();
    armed(&mut guest, "Jump_has_Done");
    let own = guest.pop(&facts("Restart_Law_Game_01", RulePlayer::Guest, true));
    assert!(matches!(own, RulePop::DropRestart { room } if room.room == 1));
    assert!(own.drops());
    assert_eq!(
        own.log_line("Restart_Law_Game_01").unwrap(),
        "rule room: room 1 of A_DESERT_TEMPLE_01: Restart_Law_Game_01 not run, a break never resets the room"
    );
    let streamed = RulePopFacts {
        queued_by_the_mod: false,
        ..facts("Restart_Law_Game_03", RulePlayer::Guest, false)
    };
    assert!(guest.pop(&streamed).drops());
    let mut host = RuleRoomWatch::new();
    assert!(
        host.pop(&facts("Restart_Law_Game_01", RulePlayer::Host, true))
            .drops()
    );
}

#[test]
fn the_move_and_run_breaks_send_back_the_game_they_fire_in() {
    let mut guest = RuleRoomWatch::new();
    armed(&mut guest, "Dash_has_Done03");
    assert!(matches!(
        guest.pop(&facts("Run_has_Done", RulePlayer::Guest, true)),
        RulePop::BreakerSentBack {
            act: RuleAct::Run,
            ..
        }
    ));
}

#[test]
fn the_arming_calls_run_only_when_the_mod_queued_them() {
    let mut watch = RuleRoomWatch::new();
    for function in ["Set_Law_Game_02", "Set_Law_Game_02_02"] {
        let arm = watch.pop(&facts(function, RulePlayer::Guest, false));
        assert!(matches!(arm, RulePop::ArmRunsHere { room } if room.room == 2));
        assert!(!arm.drops());
        assert!(arm.runs_here_outside_the_streams());
    }
    assert_eq!(
        watch.pop(&facts("Set_Law_Game_02", RulePlayer::Guest, true)),
        RulePop::PassesOn
    );
}

#[test]
fn the_return_line_names_the_player_and_the_spot_and_says_the_room_stays() {
    let room = &RULE_ROOMS[0];
    let spot = return_spot_of(room).unwrap();
    assert_eq!(
        returned_line(room, RulePlayer::Guest, spot),
        "rule room: room 1 of A_DESERT_TEMPLE_01: the guest was sent back to (20.0, 12.0, 74.0), room not reset: the partner and every object stay where they are"
    );
}

#[test]
fn one_return_runs_at_a_time_and_waits_for_the_fade_to_cover_the_screen() {
    let room = &RULE_ROOMS[0];
    let mut breaker = BreakerReturn::new();
    assert!(!breaker.is_returning());
    assert!(breaker.begin(room, RulePlayer::Guest, 1_000));
    assert!(!breaker.begin(&RULE_ROOMS[1], RulePlayer::Host, 1_100));
    assert!(breaker.is_returning());
    assert_eq!(breaker.take_if_due(1_000 + RETURN_FADE_OUT_MS - 1), None);
    assert_eq!(
        breaker.take_if_due(1_000 + RETURN_FADE_OUT_MS),
        Some((room, RulePlayer::Guest))
    );
    assert!(!breaker.is_returning());
    assert!(breaker.begin(&RULE_ROOMS[1], RulePlayer::Host, 5_000));
}

#[test]
fn the_guests_own_entry_runs_there_and_the_hosts_start_is_not_asked() {
    let mut guest = RuleRoomWatch::new();
    let start = guest.pop(&facts("Start_Law_Game_01", RulePlayer::Guest, true));
    assert!(start.skips_the_story_relay());
    assert!(!start.drops());
    assert_eq!(
        guest.pop(&RulePopFacts {
            queued_by_the_mod: false,
            ..facts("Start_Law_Game_01", RulePlayer::Guest, false)
        }),
        RulePop::PassesOn
    );
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
        guest.pop(&RulePopFacts {
            shared: false,
            ..facts("Restart_Law_Game_01", RulePlayer::Guest, true)
        }),
        RulePop::PassesOn
    );
    assert_eq!(
        guest.pop(&facts("Law_Door01_Open", RulePlayer::Guest, true)),
        RulePop::NotARuleRoomCall
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
}

#[test]
fn the_hosts_start_takes_a_guest_left_outside_in_beside_the_host() {
    // 12:45:22.453 guest: the host's Start_Law_Game_01 ran there with the guest 28 units out, behind door 602.
    let mut guest = RuleRoomWatch::new();
    let start = guest.pop(&facts("Start_Law_Game_01", RulePlayer::Guest, false));
    let RulePop::GuestTakenIn { room } = start else {
        panic!("the host's start takes the guest in, got {start:?}");
    };
    assert_eq!(room.room, 1);
    assert!(!start.drops());
    assert!(!start.skips_the_story_relay());
    let host_at = [20.4, 12.0, 74.27];
    let take_in = TakeIn::judge(Some([-7.4, 12.0, 77.8]), Some(host_at));
    let TakeIn::MoveBeside { host_at: to, units } = take_in else {
        panic!("a guest outside is moved, got {take_in:?}");
    };
    assert_eq!(to, host_at);
    assert!(units.is_some_and(|units| (27.0..29.0).contains(&units)));
    assert!(take_in.line(room).contains("taken in beside the host"));
    assert!(matches!(
        TakeIn::judge(Some([21.0, 12.0, 75.0]), Some(host_at)),
        TakeIn::AlreadyBeside { .. }
    ));
    assert_eq!(
        TakeIn::judge(Some([-7.4, 12.0, 77.8]), None),
        TakeIn::HostNotKnown
    );
}

#[test]
fn the_host_is_never_taken_in_and_a_guest_alone_starts_its_own_room() {
    let mut host = RuleRoomWatch::new();
    assert_eq!(
        host.pop(&facts("Start_Law_Game_01", RulePlayer::Host, true)),
        RulePop::PassesOn
    );
    let mut guest = RuleRoomWatch::new();
    assert_eq!(
        guest.pop(&RulePopFacts {
            shared: false,
            ..facts("Start_Law_Game_01", RulePlayer::Guest, false)
        }),
        RulePop::PassesOn
    );
}

#[test]
fn only_room_eight_resets_the_room_as_one() {
    for room in &RULE_ROOMS {
        assert_eq!(room.resets_the_room, room.room == 8, "room {}", room.room);
    }
}

#[test]
fn the_hosts_clock_or_hit_in_room_eight_resets_the_room_once_for_both() {
    let mut host = RuleRoomWatch::new();
    armed(&mut host, "Hit_has_Done08");
    let time_up = host.pop(&facts("Time_has_Done", RulePlayer::Host, true));
    assert!(matches!(
        time_up,
        RulePop::RoomResets {
            act: RuleAct::TimeUp,
            ..
        }
    ));
    assert!(time_up.drops());
    assert!(
        time_up
            .log_line("Time_has_Done")
            .unwrap()
            .contains("resets the room, both players go back to the start together")
    );
    assert!(matches!(
        host.pop(&facts("Hit_has_Done08", RulePlayer::Host, true)),
        RulePop::DropAlreadyBroken { .. }
    ));
    assert!(matches!(
        host.accept_guest_break(TEMPLE, "Hit_has_Done08"),
        RoomResetVerdict::NotArmed {
            state: Some(RuleState::Broken),
            ..
        }
    ));
    assert!(
        host.pop(&facts("Restart_Law_Game_08", RulePlayer::Host, true))
            .drops()
    );
}

#[test]
fn the_guests_hit_in_room_eight_is_asked_of_the_host_and_its_own_clock_never_ends_the_room() {
    let mut guest = RuleRoomWatch::new();
    armed(&mut guest, "Hit_has_Done08");
    let time_up = guest.pop(&facts("Time_has_Done", RulePlayer::Guest, true));
    assert!(matches!(time_up, RulePop::DropGuestTimeUp { .. }));
    assert!(time_up.drops());
    let hit = guest.pop(&facts("Hit_has_Done08", RulePlayer::Guest, true));
    assert!(matches!(hit, RulePop::GuestAsksTheHost { .. }));
    assert!(hit.drops());
    let mut host = RuleRoomWatch::new();
    armed(&mut host, "Hit_has_Done08");
    let verdict = host.accept_guest_break(TEMPLE, "Hit_has_Done08");
    assert!(matches!(verdict, RoomResetVerdict::Resets { .. }));
    assert!(
        guest_break_line(&verdict, "Hit_has_Done08").contains("the room resets for both players")
    );
    assert!(matches!(
        host.accept_guest_break(TEMPLE, "Hit_has_Done08"),
        RoomResetVerdict::NotArmed { .. }
    ));
    assert!(matches!(
        host.accept_guest_break(TEMPLE, "Time_has_Done"),
        RoomResetVerdict::NotAResetBreak
    ));
    let reset = guest.accept_hosts_reset(TEMPLE, "Hit_has_Done08");
    assert!(matches!(reset, RoomResetVerdict::Resets { .. }));
    assert!(
        hosts_reset_line(&reset, "Hit_has_Done08")
            .contains("you go back to the start with the host")
    );
    assert!(matches!(
        guest.accept_hosts_reset(TEMPLE, "Jump_has_Done"),
        RoomResetVerdict::NotAResetBreak
    ));
}

#[test]
fn the_clock_counts_the_hosts_120_seconds_and_both_copies_log_it_every_ten() {
    let mut host = RoomClock::new();
    assert_eq!(host.remaining_seconds(0), None);
    host.start(1_000);
    assert_eq!(host.remaining_seconds(1_000), Some(120));
    assert_eq!(host.remaining_seconds(31_000), Some(90));
    assert_eq!(host.remaining_seconds(500_000), Some(0));
    assert_eq!(host.line_if_due(5_000, RulePlayer::Host), None);
    assert_eq!(
        host.line_if_due(11_000, RulePlayer::Host).unwrap(),
        "rule room: the room's clock (the host's) has 110 s left"
    );
    assert_eq!(host.line_if_due(12_000, RulePlayer::Host), None);
    let mut guest = RoomClock::new();
    guest.start(2_000);
    assert!(
        guest
            .line_if_due(12_000, RulePlayer::Guest)
            .unwrap()
            .contains("shown here is the host's, 110 s left")
    );
    guest.stop();
    assert_eq!(guest.line_if_due(40_000, RulePlayer::Guest), None);
}
