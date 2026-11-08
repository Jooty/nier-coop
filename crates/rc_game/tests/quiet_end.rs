use rc_game::offsets::{ADD_ITEM_NUM_NATIVE_RVA, START_TALK_NATIVE_RVA};
use rc_game::pickups::{INJECTED_END_TAKE_BLOCK_MS, InjectedEndWindow};
use rc_game::quiet_end::{QUIET_END_SKIPPED_NATIVES, skipped_native_name};

const OPENED_AT_MS: u64 = 10_000;

fn open_window() -> InjectedEndWindow {
    let mut window = InjectedEndWindow::new();
    window.open(OPENED_AT_MS);
    window
}

#[test]
fn every_talk_start_native_is_skipped_inside_the_window() {
    let window = open_window();
    for native in QUIET_END_SKIPPED_NATIVES {
        assert_eq!(
            skipped_native_name(&window, OPENED_AT_MS + 1, native.rva),
            Some(native.name)
        );
    }
}

#[test]
fn nothing_is_skipped_before_the_window_opens_or_after_it_closes() {
    let closed = InjectedEndWindow::new();
    assert_eq!(skipped_native_name(&closed, 5, START_TALK_NATIVE_RVA), None);
    let window = open_window();
    let after = OPENED_AT_MS + INJECTED_END_TAKE_BLOCK_MS;
    assert_eq!(
        skipped_native_name(&window, after, START_TALK_NATIVE_RVA),
        None
    );
}

#[test]
fn gifts_are_never_on_the_skip_list() {
    let window = open_window();
    assert_eq!(
        skipped_native_name(&window, OPENED_AT_MS + 1, ADD_ITEM_NUM_NATIVE_RVA),
        None
    );
}
