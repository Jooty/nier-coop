use replicant_game::side_view::{
    DoorMoveFacts, INDOOR_ARRIVAL_SETTLE_MS, IndoorArrivalReading, IndoorArrivalWatch,
    OwnSideViewDoor, SIDE_VIEW_DOORS, SideViewWatch, door_by_id, door_entered_by,
    door_steps_for_a_move, door_with_start_point,
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
        door_steps_for_a_move(&facts),
        ["GM_IN_Door_of_NierHouse_02", "GM_IN_Door_of_NierHouse_03"]
    );
}

#[test]
fn a_move_out_of_a_door_place_runs_its_exit_steps_only_where_they_just_end_the_view() {
    let out_of_the_room = DoorMoveFacts {
        moved_in_side_view: true,
        moved_door: EAST_ROOM_DOOR,
        ..move_facts(AREA_ZERO)
    };
    assert_eq!(
        door_steps_for_a_move(&out_of_the_room),
        ["GM_OUT_Door_of_PopolRoom_02"]
    );
    let out_of_the_house = DoorMoveFacts {
        moved_in_side_view: true,
        moved_door: HOUSE_DOOR,
        ..move_facts(AREA_ONE)
    };
    assert!(door_steps_for_a_move(&out_of_the_house).is_empty());
}

#[test]
fn a_move_into_the_west_room_runs_nothing() {
    let facts = DoorMoveFacts {
        target_in_side_view: true,
        target_door: WEST_ROOM_DOOR,
        ..move_facts(AREA_ZERO)
    };
    assert!(door_steps_for_a_move(&facts).is_empty());
}

#[test]
fn a_move_that_crosses_no_view_or_meets_no_listed_door_runs_nothing() {
    assert!(door_steps_for_a_move(&move_facts(AREA_ONE)).is_empty());
    let both_inside = DoorMoveFacts {
        moved_in_side_view: true,
        moved_door: HOUSE_DOOR,
        target_in_side_view: true,
        target_door: HOUSE_DOOR,
        ..move_facts(AREA_ONE)
    };
    assert!(door_steps_for_a_move(&both_inside).is_empty());
    let view_from_a_trap = DoorMoveFacts {
        target_in_side_view: true,
        ..move_facts(AREA_ONE)
    };
    assert!(door_steps_for_a_move(&view_from_a_trap).is_empty());
    let door_of_another_area = DoorMoveFacts {
        target_in_side_view: true,
        target_door: HOUSE_DOOR,
        ..move_facts(AREA_ZERO)
    };
    assert!(door_steps_for_a_move(&door_of_another_area).is_empty());
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
    assert!(watch.observe(&arrived(0, 4)).is_none());
    assert!(watch.observe(&arrived(100, 5)).is_none());
    watch
}

#[test]
fn a_load_onto_an_indoor_start_point_without_the_view_finds_the_door_once() {
    let mut watch = watch_after_a_load();
    assert!(
        watch
            .observe(&arrived(100 + INDOOR_ARRIVAL_SETTLE_MS - 1, 5))
            .is_none()
    );
    let door = watch.observe(&arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5));
    assert_eq!(door.map(|door| door.id), Some(HOUSE_DOOR));
    assert!(
        watch
            .observe(&arrived(200 + INDOOR_ARRIVAL_SETTLE_MS, 5))
            .is_none()
    );
}

#[test]
fn a_player_who_was_already_there_when_the_watch_began_is_left_alone() {
    let mut watch = IndoorArrivalWatch::new();
    assert!(watch.observe(&arrived(0, 4)).is_none());
    assert!(watch.observe(&arrived(60_000, 4)).is_none());
}

#[test]
fn an_arrival_the_game_gave_the_view_itself_is_left_alone() {
    let mut watch = watch_after_a_load();
    let with_view = IndoorArrivalReading {
        in_side_view: true,
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert!(watch.observe(&with_view).is_none());
    assert!(
        watch
            .observe(&arrived(200 + INDOOR_ARRIVAL_SETTLE_MS, 5))
            .is_none()
    );
}

#[test]
fn an_arrival_waits_until_the_player_has_been_free_for_the_settle_time() {
    let mut watch = watch_after_a_load();
    let busy = IndoorArrivalReading {
        in_play_and_free: false,
        ..arrived(5_000, 5)
    };
    assert!(watch.observe(&busy).is_none());
    assert!(watch.observe(&arrived(5_100, 5)).is_none());
    assert!(
        watch
            .observe(&arrived(5_100 + INDOOR_ARRIVAL_SETTLE_MS, 5))
            .is_some()
    );
}

#[test]
fn an_arrival_on_another_start_point_or_in_another_area_finds_no_door() {
    let mut watch = watch_after_a_load();
    let outdoors = IndoorArrivalReading {
        start_point: 1,
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert!(watch.observe(&outdoors).is_none());
    let mut watch = watch_after_a_load();
    let other_area = IndoorArrivalReading {
        area: Some(AREA_ZERO),
        ..arrived(100 + INDOOR_ARRIVAL_SETTLE_MS, 5)
    };
    assert!(watch.observe(&other_area).is_none());
}
