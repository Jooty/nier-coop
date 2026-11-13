use replicant_game::callback_gate::{control_slots_free, slots_an_end_waits_on};
use replicant_game::dialogue::{
    HELD_TALK_START_WAITS_FOR_THE_PARTNER_MS, HeldTalkRelease, HeldTalkStart, HeldTalkStep,
    WATCHER_HOLD_MASK,
};

const FIRST_TALK: u32 = 1223;
const SECOND_TALK: u32 = 1224;
const NATIVE_BLOCK: u64 = 0x7FF6_1234_5000;
const GUEST_MASK_DURING_THE_MIRROR: [u32; 4] = [0x0, WATCHER_HOLD_MASK, 0x0, 0x0];
const GUEST_MASK_IN_ITS_OWN_TALK: [u32; 4] = [0x80, 0xA0, 0x0, 0x0];

#[test]
fn the_end_of_the_hosts_step_does_not_wait_on_the_hold_of_its_own_talk() {
    let held = slots_an_end_waits_on(GUEST_MASK_DURING_THE_MIRROR, true);
    assert!(control_slots_free(held, false));
    let not_held = slots_an_end_waits_on(GUEST_MASK_DURING_THE_MIRROR, false);
    assert!(!control_slots_free(not_held, false));
}

#[test]
fn a_menu_or_pause_bit_of_the_guests_own_still_holds_the_end() {
    let held = slots_an_end_waits_on(GUEST_MASK_IN_ITS_OWN_TALK, true);
    assert!(!control_slots_free(held, false));
}

#[test]
fn the_copys_talk_start_waits_until_the_hosts_talk_is_over() {
    // Run 1113k: the host's talk 1223 ran 17:17:43.84 to 45.07.
    let mut held = HeldTalkStart::new();
    held.begin(NATIVE_BLOCK, FIRST_TALK, 44_100);
    assert_eq!(held.holds(NATIVE_BLOCK), Some(FIRST_TALK));
    assert_eq!(held.poll(true, false, 44_116), HeldTalkStep::Wait);
    assert_eq!(held.poll(true, false, 45_050), HeldTalkStep::Wait);
    assert_eq!(
        held.poll(false, true, 45_085),
        HeldTalkStep::Release(HeldTalkRelease::PartnerFinished)
    );
    assert_eq!(held.holds(NATIVE_BLOCK), None);
}

#[test]
fn the_next_talk_start_waits_for_the_host_to_open_it() {
    // The host opened talk 1224 545 ms after its talk 1223 ended.
    let mut held = HeldTalkStart::new();
    held.begin(NATIVE_BLOCK, SECOND_TALK, 45_100);
    assert_eq!(held.poll(false, false, 45_400), HeldTalkStep::Wait);
    assert_eq!(held.poll(true, false, 45_646), HeldTalkStep::Wait);
    assert_eq!(
        held.poll(false, true, 47_585),
        HeldTalkStep::Release(HeldTalkRelease::PartnerFinished)
    );
}

#[test]
fn a_copy_that_comes_after_the_host_ended_the_talk_goes_on_at_once() {
    let mut held = HeldTalkStart::new();
    held.begin(NATIVE_BLOCK, FIRST_TALK, 47_753);
    assert_eq!(
        held.poll(false, true, 47_753),
        HeldTalkStep::Release(HeldTalkRelease::PartnerFinished)
    );
}

#[test]
fn a_talk_the_host_never_shows_lets_the_copy_go_on_after_the_wait() {
    let mut held = HeldTalkStart::new();
    held.begin(NATIVE_BLOCK, FIRST_TALK, 10_000);
    let before = 10_000 + HELD_TALK_START_WAITS_FOR_THE_PARTNER_MS - 1;
    assert_eq!(held.poll(false, false, before), HeldTalkStep::Wait);
    assert_eq!(
        held.poll(false, false, before + 1),
        HeldTalkStep::Release(HeldTalkRelease::PartnerNeverShowed)
    );
}

#[test]
fn another_native_block_is_not_the_held_start() {
    let mut held = HeldTalkStart::new();
    held.begin(NATIVE_BLOCK, FIRST_TALK, 0);
    assert_eq!(held.holds(NATIVE_BLOCK + 0x40), None);
    held.forget();
    assert_eq!(held.holds(NATIVE_BLOCK), None);
}
