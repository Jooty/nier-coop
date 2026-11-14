use automata_game::partner_hud::{
    Projected, TagCamera, TagInputs, project_tag, shown_name, tag_fade_alpha, tag_hidden_reason,
    tag_label,
};

const WIDTH: f32 = 1600.0;
const HEIGHT: f32 = 900.0;

// Row vectors: translation in the last row; the camera looks down minus z.
fn camera_at(eye: [f32; 3]) -> TagCamera {
    let mut view = [0.0_f32; 16];
    view[0] = 1.0;
    view[5] = 1.0;
    view[10] = 1.0;
    view[15] = 1.0;
    view[12] = -eye[0];
    view[13] = -eye[1];
    view[14] = -eye[2];
    let mut projection = [0.0_f32; 16];
    projection[0] = 1.0;
    projection[5] = 2.0;
    projection[10] = -1.0;
    projection[11] = -1.0;
    projection[14] = -0.1;
    TagCamera { view, projection }
}

fn inputs() -> TagInputs {
    TagInputs {
        enabled: true,
        connected: true,
        partner_state_age_ms: Some(40),
        same_place: true,
        local_in_play: true,
        local_free: true,
        local_paused: false,
    }
}

#[test]
fn a_point_straight_ahead_is_at_the_screen_centre() {
    let projected = project_tag(&camera_at([0.0; 3]), [0.0, 0.0, -10.0], WIDTH, HEIGHT);
    assert_eq!(projected, Projected::OnScreen { x: 800.0, y: 450.0 });
}

#[test]
fn right_of_the_view_is_right_on_the_screen_and_up_is_up() {
    let camera = camera_at([0.0; 3]);
    assert_eq!(
        project_tag(&camera, [5.0, 0.0, -10.0], WIDTH, HEIGHT),
        Projected::OnScreen {
            x: 1200.0,
            y: 450.0
        }
    );
    assert_eq!(
        project_tag(&camera, [0.0, 2.5, -10.0], WIDTH, HEIGHT),
        Projected::OnScreen { x: 800.0, y: 225.0 }
    );
}

#[test]
fn the_camera_position_is_the_last_row_of_the_view() {
    let projected = project_tag(&camera_at([5.0, 0.0, 5.0]), [5.0, 0.0, -5.0], WIDTH, HEIGHT);
    assert_eq!(projected, Projected::OnScreen { x: 800.0, y: 450.0 });
}

#[test]
fn a_point_off_the_side_gets_an_arrow_at_that_edge() {
    let projected = project_tag(&camera_at([0.0; 3]), [30.0, 0.0, -10.0], WIDTH, HEIGHT);
    let Projected::EdgeArrow {
        x,
        y,
        angle_radians,
    } = projected
    else {
        panic!("expected an edge arrow, got {projected:?}");
    };
    assert_eq!((x, y), (1536.0, 450.0));
    assert!(angle_radians.abs() < 1.0e-6);
}

#[test]
fn a_point_above_the_view_gets_an_arrow_that_points_up() {
    let projected = project_tag(&camera_at([0.0; 3]), [0.0, 30.0, -10.0], WIDTH, HEIGHT);
    let Projected::EdgeArrow {
        x,
        y,
        angle_radians,
    } = projected
    else {
        panic!("expected an edge arrow, got {projected:?}");
    };
    assert_eq!((x, y), (800.0, 36.0));
    assert!((angle_radians - core::f32::consts::FRAC_PI_2).abs() < 1.0e-6);
}

#[test]
fn a_point_behind_the_camera_gets_an_arrow_on_the_side_to_turn_to() {
    let camera = camera_at([0.0; 3]);
    let right = project_tag(&camera, [4.0, 0.0, 10.0], WIDTH, HEIGHT);
    let left = project_tag(&camera, [-4.0, 0.0, 10.0], WIDTH, HEIGHT);
    assert!(matches!(right, Projected::EdgeArrow { x, .. } if x > WIDTH * 0.5));
    assert!(matches!(left, Projected::EdgeArrow { x, .. } if x < WIDTH * 0.5));
}

#[test]
fn numbers_that_are_not_finite_draw_nothing() {
    let camera = camera_at([0.0; 3]);
    assert_eq!(
        project_tag(&camera, [f32::NAN, 0.0, -1.0], WIDTH, HEIGHT),
        Projected::Invalid
    );
    assert_eq!(
        project_tag(&camera, [0.0, 0.0, -1.0], 0.0, HEIGHT),
        Projected::Invalid
    );
    let mut broken = camera;
    broken.view = [0.0; 16];
    assert_eq!(
        project_tag(&broken, [0.0, 0.0, -1.0], WIDTH, HEIGHT),
        Projected::Invalid
    );
}

#[test]
fn the_tag_is_drawn_only_while_both_players_share_a_place_in_play() {
    assert_eq!(tag_hidden_reason(&inputs()), None);
    let hidden = |change: fn(&mut TagInputs)| {
        let mut changed = inputs();
        change(&mut changed);
        tag_hidden_reason(&changed)
    };
    assert_eq!(hidden(|i| i.enabled = false), Some("turned off"));
    assert_eq!(hidden(|i| i.connected = false), Some("no partner linked"));
    assert_eq!(
        hidden(|i| i.partner_state_age_ms = None),
        Some("no partner state yet")
    );
    assert_eq!(
        hidden(|i| i.partner_state_age_ms = Some(3001)),
        Some("the partner's state is stale")
    );
    assert_eq!(
        hidden(|i| i.same_place = false),
        Some("the partner is in another place")
    );
    assert_eq!(
        hidden(|i| i.local_paused = true),
        Some("this game is paused or in a menu")
    );
    assert_eq!(
        hidden(|i| i.local_free = false),
        Some("a scene holds this player")
    );
    assert_eq!(
        hidden(|i| i.local_in_play = false),
        Some("this game is not in play")
    );
}

#[test]
fn the_tag_fades_with_distance_but_never_away() {
    assert_eq!(tag_fade_alpha(10.0), 1.0);
    assert!(tag_fade_alpha(130.0) < 1.0);
    assert!((tag_fade_alpha(5000.0) - 0.35).abs() < 1.0e-6);
}

#[test]
fn the_label_is_the_name_and_the_distance_when_asked() {
    assert_eq!(tag_label("Jay", None), "Jay");
    assert_eq!(tag_label("Jay", Some(34.6)), "Jay \u{b7} 35 m");
}

#[test]
fn a_lobby_name_is_shown_without_markup_and_within_a_length() {
    assert_eq!(shown_name("  Jay  "), "Jay");
    assert_eq!(shown_name("<b>J%ay\\\n"), "bJay");
    assert_eq!(shown_name(" \t "), "Partner");
    assert_eq!(shown_name(&"x".repeat(80)).chars().count(), 24);
}

#[test]
fn the_pause_word_is_said_when_it_changes_and_not_for_the_first_reading() {
    use automata_game::event_follow::EventId;
    use automata_game::partner_hud::{PauseWordFacts, PauseWordWatch};
    let facts = || PauseWordFacts {
        pause_state_word: Some(2),
        playing_event: Some(EventId {
            kind: 1,
            id: 2,
            sub: 3,
        }),
        story_place: "0x12 AA".to_owned(),
    };
    let mut watch = PauseWordWatch::default();
    assert!(watch.note(0, false, facts).is_empty());
    assert!(watch.note(10, false, facts).is_empty());
    let said = watch.note(20, true, facts);
    assert_eq!(
        said,
        vec![
            "pause word: the word read as \"paused or in a menu\" went from false to true; pause state word 0x2; an event plays (1/2/3); story place 0x12 AA"
                .to_owned()
        ]
    );
    let back = watch.note(30, false, || PauseWordFacts {
        pause_state_word: None,
        playing_event: None,
        story_place: "0x12 AA".to_owned(),
    });
    assert!(back[0].contains("went from true to false; pause state word not read; no event plays"));
}
