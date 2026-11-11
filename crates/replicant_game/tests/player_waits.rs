use replicant_game::fight_sync::{
    GuestFightStream, PLAYER_WAIT_BEHIND_STREAM_GRACE_MS, PlayerWaitNative,
    format_player_wait_release, player_wait_ends,
};

#[test]
fn a_player_wait_ends_once_a_host_entry_waited_past_the_grace() {
    assert!(!player_wait_ends(
        true,
        Some(PLAYER_WAIT_BEHIND_STREAM_GRACE_MS)
    ));
    assert!(player_wait_ends(
        true,
        Some(PLAYER_WAIT_BEHIND_STREAM_GRACE_MS + 1)
    ));
}

#[test]
fn a_player_wait_with_no_entry_behind_or_already_over_never_ends() {
    assert!(!player_wait_ends(true, None));
    assert!(!player_wait_ends(false, Some(60_000)));
}

#[test]
fn nothing_waits_behind_the_slot_outside_a_joined_fight() {
    let stream = GuestFightStream::new();
    assert_eq!(stream.entry_behind_ms(Some(3), 60_000), None);
    assert_eq!(stream.entry_behind_ms(None, 60_000), None);
}

#[test]
fn both_player_waits_have_their_own_poll() {
    let polls: Vec<u32> = PlayerWaitNative::ALL
        .into_iter()
        .map(PlayerWaitNative::poll_rva)
        .collect();
    assert_eq!(polls, vec![0x70_C310, 0x70_C4C0]);
}

#[test]
fn the_player_wait_line_names_the_handler_and_the_native() {
    assert_eq!(
        format_player_wait_release(PlayerWaitNative::OnGround, "EID_1050_a0050_CA_Timer", 3_016),
        "player wait: ended your wait in EID_1050_a0050_CA_Timer (the host moved on, _WaitPlayerOnGround, a host entry waited 3016 ms behind it)"
    );
}
