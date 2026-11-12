use nc_wire::{MAX_PLAYERS_PER_SESSION, Member};

#[test]
fn a_session_holds_exactly_two_players() {
    assert_eq!(MAX_PLAYERS_PER_SESSION, 2);
}

#[test]
fn a_member_is_its_slot_then_its_name() {
    let member = Member {
        slot: 1,
        name: "Ka".to_owned(),
    };
    let bytes = postcard::to_stdvec(&member).expect("serializes");
    assert_eq!(bytes, vec![1, 2, b'K', b'a']);
    assert_eq!(postcard::from_bytes::<Member>(&bytes), Ok(member));
}
