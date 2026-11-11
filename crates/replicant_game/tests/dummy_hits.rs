use replicant_game::dummy_hits::{
    DUMMY_CALLBACK_NAME_ARG_INDEX, DummyHitCallbacks, DummyNameRefusal, dummy_callback_name,
    dummy_hit_log_line, dummy_native_args_len,
};
use replicant_game::story_relay::{CallKey, StoryRelayLedger};

const PHASE: u16 = 7;
const CALLBACK: &str = "AttackCount_Talk";
const TEXT_TAG: i32 = 2;

fn args_with_name(tag: i32, name: &[u8]) -> Vec<u8> {
    let mut block = vec![0_u8; dummy_native_args_len()];
    let slot = 0x10 + DUMMY_CALLBACK_NAME_ARG_INDEX * 0x90 + 8;
    block[slot..slot + 4].copy_from_slice(&tag.to_le_bytes());
    block[slot + 0x10..slot + 0x10 + name.len()].copy_from_slice(name);
    block
}

#[test]
fn each_games_own_hits_count_there() {
    let mut callbacks = DummyHitCallbacks::new();
    assert!(callbacks.register(PHASE, CALLBACK));
    let counts: Vec<u32> = (0..12)
        .map(|_| callbacks.count_run(PHASE, CALLBACK))
        .collect();
    assert_eq!(counts, (1..=12).collect::<Vec<u32>>());
}

#[test]
fn putting_the_dummy_again_restarts_its_count() {
    let mut callbacks = DummyHitCallbacks::new();
    callbacks.register(PHASE, CALLBACK);
    callbacks.count_run(PHASE, CALLBACK);
    callbacks.count_run(PHASE, CALLBACK);
    callbacks.register(PHASE, CALLBACK);
    assert_eq!(callbacks.count_run(PHASE, CALLBACK), 1);
}

#[test]
fn a_callback_is_a_dummy_hit_only_in_the_phase_it_was_put() {
    let mut callbacks = DummyHitCallbacks::new();
    callbacks.register(PHASE, CALLBACK);
    assert!(callbacks.is_registered(PHASE, CALLBACK));
    assert!(!callbacks.is_registered(PHASE + 1, CALLBACK));
    assert!(!callbacks.is_registered(PHASE, "Other_Callback"));
    assert_eq!(callbacks.count_run(PHASE + 1, CALLBACK), 0);
}

#[test]
fn the_registry_stops_growing_at_its_cap() {
    let mut callbacks = DummyHitCallbacks::new();
    for index in 0..replicant_game::dummy_hits::MAX_DUMMY_CALLBACKS {
        assert!(callbacks.register(PHASE, &format!("Callback_{index}")));
    }
    assert!(!callbacks.register(PHASE, "One_Too_Many"));
    assert!(callbacks.register(PHASE, "Callback_0"));
}

#[test]
fn the_log_line_names_the_callback_and_the_count_of_this_game() {
    assert_eq!(
        dummy_hit_log_line(CALLBACK, 4),
        "dummy hit: AttackCount_Talk ran here, runs here 4"
    );
}

#[test]
fn the_callback_name_is_read_from_the_third_native_argument() {
    let block = args_with_name(TEXT_TAG, CALLBACK.as_bytes());
    assert_eq!(dummy_callback_name(&block), Ok(CALLBACK.to_owned()));
}

#[test]
fn an_unreadable_callback_name_is_refused_with_its_reason() {
    assert_eq!(
        dummy_callback_name(&args_with_name(4, CALLBACK.as_bytes())),
        Err(DummyNameRefusal::NotText(4))
    );
    assert_eq!(
        dummy_callback_name(&args_with_name(TEXT_TAG, b"")),
        Err(DummyNameRefusal::Empty)
    );
    assert_eq!(
        dummy_callback_name(&[0_u8; 8]),
        Err(DummyNameRefusal::Truncated)
    );
}

#[test]
fn injected_dummy_hits_are_matched_by_pointer_and_leave_the_dedupe_lists_alone() {
    let mut ledger = StoryRelayLedger::new();
    let key = CallKey {
        function: 1,
        args: 2,
    };
    ledger.note_injected(0x1000, key, 0);
    ledger.note_injected(0x2000, key, 0);
    assert!(!ledger.take_injected_by_pointer(0x3000, 1));
    assert!(ledger.take_injected_by_pointer(0x2000, 1));
    assert!(ledger.take_injected_by_pointer(0x1000, 1));
    assert!(!ledger.take_injected_by_pointer(0x1000, 1));
    assert!(ledger.is_idle());
}
