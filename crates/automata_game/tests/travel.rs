use automata_game::address::{FAST_TRAVEL_OBJECT, FAST_TRAVEL_TO_POINT_FUNCTION};
use automata_game::exe::ExeVariant;
use automata_game::start::PlayReadings;
use automata_game::travel::{
    ARRIVAL_SHOWN_AS_CURRENT_MS, BringPartnerRefusal, FAST_TRAVEL_IDLE_STATE,
    FAST_TRAVEL_STATE_THE_TRAVEL_CALL_RESETS_TO_IDLE, FOLLOWED_JUMP_WAIT_MS, GoToPartnerReadings,
    GoToPartnerRefusal, IncomingTeleport, JUMP_ARRIVAL_LIMIT_MS, JumpOrigin, JumpRefusal, JumpRole,
    LoadedPoints, MoveAsker, SCENE_LEAVE_LIMIT_MS, SCENE_LEAVE_LIMIT_WHILE_THE_TRANSPORT_STEPS_MS,
    TRAVEL_POINT_CODES, TeleportRequest, TravelAction, TravelMachine, TravelPoint, TravelView,
    WorldReadings, decide_bring_partner, decide_followed_jump, decide_go_to_partner,
    decide_local_jump, go_to_partner_line, handle_teleport_request, move_distance, newest_requests,
    point_list_log_line, world_refusal,
};

const RUNNING: i32 = 1;
const FADING_OUT: i32 = 2;
const IN_THE_WORLD_PHASE: i32 = 0x210;
const TITLE_PHASE: i32 = 0xF00;
const STEP_AFTER_THE_JUMP: i32 = 6;
const STEP_THAT_MAKES_THE_JUMP: i32 = 5;

fn point(code: &str) -> TravelPoint {
    TravelPoint::from_code(code).expect("a known code")
}

fn in_play() -> PlayReadings {
    PlayReadings {
        scene_state: RUNNING,
        phase_id: IN_THE_WORLD_PHASE,
        scene_loaded: 1,
        player_present: true,
    }
}

fn world_with(play: PlayReadings, codes: &[&str]) -> WorldReadings {
    WorldReadings {
        play,
        fast_travel_state: Some(FAST_TRAVEL_IDLE_STATE),
        fast_travel_step: Some(STEP_AFTER_THE_JUMP),
        loaded: LoadedPoints::of_names(codes.iter().copied()),
    }
}

fn ready_world() -> WorldReadings {
    world_with(in_play(), &["ft_AP", "ft_RC", "re_made_up"])
}

fn waiting_at_the_last_step() -> WorldReadings {
    WorldReadings {
        fast_travel_step: Some(STEP_THAT_MAKES_THE_JUMP),
        ..ready_world()
    }
}

fn changing_scene() -> WorldReadings {
    world_with(
        PlayReadings {
            scene_state: FADING_OUT,
            ..in_play()
        },
        &["ft_AP", "ft_RC"],
    )
}

fn called(machine: &mut TravelMachine, code: &str, origin: JumpOrigin, now_ms: u64) {
    machine.call_returned(now_ms, point(code), origin, true);
}

#[test]
fn the_addresses_are_the_ones_of_the_world_page() {
    assert_eq!(FAST_TRAVEL_OBJECT.rva(ExeVariant::Main), 0xFC0130);
    assert_eq!(FAST_TRAVEL_OBJECT.rva(ExeVariant::Compat), 0xFB4030);
    assert_eq!(
        FAST_TRAVEL_TO_POINT_FUNCTION.rva(ExeVariant::Main),
        0x358860
    );
    assert_eq!(
        FAST_TRAVEL_TO_POINT_FUNCTION.rva(ExeVariant::Compat),
        0x352CD0
    );
}

#[test]
fn there_are_twenty_two_codes_and_each_maps_both_ways() {
    assert_eq!(TRAVEL_POINT_CODES.len(), 22);
    assert_eq!(TravelPoint::all().count(), 22);
    for (index, code) in TRAVEL_POINT_CODES.iter().enumerate() {
        let from_code = TravelPoint::from_code(code).expect("known");
        assert_eq!(usize::from(from_code.index()), index);
        assert_eq!(TravelPoint::from_index(index as u32), Some(from_code));
        assert_eq!(from_code.code(), *code);
    }
    assert_eq!(TravelPoint::from_index(22), None);
    assert_eq!(TravelPoint::from_index(u32::MAX), None);
    assert_eq!(TravelPoint::from_code("ft_made_up"), None);
    assert_eq!(TravelPoint::from_code("FT_AP"), None);
}

#[test]
fn a_point_is_shown_with_the_games_code_as_it_is() {
    assert_eq!(point("ft_AP_T2").label(), "Travel point ft_AP_T2");
    assert_eq!(point("ft_BK").label(), "Travel point ft_BK");
}

#[test]
fn only_known_codes_of_the_games_list_are_offered() {
    let loaded = LoadedPoints::of_names(["re_made_up", "ft_RC", "ft_AP", "ft_AP", "ft_nope"]);
    assert_eq!(loaded.points(), vec![point("ft_RC"), point("ft_AP")]);
    assert!(loaded.contains(point("ft_AP")));
    assert!(!loaded.contains(point("ft_BK")));
    assert!(LoadedPoints::of_names([]).is_empty());
    assert!(LoadedPoints::of_names(["re_made_up"]).is_empty());
    assert!(LoadedPoints::default().points().is_empty());
}

#[test]
fn the_point_list_line_names_known_and_other_points_and_says_it_is_a_guess() {
    let names = vec![
        "ft_AP".to_owned(),
        "ft_RC".to_owned(),
        "re_made_up".to_owned(),
    ];
    let line = point_list_log_line(&names);
    assert!(line.contains("2 travel points (ft_AP, ft_RC)"), "{line}");
    assert!(line.contains("1 other points (re_made_up)"), "{line}");
    assert!(line.contains("a guess until run"), "{line}");
    let empty = point_list_log_line(&[]);
    assert!(empty.contains("0 travel points (none)"), "{empty}");
}

#[test]
fn the_role_comes_from_the_link() {
    assert_eq!(JumpRole::of_link(false, None), JumpRole::Solo);
    assert_eq!(JumpRole::of_link(false, Some(1)), JumpRole::Solo);
    assert_eq!(JumpRole::of_link(true, Some(0)), JumpRole::Host);
    assert_eq!(JumpRole::of_link(true, Some(1)), JumpRole::Guest);
    assert_eq!(JumpRole::of_link(true, None), JumpRole::Guest);
}

#[test]
fn the_host_and_a_lone_player_may_jump_and_a_guest_may_not() {
    let world = ready_world();
    for role in [JumpRole::Host, JumpRole::Solo] {
        assert_eq!(
            decide_local_jump(role, false, Some(&world), point("ft_AP")),
            Ok(())
        );
    }
    assert_eq!(
        decide_local_jump(JumpRole::Guest, false, Some(&world), point("ft_AP")),
        Err(JumpRefusal::NotHost)
    );
    assert_eq!(
        decide_followed_jump(false, Some(&world), point("ft_AP")),
        Ok(())
    );
}

#[test]
fn each_reason_a_jump_does_not_start_is_told_apart() {
    let ap = point("ft_AP");
    let host = |under_way, world: Option<&WorldReadings>, point| {
        decide_local_jump(JumpRole::Host, under_way, world, point)
    };
    assert_eq!(host(false, None, ap), Err(JumpRefusal::NoWorldYet));
    assert_eq!(
        host(true, Some(&ready_world()), ap),
        Err(JumpRefusal::JumpUnderWay)
    );
    assert_eq!(
        host(false, Some(&changing_scene()), ap),
        Err(JumpRefusal::SceneChangeUnderWay)
    );
    let not_loaded = world_with(
        PlayReadings {
            scene_loaded: 0,
            ..in_play()
        },
        &["ft_AP"],
    );
    assert_eq!(
        host(false, Some(&not_loaded), ap),
        Err(JumpRefusal::SceneChangeUnderWay)
    );
    let title = world_with(
        PlayReadings {
            phase_id: TITLE_PHASE,
            ..in_play()
        },
        &["ft_AP"],
    );
    assert_eq!(host(false, Some(&title), ap), Err(JumpRefusal::NoWorldYet));
    let no_hero = world_with(
        PlayReadings {
            player_present: false,
            ..in_play()
        },
        &["ft_AP"],
    );
    assert_eq!(
        host(false, Some(&no_hero), ap),
        Err(JumpRefusal::NoWorldYet)
    );
    let unreadable = WorldReadings {
        fast_travel_state: None,
        ..ready_world()
    };
    assert_eq!(
        host(false, Some(&unreadable), ap),
        Err(JumpRefusal::TransportUnreadable)
    );
    let busy = WorldReadings {
        fast_travel_state: Some(0x9126_EBA2),
        ..ready_world()
    };
    assert_eq!(
        host(false, Some(&busy), ap),
        Err(JumpRefusal::TransportBusy(0x9126_EBA2))
    );
    assert_eq!(
        host(false, Some(&ready_world()), point("ft_BK")),
        Err(JumpRefusal::PointNotLoaded(point("ft_BK")))
    );
}

#[test]
fn the_state_the_travel_call_resets_itself_does_not_block() {
    let world = WorldReadings {
        fast_travel_state: Some(FAST_TRAVEL_STATE_THE_TRAVEL_CALL_RESETS_TO_IDLE),
        ..ready_world()
    };
    assert_eq!(world_refusal(false, Some(&world)), None);
}

#[test]
fn the_refusals_read_as_plain_reasons() {
    assert_eq!(JumpRefusal::NotHost.to_string(), "only the host may jump");
    assert_eq!(JumpRefusal::NoWorldYet.to_string(), "no world yet");
    assert_eq!(
        JumpRefusal::SceneChangeUnderWay.to_string(),
        "refused while a scene change is under way"
    );
    assert_eq!(
        JumpRefusal::PointNotLoaded(point("ft_BK")).to_string(),
        "ft_BK is not among the points the game holds now"
    );
    assert!(
        JumpRefusal::TransportBusy(0x12)
            .to_string()
            .contains("0x12")
    );
}

#[test]
fn a_local_jump_runs_from_the_call_to_the_scene_running_again() {
    let mut machine = TravelMachine::default();
    let ap = point("ft_AP");
    let step = machine.request_local(JumpRole::Host, &ready_world(), ap);
    assert_eq!(
        step.action,
        TravelAction::Call {
            point: ap,
            origin: JumpOrigin::Local
        }
    );
    assert!(!machine.is_under_way());
    machine.call_returned(1_000, ap, JumpOrigin::Local, true);
    assert!(machine.is_under_way());
    assert!(machine.expects_scene_request());
    assert_eq!(machine.line(), "jumping to Travel point ft_AP");

    let still_running = machine.step(1_100, &waiting_at_the_last_step());
    assert_eq!(still_running.action, TravelAction::Nothing);
    assert!(machine.expects_scene_request());

    let left = machine.step(1_200, &changing_scene());
    assert!(left.logs[0].contains("left its running state"));
    assert!(machine.is_under_way());
    assert!(!machine.expects_scene_request());
    assert_eq!(machine.current_point(1_200), None);

    machine.step(1_200 + SCENE_LEAVE_LIMIT_MS * 2, &changing_scene());
    assert!(machine.is_under_way());

    let arrived = machine.step(60_000, &ready_world());
    assert!(arrived.logs[0].contains("arrived at ft_AP"));
    assert!(!machine.is_under_way());
    assert_eq!(machine.line(), "arrived at Travel point ft_AP");
    assert_eq!(machine.current_point(60_000), Some(ap));
    assert_eq!(
        machine.current_point(60_000 + ARRIVAL_SHOWN_AS_CURRENT_MS - 1),
        Some(ap)
    );
    assert_eq!(
        machine.current_point(60_000 + ARRIVAL_SHOWN_AS_CURRENT_MS),
        None
    );
}

#[test]
fn a_refused_local_jump_calls_nothing_and_says_why() {
    let mut machine = TravelMachine::default();
    let step = machine.request_local(JumpRole::Guest, &ready_world(), point("ft_AP"));
    assert_eq!(step.action, TravelAction::Nothing);
    assert_eq!(
        machine.line(),
        "not jumped to Travel point ft_AP: only the host may jump"
    );
    assert_eq!(step.logs, vec![format!("travel: {}", machine.line())]);
    assert!(!machine.is_under_way());
}

#[test]
fn a_call_the_game_does_not_take_is_shown_and_nothing_is_under_way() {
    let mut machine = TravelMachine::default();
    let logs = machine.call_returned(0, point("ft_RC"), JumpOrigin::Local, false);
    assert!(logs[0].contains("did not take the call for ft_RC"));
    assert!(!machine.is_under_way());
    assert!(!machine.expects_scene_request());
    assert_eq!(
        machine.line(),
        "not jumped to Travel point ft_RC: the game's transport did not take the call"
    );
}

#[test]
fn a_call_with_no_scene_change_gives_up_after_the_limit() {
    let mut machine = TravelMachine::default();
    let step_unreadable = WorldReadings {
        fast_travel_step: None,
        ..ready_world()
    };
    called(&mut machine, "ft_RC", JumpOrigin::Local, 1_000);
    let before = machine.step(1_000 + SCENE_LEAVE_LIMIT_MS - 1, &step_unreadable);
    assert!(before.logs.is_empty());
    assert!(machine.is_under_way());
    let after = machine.step(1_000 + SCENE_LEAVE_LIMIT_MS, &step_unreadable);
    assert!(after.logs[0].contains("no scene change within 15 s"));
    assert!(!after.cancel_waiting_step);
    assert!(!machine.is_under_way());
    assert!(!machine.expects_scene_request());
    assert_eq!(machine.current_point(1_000 + SCENE_LEAVE_LIMIT_MS), None);
    assert_eq!(
        machine.line(),
        "not jumped to Travel point ft_RC: no scene change within 15 s of the call"
    );
}

#[test]
fn a_call_the_transport_still_steps_through_is_given_more_time() {
    let stepping = |step| WorldReadings {
        fast_travel_step: Some(step),
        ..ready_world()
    };
    assert!(stepping(0).transport_is_stepping());
    assert!(stepping(5).transport_is_stepping());
    assert!(!stepping(6).transport_is_stepping());
    assert!(!stepping(-1).transport_is_stepping());
    let unreadable = WorldReadings {
        fast_travel_step: None,
        ..ready_world()
    };
    assert!(!unreadable.transport_is_stepping());

    let mut machine = TravelMachine::default();
    called(&mut machine, "ft_RC", JumpOrigin::Local, 0);
    machine.step(SCENE_LEAVE_LIMIT_MS, &stepping(2));
    assert!(machine.is_under_way());
    assert!(machine.expects_scene_request());
    machine.step(
        SCENE_LEAVE_LIMIT_WHILE_THE_TRANSPORT_STEPS_MS - 1,
        &stepping(5),
    );
    assert!(machine.is_under_way());
    let gave_up = machine.step(SCENE_LEAVE_LIMIT_WHILE_THE_TRANSPORT_STEPS_MS, &stepping(5));
    assert!(gave_up.logs[0].contains("step counter reads Some(5)"));
    assert!(gave_up.cancel_waiting_step);
    assert!(!machine.is_under_way());
    assert_eq!(
        machine.line(),
        "not jumped to Travel point ft_RC: no scene change within 60 s of the call"
    );

    let mut stopped = TravelMachine::default();
    called(&mut stopped, "ft_RC", JumpOrigin::Local, 0);
    stopped.step(SCENE_LEAVE_LIMIT_MS - 1, &stepping(5));
    let done_without_a_change = stopped.step(SCENE_LEAVE_LIMIT_MS, &stepping(6));
    assert!(done_without_a_change.logs[0].contains("step counter reads Some(6)"));
    assert!(!done_without_a_change.cancel_waiting_step);
    assert!(!stopped.is_under_way());
}

#[test]
fn a_scene_that_never_runs_again_gives_up_after_its_own_limit() {
    let mut machine = TravelMachine::default();
    called(&mut machine, "ft_RC", JumpOrigin::Local, 0);
    machine.step(100, &changing_scene());
    machine.step(100 + JUMP_ARRIVAL_LIMIT_MS - 1, &changing_scene());
    assert!(machine.is_under_way());
    let gave_up = machine.step(100 + JUMP_ARRIVAL_LIMIT_MS, &changing_scene());
    assert!(gave_up.logs[0].contains("did not run again within 300 s"));
    assert!(!machine.is_under_way());
}

#[test]
fn a_guest_follows_the_hosts_jump_once_its_world_is_ready() {
    let mut machine = TravelMachine::default();
    let ap = point("ft_AP");
    let logs = machine.follow_host(1_000, ap);
    assert!(logs[0].contains("the host jumps to ft_AP"));
    assert_eq!(
        machine.line(),
        "the host jumps to Travel point ft_AP; following"
    );

    let waiting = machine.step(1_100, &changing_scene());
    assert_eq!(waiting.action, TravelAction::Nothing);
    assert_eq!(waiting.logs.len(), 1);
    assert!(waiting.logs[0].contains("cannot follow the host to ft_AP yet"));
    let same_reason = machine.step(1_200, &changing_scene());
    assert!(same_reason.logs.is_empty());

    let ready = machine.step(1_300, &ready_world());
    assert_eq!(
        ready.action,
        TravelAction::Call {
            point: ap,
            origin: JumpOrigin::Host
        }
    );
    machine.call_returned(1_300, ap, JumpOrigin::Host, true);
    assert_eq!(machine.line(), "following the host to Travel point ft_AP");
    let no_second_call = machine.step(1_400, &waiting_at_the_last_step());
    assert_eq!(no_second_call.action, TravelAction::Nothing);
    machine.step(1_500, &changing_scene());
    machine.step(9_000, &ready_world());
    assert_eq!(machine.line(), "followed the host to Travel point ft_AP");
    assert_eq!(machine.current_point(9_000), Some(ap));
}

#[test]
fn a_guest_that_cannot_follow_in_time_says_why() {
    let mut machine = TravelMachine::default();
    machine.follow_host(0, point("ft_BK"));
    machine.step(10, &ready_world());
    let late = machine.step(FOLLOWED_JUMP_WAIT_MS, &ready_world());
    assert_eq!(late.action, TravelAction::Nothing);
    assert_eq!(
        machine.line(),
        "did not follow the host to Travel point ft_BK: ft_BK is not among the points the game holds now"
    );
    assert_eq!(late.logs, vec![format!("travel: {}", machine.line())]);
    let nothing_left = machine.step(
        FOLLOWED_JUMP_WAIT_MS + 10,
        &world_with(in_play(), &["ft_BK"]),
    );
    assert_eq!(nothing_left.action, TravelAction::Nothing);
}

#[test]
fn a_new_jump_forgets_the_last_arrival() {
    let mut machine = TravelMachine::default();
    called(&mut machine, "ft_AP", JumpOrigin::Local, 0);
    machine.step(10, &changing_scene());
    machine.step(20, &ready_world());
    assert_eq!(machine.current_point(30), Some(point("ft_AP")));
    let step = machine.request_local(JumpRole::Solo, &ready_world(), point("ft_RC"));
    assert!(matches!(step.action, TravelAction::Call { .. }));
    assert_eq!(machine.current_point(30), None);
}

#[test]
fn a_message_asks_to_come_or_to_follow_and_only_a_guest_follows() {
    assert_eq!(
        handle_teleport_request(JumpRole::Host, TeleportRequest::ComeToSender),
        IncomingTeleport::GoToPartner
    );
    assert_eq!(
        handle_teleport_request(JumpRole::Guest, TeleportRequest::ComeToSender),
        IncomingTeleport::GoToPartner
    );
    let follow = TeleportRequest::follow(point("ft_DC"));
    assert_eq!(
        follow,
        TeleportRequest::FollowJump {
            point: point("ft_DC").index()
        }
    );
    assert_eq!(
        handle_teleport_request(JumpRole::Guest, follow),
        IncomingTeleport::FollowJump(point("ft_DC"))
    );
    for role in [JumpRole::Host, JumpRole::Solo] {
        let ignored = handle_teleport_request(role, follow);
        assert_eq!(ignored, IncomingTeleport::IgnoredJumpOfANonHost);
        assert!(ignored.ignored_line().is_some());
    }
    let unknown =
        handle_teleport_request(JumpRole::Guest, TeleportRequest::FollowJump { point: 22 });
    assert_eq!(unknown, IncomingTeleport::IgnoredUnknownPoint(22));
    assert!(
        unknown
            .ignored_line()
            .is_some_and(|line| line.contains("22"))
    );
    assert_eq!(IncomingTeleport::GoToPartner.ignored_line(), None);
}

#[test]
fn a_request_is_valid_only_for_a_known_point() {
    assert!(TeleportRequest::ComeToSender.is_valid());
    assert!(TeleportRequest::FollowJump { point: 21 }.is_valid());
    assert!(!TeleportRequest::FollowJump { point: 22 }.is_valid());
}

fn ready_to_move() -> GoToPartnerReadings {
    GoToPartnerReadings {
        hero_present: true,
        hero_state: Some(0),
        partner_body_present: true,
        local_free: true,
        partner_state_fresh: true,
        same_phase: Some(true),
        ms_since_last_move: None,
    }
}

#[test]
fn going_to_the_partner_needs_a_hero_a_partner_body_and_a_state_the_teleport_takes() {
    let ready = ready_to_move();
    assert_eq!(decide_go_to_partner(&ready), Ok(()));
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            hero_present: false,
            hero_state: None,
            ..ready
        }),
        Err(GoToPartnerRefusal::NoHero)
    );
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            partner_body_present: false,
            ..ready
        }),
        Err(GoToPartnerRefusal::NoPartnerBody)
    );
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            hero_state: None,
            ..ready
        }),
        Err(GoToPartnerRefusal::HeroStateUnreadable)
    );
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            hero_state: Some(0xD3),
            ..ready
        }),
        Err(GoToPartnerRefusal::HeroInAStateTheTeleportIgnores)
    );
    assert_eq!(
        GoToPartnerRefusal::NoPartnerBody.to_string(),
        "there is no partner body yet"
    );
    assert!(
        GoToPartnerRefusal::HeroInAStateTheTeleportIgnores
            .to_string()
            .contains("0xD3")
    );
}

#[test]
fn a_move_is_refused_while_the_hero_is_held_dying_or_was_just_moved() {
    let ready = ready_to_move();
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            local_free: false,
            ..ready
        }),
        Err(GoToPartnerRefusal::NotFree)
    );
    for dying in [0x35, 0x36] {
        assert_eq!(
            decide_go_to_partner(&GoToPartnerReadings {
                hero_state: Some(dying),
                ..ready
            }),
            Err(GoToPartnerRefusal::HeroDying)
        );
    }
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            ms_since_last_move: Some(1_999),
            ..ready
        }),
        Err(GoToPartnerRefusal::MovedTooRecently)
    );
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            ms_since_last_move: Some(2_000),
            ..ready
        }),
        Ok(())
    );
}

#[test]
fn a_move_needs_a_fresh_position_of_the_other_player_in_the_same_phase() {
    let ready = ready_to_move();
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            partner_state_fresh: false,
            ..ready
        }),
        Err(GoToPartnerRefusal::PartnerStateStale)
    );
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            same_phase: None,
            ..ready
        }),
        Err(GoToPartnerRefusal::PartnerStateStale)
    );
    assert_eq!(
        decide_go_to_partner(&GoToPartnerReadings {
            same_phase: Some(false),
            ..ready
        }),
        Err(GoToPartnerRefusal::DifferentPhase)
    );
}

#[test]
fn bringing_the_partner_needs_a_connected_player_in_the_same_phase() {
    assert_eq!(decide_bring_partner(true, Some(true)), Ok(()));
    assert_eq!(decide_bring_partner(true, None), Ok(()));
    assert_eq!(
        decide_bring_partner(false, Some(true)),
        Err(BringPartnerRefusal::NotConnected)
    );
    assert_eq!(
        decide_bring_partner(true, Some(false)),
        Err(BringPartnerRefusal::DifferentPhase)
    );
    assert_eq!(
        BringPartnerRefusal::NotConnected.to_string(),
        "no other player is connected"
    );
}

#[test]
fn only_the_newest_request_of_each_kind_is_kept() {
    let requests = [
        TeleportRequest::ComeToSender,
        TeleportRequest::FollowJump { point: 1 },
        TeleportRequest::ComeToSender,
        TeleportRequest::FollowJump { point: 4 },
        TeleportRequest::ComeToSender,
    ];
    let newest = newest_requests(&requests);
    assert!(newest.come_to_sender);
    assert_eq!(
        newest.follow_jump,
        Some(TeleportRequest::FollowJump { point: 4 })
    );
    assert_eq!(newest.ignored, 3);
    let none = newest_requests(&[]);
    assert!(!none.come_to_sender);
    assert_eq!(none.follow_jump, None);
    assert_eq!(none.ignored, 0);
}

#[test]
fn the_distance_of_a_move_is_the_straight_line() {
    assert_eq!(move_distance([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]), 5.0);
    assert_eq!(move_distance([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]), 0.0);
}

#[test]
fn a_call_is_refused_while_the_transport_counts_its_own_steps() {
    for step in 0..=5 {
        let stepping = WorldReadings {
            fast_travel_step: Some(step),
            ..ready_world()
        };
        assert_eq!(
            decide_local_jump(JumpRole::Host, false, Some(&stepping), point("ft_AP")),
            Err(JumpRefusal::TransportStepping(step))
        );
    }
    for at_rest in [-1, 6] {
        let resting = WorldReadings {
            fast_travel_step: Some(at_rest),
            ..ready_world()
        };
        assert_eq!(
            decide_local_jump(JumpRole::Host, false, Some(&resting), point("ft_AP")),
            Ok(())
        );
    }
    assert!(
        JumpRefusal::TransportStepping(3)
            .to_string()
            .contains("step 3")
    );
}

#[test]
fn a_call_the_transport_dropped_ends_at_once_and_cancels_nothing() {
    let mut machine = TravelMachine::default();
    called(&mut machine, "ft_RC", JumpOrigin::Local, 0);
    let dropped = machine.step(16, &ready_world());
    assert!(dropped.logs[0].contains("the wait ends now"));
    assert!(!dropped.cancel_waiting_step);
    assert!(!machine.is_under_way());
    assert!(!machine.expects_scene_request());
    assert_eq!(
        machine.line(),
        "not jumped to Travel point ft_RC: the game's transport dropped the call"
    );
}

#[test]
fn a_step_below_the_last_one_is_left_alone_at_the_limit() {
    let mut machine = TravelMachine::default();
    called(&mut machine, "ft_RC", JumpOrigin::Local, 0);
    let in_the_long_sequence = WorldReadings {
        fast_travel_step: Some(2),
        ..ready_world()
    };
    let gave_up = machine.step(
        SCENE_LEAVE_LIMIT_WHILE_THE_TRANSPORT_STEPS_MS,
        &in_the_long_sequence,
    );
    assert!(!gave_up.cancel_waiting_step);
    assert!(gave_up.logs[0].contains("left alone"));
}

#[test]
fn the_guest_is_told_when_the_scene_leaves_not_when_the_call_is_taken() {
    let mut machine = TravelMachine::default();
    let ap = point("ft_AP");
    called(&mut machine, "ft_AP", JumpOrigin::Local, 0);
    let waiting = machine.step(16, &waiting_at_the_last_step());
    assert_eq!(waiting.tell_guest, None);
    let left = machine.step(32, &changing_scene());
    assert_eq!(left.tell_guest, Some(ap));
    let later = machine.step(48, &changing_scene());
    assert_eq!(later.tell_guest, None);

    let mut follower = TravelMachine::default();
    called(&mut follower, "ft_AP", JumpOrigin::Host, 0);
    assert_eq!(follower.step(16, &changing_scene()).tell_guest, None);
}

#[test]
fn the_wait_to_follow_counts_only_while_this_game_is_in_play() {
    let mut machine = TravelMachine::default();
    machine.follow_host(0, point("ft_BK"));
    assert!(machine.has_waiting_follow());
    let loading = machine.step(FOLLOWED_JUMP_WAIT_MS * 4, &changing_scene());
    assert_eq!(loading.action, TravelAction::Nothing);
    assert!(machine.has_waiting_follow());
    let in_play_at = FOLLOWED_JUMP_WAIT_MS * 4 + 16;
    machine.step(in_play_at, &ready_world());
    machine.step(in_play_at + FOLLOWED_JUMP_WAIT_MS - 17, &ready_world());
    assert!(machine.has_waiting_follow());
    let late = machine.step(in_play_at + FOLLOWED_JUMP_WAIT_MS, &ready_world());
    assert!(late.logs[0].contains("did not follow the host"));
    assert!(!machine.has_waiting_follow());
}

#[test]
fn the_move_line_says_who_asked_and_how_it_went() {
    let refused = Err("there is no partner body yet".to_owned());
    assert_eq!(
        go_to_partner_line(MoveAsker::Local, &Ok(())),
        "moved to the partner"
    );
    assert_eq!(
        go_to_partner_line(MoveAsker::Local, &refused),
        "not moved: there is no partner body yet"
    );
    assert_eq!(
        go_to_partner_line(MoveAsker::Partner, &Ok(())),
        "the other player brought you to them"
    );
    assert_eq!(
        go_to_partner_line(MoveAsker::Partner, &refused),
        "the other player asked to bring you; not moved: there is no partner body yet"
    );
}

#[test]
fn the_view_lists_loaded_points_and_blocks_like_the_rules() {
    let empty = TravelView::default();
    assert!(empty.points().is_empty());
    assert_eq!(empty.jump_blocker(), Some(JumpRefusal::NoWorldYet));
    assert_eq!(empty.status_line(), "teleport: nothing asked yet");
    assert!(empty.notes().is_empty());

    let host = TravelView {
        role: JumpRole::Host,
        world: Some(ready_world()),
        ..TravelView::default()
    };
    assert_eq!(host.points(), vec![point("ft_RC"), point("ft_AP")]);
    assert_eq!(host.jump_blocker(), None);
    assert_eq!(host.point_blocker(point("ft_AP")), None);
    assert_eq!(
        host.point_blocker(point("ft_BK")),
        Some(JumpRefusal::PointNotLoaded(point("ft_BK")))
    );

    let guest = TravelView {
        role: JumpRole::Guest,
        ..host.clone()
    };
    assert_eq!(guest.jump_blocker(), Some(JumpRefusal::NotHost));
    assert_eq!(
        guest.point_blocker(point("ft_AP")),
        Some(JumpRefusal::NotHost)
    );

    let busy = TravelView {
        jump_under_way: true,
        ..host.clone()
    };
    assert_eq!(busy.jump_blocker(), Some(JumpRefusal::JumpUnderWay));
}

#[test]
fn the_view_shows_the_last_move_and_the_last_jump() {
    let view = TravelView {
        travel_line: "arrived at Travel point ft_AP".to_owned(),
        move_line: "the other player brought you to them".to_owned(),
        ..TravelView::default()
    };
    assert_eq!(
        view.notes(),
        vec![
            "the other player brought you to them".to_owned(),
            "arrived at Travel point ft_AP".to_owned()
        ]
    );
    assert_eq!(
        view.status_line(),
        "teleport: the other player brought you to them; arrived at Travel point ft_AP"
    );
}

#[test]
fn the_same_jump_asked_again_while_it_waits_is_one_wait_and_one_line() {
    let mut machine = TravelMachine::default();
    let bk = point("ft_BK");
    assert_eq!(machine.follow_host(0, bk).len(), 1);
    machine.step(FOLLOWED_JUMP_WAIT_MS - 1, &ready_world());
    assert!(
        machine
            .follow_host(FOLLOWED_JUMP_WAIT_MS - 1, bk)
            .is_empty()
    );
    let late = machine.step(FOLLOWED_JUMP_WAIT_MS, &ready_world());
    assert!(late.logs[0].contains("did not follow the host"));
    assert_eq!(
        machine
            .follow_host(FOLLOWED_JUMP_WAIT_MS + 1, point("ft_AP"))
            .len(),
        1
    );
}
