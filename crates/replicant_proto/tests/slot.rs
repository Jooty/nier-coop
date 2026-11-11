use replicant_proto::PlayerSlot;

#[test]
fn only_slot_zero_is_the_host() {
    assert!(PlayerSlot(0).is_host());
    assert!(!PlayerSlot(1).is_host());
}
