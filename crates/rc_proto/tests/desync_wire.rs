use rc_proto::{DesyncDigest, GroupRoster, MAX_DESYNC_GROUPS, Message, WireError, decode, encode};

fn digest() -> DesyncDigest {
    DesyncDigest {
        phase: 12,
        position: [1.0, 2.0, 3.0],
        story: Some(77),
        groups: vec![
            GroupRoster {
                group: 3,
                count: 2,
                ids_hash: 9,
            },
            GroupRoster {
                group: 8,
                count: 1,
                ids_hash: 4,
            },
        ],
        groups_not_listed: 0,
    }
}

#[test]
fn a_digest_round_trips_and_is_relayed() {
    let message = Message::Desync(digest());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_full_digest_fits_the_message_limit() {
    let mut full = digest();
    full.story = Some(u64::MAX);
    full.groups = (0..MAX_DESYNC_GROUPS as u16)
        .map(|group| GroupRoster {
            group: 900 + group,
            count: u16::MAX,
            ids_hash: u32::MAX,
        })
        .collect();
    assert!(encode(&Message::Desync(full)).is_ok());
}

#[test]
fn unsorted_or_too_many_groups_are_refused() {
    let mut unsorted = digest();
    unsorted.groups.reverse();
    assert_eq!(
        unsorted.validate(),
        Err(WireError::OutOfRange { field: "groups" })
    );
    let mut many = digest();
    many.groups = (0..=MAX_DESYNC_GROUPS as u16)
        .map(|group| GroupRoster {
            group,
            count: 1,
            ids_hash: 0,
        })
        .collect();
    assert_eq!(
        many.validate(),
        Err(WireError::OutOfRange { field: "groups" })
    );
}

#[test]
fn equal_digests_have_no_differences() {
    assert!(digest().differences(&digest(), None).is_empty());
}

#[test]
fn a_different_phase_is_the_only_difference_reported() {
    let mut partner = digest();
    partner.phase = 13;
    partner.story = Some(1);
    assert_eq!(
        digest().differences(&partner, None),
        vec!["phase (yours 12, partner 13)".to_owned()]
    );
}

#[test]
fn a_story_digest_is_compared_only_when_both_sides_have_one() {
    let mut partner = digest();
    partner.story = None;
    assert!(digest().differences(&partner, None).is_empty());
    partner.story = Some(78);
    let found = digest().differences(&partner, None);
    assert_eq!(found.len(), 1);
    assert!(found[0].starts_with("story digest (yours 000000000000004d, partner"));
}

#[test]
fn an_enemy_group_with_another_count_or_hash_or_missing_is_named() {
    let mut partner = digest();
    partner.groups[0].count = 3;
    partner.groups.pop();
    let found = digest().differences(&partner, None);
    assert_eq!(
        found,
        vec![
            "enemy group 3 (yours 2 live, ids 00000009, partner 3 live, ids 00000009)".to_owned(),
            "enemy group 8 (yours 1 live, ids 00000004, partner none)".to_owned(),
        ]
    );
}

#[test]
fn a_partner_position_far_from_what_was_seen_is_named() {
    let found = digest().differences(&digest(), Some([100.0, 2.0, 3.0]));
    assert_eq!(found.len(), 1);
    assert!(found[0].starts_with("partner position (yours sees 100,2,3, partner reports 1,2,3)"));
    assert!(
        digest()
            .differences(&digest(), Some([4.0, 2.0, 3.0]))
            .is_empty()
    );
}
