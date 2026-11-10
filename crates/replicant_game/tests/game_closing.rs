use replicant_game::game_closing::{WM_CLOSE, WM_DESTROY, actors_to_destroy, starts_closing};

const WM_PAINT: u32 = 0x000F;

#[test]
fn closing_and_destroying_the_window_start_the_close() {
    assert!(starts_closing(WM_CLOSE));
    assert!(starts_closing(WM_DESTROY));
    assert!(!starts_closing(WM_PAINT));
}

#[test]
fn a_closing_game_destroys_no_guarded_actor() {
    assert!(actors_to_destroy(true, vec![0x10, 0x20]).is_empty());
}

#[test]
fn a_running_game_destroys_every_guarded_actor() {
    assert_eq!(
        actors_to_destroy(false, vec![0x10, 0, 0x20]),
        vec![0x10, 0x20]
    );
}
