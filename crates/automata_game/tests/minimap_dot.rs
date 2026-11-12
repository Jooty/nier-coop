use automata_game::exe::ExeVariant;
use automata_game::partner_hud::{
    GameMinimap, MINIMAP_UPDATE_FUNCTION_COMPAT_START, MINIMAP_UPDATE_FUNCTION_MAIN_START,
    minimap_dot, minimap_hidden_reason, minimap_update_start, ui_point_on_display,
};

use std::f32::consts::FRAC_PI_2;

fn open_map() -> GameMinimap {
    // A 200-unit square map centred at 1700,180, 2 UI units per world unit, north up.
    GameMinimap {
        open_state: 2,
        kind: 1,
        centre: [1700.0, 180.0],
        left: 1600.0,
        top: 280.0,
        bottom: 80.0,
        right: 1800.0,
        ui_units_per_world_unit: 2.0,
        rotation_radians: 0.0,
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0e-3
}

#[test]
fn the_partner_at_the_players_place_sits_on_the_centre() {
    let dot = minimap_dot([5.0, 1.0, 5.0], [5.0, 1.0, 5.0], &open_map()).unwrap();
    assert!(close(dot.x, 1700.0) && close(dot.y, 180.0) && !dot.clamped_to_edge);
}

#[test]
fn plus_x_is_right_and_plus_z_is_down_on_a_map_that_does_not_turn() {
    let map = open_map();
    let right = minimap_dot([0.0; 3], [10.0, 0.0, 0.0], &map).unwrap();
    assert!(close(right.x, 1720.0) && close(right.y, 180.0));
    let down = minimap_dot([0.0; 3], [0.0, 0.0, 10.0], &map).unwrap();
    assert!(close(down.x, 1700.0) && close(down.y, 200.0));
}

#[test]
fn the_dot_turns_the_way_the_game_turns_its_icons() {
    let map = GameMinimap {
        rotation_radians: FRAC_PI_2,
        ..open_map()
    };
    // The game's turn about y sends depth to minus x and across to plus y.
    let deeper = minimap_dot([0.0; 3], [0.0, 0.0, 10.0], &map).unwrap();
    assert!(close(deeper.x, 1680.0) && close(deeper.y, 180.0));
    let across = minimap_dot([0.0; 3], [10.0, 0.0, 0.0], &map).unwrap();
    assert!(close(across.x, 1700.0) && close(across.y, 200.0));
}

#[test]
fn the_scale_is_the_games_ui_units_per_world_unit() {
    let map = GameMinimap {
        ui_units_per_world_unit: 0.5,
        ..open_map()
    };
    let dot = minimap_dot([0.0; 3], [40.0, 0.0, 0.0], &map).unwrap();
    assert!(close(dot.x, 1720.0));
}

#[test]
fn height_moves_the_dot_only_in_a_side_view() {
    let map = open_map();
    assert_eq!(
        minimap_dot([0.0; 3], [3.0, 0.0, -4.0], &map),
        minimap_dot([0.0, 50.0, 0.0], [3.0, -20.0, -4.0], &map)
    );
    let side_view = GameMinimap { kind: 2, ..map };
    let higher = minimap_dot([0.0; 3], [0.0, 10.0, 99.0], &side_view).unwrap();
    assert!(close(higher.x, 1700.0) && close(higher.y, 160.0));
}

#[test]
fn a_far_partner_stays_on_the_rim_in_their_direction() {
    let dot = minimap_dot([0.0; 3], [9000.0, 0.0, 0.0], &open_map()).unwrap();
    assert!(dot.clamped_to_edge);
    assert!(close(dot.x, 1700.0 + 100.0 * 0.92) && close(dot.y, 180.0));
}

#[test]
fn the_rim_is_the_circle_the_narrower_side_of_the_rect_holds() {
    let map = GameMinimap {
        top: 230.0,
        bottom: 130.0,
        ..open_map()
    };
    let dot = minimap_dot([0.0; 3], [0.0, 0.0, -9000.0], &map).unwrap();
    assert!(dot.clamped_to_edge && close(dot.y, 180.0 - 50.0 * 0.92));
}

#[test]
fn the_dot_shows_only_while_the_game_opens_or_shows_its_minimap() {
    let map = open_map();
    assert_eq!(minimap_hidden_reason(Some(&map), 16), None);
    let opening = GameMinimap {
        open_state: 1,
        ..map
    };
    assert_eq!(minimap_hidden_reason(Some(&opening), 16), None);
    for state in [0, 3, 4] {
        let shut = GameMinimap {
            open_state: state,
            ..map
        };
        assert!(minimap_hidden_reason(Some(&shut), 16).is_some());
        assert_eq!(minimap_dot([0.0; 3], [1.0, 0.0, 0.0], &shut), None);
    }
}

#[test]
fn a_minimap_the_game_stopped_updating_is_not_on_screen() {
    assert!(minimap_hidden_reason(None, 0).is_some());
    assert!(minimap_hidden_reason(Some(&open_map()), 1000).is_some());
}

#[test]
fn a_minimap_with_no_size_or_scale_draws_no_dot() {
    let released = GameMinimap {
        left: 0.0,
        top: 0.0,
        bottom: 0.0,
        right: 0.0,
        ..open_map()
    };
    assert!(minimap_hidden_reason(Some(&released), 0).is_some());
    let unscaled = GameMinimap {
        ui_units_per_world_unit: f32::NAN,
        ..open_map()
    };
    assert_eq!(minimap_dot([0.0; 3], [1.0, 0.0, 0.0], &unscaled), None);
    assert_eq!(
        minimap_dot([f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], &open_map()),
        None
    );
}

#[test]
fn ui_units_reach_the_display_through_the_camera_viewport() {
    let on_display =
        ui_point_on_display([1700.0, 180.0], [0, 0, 1920, 1080], [3840.0, 2160.0]).unwrap();
    assert!(close(on_display[0], 3400.0) && close(on_display[1], 360.0));
    let letterboxed =
        ui_point_on_display([100.0, 160.0], [0, 60, 1920, 960], [1920.0, 960.0]).unwrap();
    assert!(close(letterboxed[0], 100.0) && close(letterboxed[1], 100.0));
    assert_eq!(
        ui_point_on_display([1.0, 1.0], [0, 0, 0, 1080], [1920.0, 1080.0]),
        None
    );
}

#[test]
fn each_exe_has_its_own_noted_start_of_the_minimap_update() {
    assert_eq!(
        minimap_update_start(ExeVariant::Main),
        &MINIMAP_UPDATE_FUNCTION_MAIN_START[..]
    );
    assert_eq!(
        minimap_update_start(ExeVariant::Compat),
        &MINIMAP_UPDATE_FUNCTION_COMPAT_START[..]
    );
}
