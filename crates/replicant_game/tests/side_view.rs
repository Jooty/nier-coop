use replicant_game::side_view::{
    ArrivalViewFix, CHANGE_CAMERA_STATUS_NATIVE_RVA, DoorMoveAction, DoorMoveFacts,
    INDOOR_ARRIVAL_SETTLE_MS, IndoorArrivalReading, IndoorArrivalWatch, OwnSideViewDoor,
    SIDE_VIEW_DOORS, SIDE_VIEW_MODE_OFF_NATIVE_RVA, SideViewWatch,
    TERRAIN_PARTS_VISIBLE_NATIVE_RVA, ViewOffArg, door_action_for_a_move, door_by_id,
    door_entered_by, door_with_start_point, view_off_calls,
};
use replicant_game::story_calls::is_event_id_name;
use replicant_proto::NO_SIDE_VIEW_DOOR;

#[test]
fn the_first_look_only_sets_the_baseline() {
    let mut watch = SideViewWatch::default();
    assert_eq!(watch.observe(true), None);
}

#[test]
fn a_flip_is_reported_once() {
    let mut watch = SideViewWatch::default();
    watch.observe(false);
    assert_eq!(watch.observe(true), Some(true));
    assert_eq!(watch.observe(true), None);
    assert_eq!(watch.observe(false), Some(false));
}

const AREA_ONE: &str = "A_CENTER_VILLAGE_01";
const AREA_ZERO: &str = "A_CENTER_LIBRARY_01";
const HOUSE_DOOR: u16 = 1;
const EAST_ROOM_DOOR: u16 = 2;
const WEST_ROOM_DOOR: u16 = 4;
const INDOOR_START_POINT: i32 = 8;

#[test]
fn every_door_has_its_own_id_and_none_uses_the_no_door_id() {
    let mut ids: Vec<u16> = SIDE_VIEW_DOORS.iter().map(|door| door.id).collect();
    assert!(!ids.contains(&NO_SIDE_VIEW_DOOR));
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), SIDE_VIEW_DOORS.len());
}

#[test]
fn every_door_notes_its_source_and_names_a_callback_of_its_area() {
    for door in &SIDE_VIEW_DOORS {
        assert!(door.source.starts_with("listing c_"), "{}", door.id);
        assert!(door.source.contains(door.area), "{}", door.id);
        assert!(door.entry_callback.starts_with("GM_IN_"), "{}", door.id);
    }
}

#[test]
fn entry_steps_begin_with_the_callback_that_turns_the_view_on() {
    for door in SIDE_VIEW_DOORS
        .iter()
        .filter(|door| !door.entry_steps.is_empty())
    {
        assert_eq!(door.entry_steps[0], door.entry_callback, "{}", door.id);
        assert!(
            door.entry_steps
                .iter()
                .all(|step| step.starts_with("GM_IN_")),
            "{}",
            door.id
        );
    }
}

#[test]
fn no_step_the_mod_may_run_is_a_story_event_or_one_of_the_refused_door_callbacks() {
    const REFUSED: [&str; 7] = [
        "GM_OUT_Door_of_NierHouse_02",
        "GM_OUT_Door_of_NierHouse_14",
        "GM_OUT_Door_of_NierHouse_24",
        "GM_OUT_Door_of_PopolRoom_12",
        "GM_OUT_Door_of_PopolRoom_13",
        "GM_IN_Door_of_WestRoom_03",
        "GM_IN_Door_of_WestRoom2_03",
    ];
    for door in &SIDE_VIEW_DOORS {
        for step in door.entry_steps.iter().chain(door.exit_steps) {
            assert!(!REFUSED.contains(step), "{step}");
            assert!(!is_event_id_name(step), "{step}");
        }
    }
}

#[test]
fn the_house_of_area_one_may_be_entered_but_not_left_by_the_mod() {
    let door = door_by_id(AREA_ONE, HOUSE_DOOR).expect("the house is listed");
    assert_eq!(door.entry_steps.len(), 2);
    assert!(door.exit_steps.is_empty());
    assert_eq!(door.indoor_start_points, &[INDOOR_START_POINT]);
}

#[test]
fn the_east_room_of_area_zero_may_be_entered_and_left_by_the_mod() {
    let door = door_by_id(AREA_ZERO, EAST_ROOM_DOOR).expect("the room is listed");
    assert_eq!(door.entry_steps.len(), 2);
    assert_eq!(door.exit_steps.len(), 1);
}

#[test]
fn the_west_room_of_area_zero_may_only_be_left_by_the_mod() {
    let door = door_by_id(AREA_ZERO, WEST_ROOM_DOOR).expect("the room is listed");
    assert!(door.entry_steps.is_empty());
    assert_eq!(door.exit_steps.len(), 1);
}

#[test]
fn a_door_is_only_found_in_its_own_area() {
    assert!(door_by_id(AREA_ZERO, HOUSE_DOOR).is_none());
    assert!(door_by_id(AREA_ONE, NO_SIDE_VIEW_DOOR).is_none());
    assert!(door_entered_by(AREA_ZERO, "GM_IN_Door_of_NierHouse_02").is_none());
    assert_eq!(
        door_entered_by(AREA_ONE, "GM_IN_Door_of_NierHouse_02").map(|door| door.id),
        Some(HOUSE_DOOR)
    );
    assert!(door_with_start_point(AREA_ZERO, INDOOR_START_POINT).is_none());
    assert!(door_with_start_point(AREA_ONE, 0).is_none());
}

fn move_facts(area: &str) -> DoorMoveFacts<'_> {
    DoorMoveFacts {
        area,
        moved_in_side_view: false,
        moved_door: NO_SIDE_VIEW_DOOR,
        target_in_side_view: false,
        target_door: NO_SIDE_VIEW_DOOR,
    }
}

#[test]
fn a_move_into_a_door_place_runs_that_doors_entry_steps() {
    let facts = DoorMoveFacts {
        target_in_side_view: true,
        target_door: HOUSE_DOOR,
        ..move_facts(AREA_ONE)
    };
    assert_eq!(
        door_action_for_a_move(&facts),
        DoorMoveAction::RunSteps(&["GM_IN_Door_of_NierHouse_02", "GM_IN_Door_of_NierHouse_03"])
    );
}

#[test]
fn a_move_out_runs_the_exit_steps_where_they_just_end_the_view_and_the_off_routine_elsewhere() {
    let out_of_the_room = DoorMoveFacts {
        moved_in_side_view: true,
        moved_door: EAST_ROOM_DOOR,
        ..move_facts(AREA_ZERO)
    };
    assert_eq!(
        door_action_for_a_move(&out_of_the_room),
        DoorMoveAction::RunSteps(&["GM_OUT_Door_of_PopolRoom_02"])
    );
    let out_of_the_house = DoorMoveFacts {
        moved_in_side_view: true,
        moved_door: HOUSE_DOOR,
        ..move_facts(AREA_ONE)
    };
    let house = door_by_id(AREA_ONE, HOUSE_DOOR).expect("the house is listed");
    assert_eq!(
        door_action_for_a_move(&out_of_the_house),
        DoorMoveAction::ViewOffRoutine(house)
    );
}

#[test]
fn a_move_into_the_west_room_runs_nothing() {
    let facts = DoorMoveFacts {
        target_in_side_view: true,
        target_door: WEST_ROOM_DOOR,
        ..move_facts(AREA_ZERO)
    };
    assert!(door_action_for_a_move(&facts).is_nothing());
}

#[test]
fn a_move_that_crosses_no_view_or_meets_no_listed_door_runs_nothing() {
    assert!(door_action_for_a_move(&move_facts(AREA_ONE)).is_nothing());
    let both_inside = DoorMoveFacts {
        moved_in_side_view: true,
        moved_door: HOUSE_DOOR,
        target_in_side_view: true,
        target_door: HOUSE_DOOR,
        ..move_facts(AREA_ONE)
    };
    assert!(door_action_for_a_move(&both_inside).is_nothing());
    let view_from_a_trap = DoorMoveFacts {
        target_in_side_view: true,
        ..move_facts(AREA_ONE)
    };
    assert!(door_action_for_a_move(&view_from_a_trap).is_nothing());
    let door_of_another_area = DoorMoveFacts {
        target_in_side_view: true,
        target_door: HOUSE_DOOR,
        ..move_facts(AREA_ZERO)
    };
    assert!(door_action_for_a_move(&door_of_another_area).is_nothing());
}

#[test]
fn a_player_remembers_the_listed_door_that_turned_its_view_on() {
    let mut own = OwnSideViewDoor::new();
    assert_eq!(own.door(), NO_SIDE_VIEW_DOOR);
    own.view_flipped(true, Some(AREA_ONE), "GM_IN_Door_of_NierHouse_02");
    assert_eq!(own.door(), HOUSE_DOOR);
    own.view_flipped(false, Some(AREA_ONE), "GM_OUT_Door_of_NierHouse_02");
    assert_eq!(own.door(), NO_SIDE_VIEW_DOOR);
}

#[test]
fn a_view_turned_on_by_anything_else_is_no_door() {
    let mut own = OwnSideViewDoor::new();
    own.view_flipped(true, Some(AREA_ONE), "SomeTrapCallback");
    assert_eq!(own.door(), NO_SIDE_VIEW_DOOR);
    own.view_flipped(true, None, "GM_IN_Door_of_NierHouse_02");
    assert_eq!(own.door(), NO_SIDE_VIEW_DOOR);
    own.view_flipped(true, Some(AREA_ZERO), "GM_IN_Door_of_NierHouse_02");
    assert_eq!(own.door(), NO_SIDE_VIEW_DOOR);
}

fn arrived(now_ms: u64, area_loads: u32) -> IndoorArrivalReading<'static> {
    IndoorArrivalReading {
        now_ms,
        area_loads,
        area: Some(AREA_ONE),
        start_point: INDOOR_START_POINT,
        in_play_and_free: true,
        in_side_view: false,
    }
}

fn watch_after_a_load() -> IndoorArrivalWatch {
    let mut watch = IndoorArrivalWatch::new();
    assert_eq!(watch.observe(&arrived(0, 4)), ArrivalViewFix::Nothing);
    assert_eq!(watch.observe(&arrived(100, 5)), ArrivalViewFix::Nothing);
    watch
}

#[test]
fn a_load_onto_an_indoor_start_point_without_the_view_finds_the_door_once() {
    let mut watch = watch_after_a_load();
    assert_eq!(
        watch.observe(&arrived(100 + INDOOR_ARRIVAL_SETTLE_MS - 1, 5)),
        ArrivalViewFix::Nothing
    );
    let house = door_by_id(AREA_ONE, HOUSE_DOOR).expect("the house is listed");
    assert_eq!(
        watch.observe(&arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)),
        ArrivalViewFix::RunEntrySteps(house)
    );
    assert_eq!(
        watch.observe(&arrived(200 + INDOOR_ARRIVAL_SETTLE_MS, 5)),
        ArrivalViewFix::Nothing
    );
}

#[test]
fn a_player_who_was_already_there_when_the_watch_began_is_left_alone() {
    let mut watch = IndoorArrivalWatch::new();
    assert_eq!(watch.observe(&arrived(0, 4)), ArrivalViewFix::Nothing);
    assert_eq!(watch.observe(&arrived(60_000, 4)), ArrivalViewFix::Nothing);
}

#[test]
fn an_arrival_the_game_gave_the_view_itself_is_left_alone() {
    let mut watch = watch_after_a_load();
    let with_view = IndoorArrivalReading {
        in_side_view: true,
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert_eq!(watch.observe(&with_view), ArrivalViewFix::Nothing);
    assert_eq!(
        watch.observe(&arrived(200 + INDOOR_ARRIVAL_SETTLE_MS, 5)),
        ArrivalViewFix::Nothing
    );
}

#[test]
fn an_arrival_waits_until_the_player_has_been_free_for_the_settle_time() {
    let mut watch = watch_after_a_load();
    let busy = IndoorArrivalReading {
        in_play_and_free: false,
        ..arrived(5_000, 5)
    };
    assert_eq!(watch.observe(&busy), ArrivalViewFix::Nothing);
    assert_eq!(watch.observe(&arrived(5_100, 5)), ArrivalViewFix::Nothing);
    assert!(matches!(
        watch.observe(&arrived(5_100 + INDOOR_ARRIVAL_SETTLE_MS, 5)),
        ArrivalViewFix::RunEntrySteps(_)
    ));
}

#[test]
fn an_arrival_on_another_start_point_or_in_another_area_finds_no_door() {
    let mut watch = watch_after_a_load();
    let outdoors = IndoorArrivalReading {
        start_point: 1,
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert_eq!(watch.observe(&outdoors), ArrivalViewFix::Nothing);
    let mut watch = watch_after_a_load();
    let other_area = IndoorArrivalReading {
        area: Some(AREA_ZERO),
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert_eq!(watch.observe(&other_area), ArrivalViewFix::Nothing);
}

fn left_by_the_mod_with_the_view_on() -> IndoorArrivalWatch {
    let mut watch = IndoorArrivalWatch::new();
    assert_eq!(watch.observe(&arrived(0, 4)), ArrivalViewFix::Nothing);
    watch.mod_requested_an_area_change(true);
    assert_eq!(watch.observe(&arrived(100, 5)), ArrivalViewFix::Nothing);
    watch
}

fn landed_outdoors_with_the_view_on(now_ms: u64) -> IndoorArrivalReading<'static> {
    IndoorArrivalReading {
        start_point: 1,
        in_side_view: true,
        ..arrived(now_ms, 5)
    }
}

#[test]
fn a_view_the_mods_area_change_carried_out_of_a_door_place_is_turned_off_once() {
    let mut watch = left_by_the_mod_with_the_view_on();
    let settled = 100 + INDOOR_ARRIVAL_SETTLE_MS;
    assert_eq!(
        watch.observe(&landed_outdoors_with_the_view_on(settled)),
        ArrivalViewFix::TurnViewOff
    );
    assert_eq!(
        watch.observe(&landed_outdoors_with_the_view_on(settled + 100)),
        ArrivalViewFix::Nothing
    );
}

#[test]
fn a_view_that_is_on_after_a_load_the_mod_did_not_make_is_never_touched() {
    let mut watch = watch_after_a_load();
    assert_eq!(
        watch.observe(&landed_outdoors_with_the_view_on(
            100 + INDOOR_ARRIVAL_SETTLE_MS
        )),
        ArrivalViewFix::Nothing
    );
}

#[test]
fn a_mod_area_change_made_with_the_view_off_never_turns_a_view_off() {
    let mut watch = IndoorArrivalWatch::new();
    assert_eq!(watch.observe(&arrived(0, 4)), ArrivalViewFix::Nothing);
    watch.mod_requested_an_area_change(false);
    assert_eq!(watch.observe(&arrived(100, 5)), ArrivalViewFix::Nothing);
    assert_eq!(
        watch.observe(&landed_outdoors_with_the_view_on(
            100 + INDOOR_ARRIVAL_SETTLE_MS
        )),
        ArrivalViewFix::Nothing
    );
}

#[test]
fn a_view_carried_onto_an_indoor_start_point_is_kept() {
    let mut watch = left_by_the_mod_with_the_view_on();
    let indoors = IndoorArrivalReading {
        in_side_view: true,
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert_eq!(watch.observe(&indoors), ArrivalViewFix::Nothing);
}

#[test]
fn the_mods_area_change_is_forgotten_after_the_load_it_made() {
    let mut watch = left_by_the_mod_with_the_view_on();
    let view_off = IndoorArrivalReading {
        start_point: 1,
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert_eq!(watch.observe(&view_off), ArrivalViewFix::Nothing);
    assert_eq!(
        watch.observe(&landed_outdoors_with_the_view_on(5_000)),
        ArrivalViewFix::Nothing
    );
    assert_eq!(
        watch.observe(&IndoorArrivalReading {
            area_loads: 6,
            ..landed_outdoors_with_the_view_on(6_000)
        }),
        ArrivalViewFix::Nothing
    );
    assert_eq!(
        watch.observe(&IndoorArrivalReading {
            area_loads: 6,
            ..landed_outdoors_with_the_view_on(6_000 + INDOOR_ARRIVAL_SETTLE_MS)
        }),
        ArrivalViewFix::Nothing
    );
}

#[test]
fn the_off_routine_calls_the_games_view_off_native_and_never_a_story_call() {
    for door in [None, door_by_id(AREA_ONE, HOUSE_DOOR)] {
        let calls = view_off_calls(door);
        let offs = calls
            .iter()
            .filter(|call| call.native_rva == SIDE_VIEW_MODE_OFF_NATIVE_RVA)
            .count();
        assert_eq!(offs, 1);
        let mut natives: Vec<u32> = calls.iter().map(|call| call.native_rva).collect();
        natives.sort_unstable();
        natives.dedup();
        assert!(natives.len() <= 8, "only the eight view natives are called");
    }
}

#[test]
fn leaving_a_door_place_inside_its_area_also_restores_its_shell_and_outdoor_camera() {
    let house = door_by_id(AREA_ONE, HOUSE_DOOR).expect("the house is listed");
    let calls = view_off_calls(Some(house));
    let shown: Vec<&ViewOffArg> = calls
        .iter()
        .filter(|call| call.native_rva == TERRAIN_PARTS_VISIBLE_NATIVE_RVA)
        .flat_map(|call| &call.args)
        .collect();
    assert_eq!(shown, [&ViewOffArg::Text("PARTS_GROUP001")]);
    let status = calls
        .iter()
        .find(|call| call.native_rva == CHANGE_CAMERA_STATUS_NATIVE_RVA)
        .expect("the outdoor camera status is set");
    assert_eq!(status.args[0], ViewOffArg::Int(house.outdoor_camera_status));
}

#[test]
fn after_an_area_change_the_off_routine_leaves_the_new_areas_shell_and_camera_status_alone() {
    let calls = view_off_calls(None);
    assert!(calls.iter().all(|call| {
        call.native_rva != TERRAIN_PARTS_VISIBLE_NATIVE_RVA
            && call.native_rva != CHANGE_CAMERA_STATUS_NATIVE_RVA
    }));
}
