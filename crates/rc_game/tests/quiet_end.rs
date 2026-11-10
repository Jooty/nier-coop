use rc_game::offsets::{
    ADD_ITEM_NUM_NATIVE_RVA, START_TALK_NATIVE_RVA, START_TALK_UNIQUE_ACTOR_NATIVE_RVA,
};
use rc_game::pickups::{INJECTED_END_TAKE_BLOCK_MS, InjectedEndWindow};
use rc_game::quiet_end::{
    LUA_ARG_STRIDE, LUA_ARGS_OFFSET, QUIET_END_SKIPPED_NATIVES, TalkStartVerdict, lua_arg_as_int,
    lua_arg_offset, skipped_native_name, talk_start_verdict,
};

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

#[test]
fn a_shared_event_end_runs_its_talks_for_real() {
    assert_eq!(
        talk_start_verdict(START_TALK_NATIVE_RVA, true, true),
        TalkStartVerdict::RunSharedEventEnd
    );
    assert!(!talk_start_verdict(START_TALK_UNIQUE_ACTOR_NATIVE_RVA, true, true).skips());
}

#[test]
fn a_non_shared_partner_end_still_skips_its_talks() {
    assert_eq!(
        talk_start_verdict(START_TALK_NATIVE_RVA, true, false),
        TalkStartVerdict::SkipPartnerEnd
    );
    assert!(talk_start_verdict(START_TALK_UNIQUE_ACTOR_NATIVE_RVA, true, false).skips());
}

#[test]
fn without_an_injected_end_a_talk_start_runs() {
    assert_eq!(
        talk_start_verdict(START_TALK_NATIVE_RVA, false, false),
        TalkStartVerdict::RunNoInjectedEnd
    );
    assert_eq!(
        talk_start_verdict(ADD_ITEM_NUM_NATIVE_RVA, true, false),
        TalkStartVerdict::RunNotATalkStart
    );
}

#[test]
fn lua_int_and_number_arguments_decode_to_the_talk_id() {
    let mut int_payload = [0_u8; 8];
    int_payload[..4].copy_from_slice(&2444_i32.to_le_bytes());
    assert_eq!(lua_arg_as_int(4, int_payload), 2444);
    assert_eq!(lua_arg_as_int(1, 1057.0_f64.to_le_bytes()), 1057);
    assert_eq!(lua_arg_as_int(0, [0xFF; 8]), 0);
    assert_eq!(lua_arg_offset(1), LUA_ARGS_OFFSET + LUA_ARG_STRIDE);
}
