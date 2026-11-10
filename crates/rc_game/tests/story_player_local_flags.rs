use rc_game::fight_sync::TRAP_PRIVATE_GAME_FLAGS;
use rc_game::story_progress::{
    FeedSend, STORY_DRIFT_GRACE_MS, StoryArrays, StoryFeed, StoryKey, StoryMirror,
    is_player_local_flag, without_player_local_flags,
};
use rc_proto::{StoryArray, StoryChange};

const SIDE_VIEW_FLAG: u16 = 1452;
const NORMAL_FLAG: u16 = 1451;
const DOCS_COPY: &str = include_str!("../data/fight-sync.toml");

fn flag(id: u16, on: bool) -> StoryChange {
    StoryChange::Flag {
        array: StoryArray::GameFlags,
        id,
        on,
    }
}

fn with(changes: &[StoryChange]) -> StoryArrays {
    let mut arrays = StoryArrays::default();
    for &change in changes {
        arrays.apply(change);
    }
    arrays
}

fn sent_changes(send: FeedSend) -> Vec<StoryChange> {
    match send {
        FeedSend::Nothing => Vec::new(),
        FeedSend::Changes(batches) => batches.into_iter().flat_map(|b| b.changes).collect(),
        FeedSend::WholeArrays(_) => panic!("a few changes are not sent whole"),
    }
}

fn joined_mirror(host: &StoryArrays) -> StoryMirror {
    let mut feed = StoryFeed::default();
    let mut mirror = StoryMirror::default();
    for part in feed.whole(host) {
        assert!(mirror.receive_part(&part));
    }
    assert!(mirror.is_complete());
    mirror
}

#[test]
fn the_player_local_flags_are_the_generated_trap_private_flags() {
    let doc: toml::Table = toml::from_str(DOCS_COPY).expect("the docs copy parses");
    let listed: Vec<u16> = doc["trap_private_flags"]
        .as_array()
        .expect("trap-private flags")
        .iter()
        .map(|v| u16::try_from(v.as_integer().expect("a flag")).expect("a flag id"))
        .collect();
    assert_eq!(listed, TRAP_PRIVATE_GAME_FLAGS);
    assert!(TRAP_PRIVATE_GAME_FLAGS.contains(&SIDE_VIEW_FLAG));
    assert!(!TRAP_PRIVATE_GAME_FLAGS.contains(&NORMAL_FLAG));
}

#[test]
fn only_game_flags_of_the_list_are_player_local() {
    let key = |array, id| StoryKey { array, id };
    assert!(is_player_local_flag(key(
        StoryArray::GameFlags,
        SIDE_VIEW_FLAG
    )));
    assert!(!is_player_local_flag(key(
        StoryArray::GameFlags,
        NORMAL_FLAG
    )));
    assert!(!is_player_local_flag(key(
        StoryArray::SnowFlags,
        SIDE_VIEW_FLAG
    )));
    assert!(!is_player_local_flag(key(StoryArray::GameCounters, 20)));
}

#[test]
fn the_host_never_sends_a_player_local_flag_whole_or_as_a_change() {
    let mut feed = StoryFeed::default();
    let parts = feed.whole(&with(&[
        flag(SIDE_VIEW_FLAG, true),
        flag(NORMAL_FLAG, true),
    ]));
    let mut received = StoryArrays::default();
    for part in &parts {
        assert!(received.set_bytes(part.array, &part.bytes));
    }
    assert!(!received.flag(StoryArray::GameFlags, SIDE_VIEW_FLAG));
    assert!(received.flag(StoryArray::GameFlags, NORMAL_FLAG));
    let side_view_cleared = with(&[flag(NORMAL_FLAG, true)]);
    assert_eq!(sent_changes(feed.next(&side_view_cleared)), Vec::new());
    let normal_cleared = with(&[flag(SIDE_VIEW_FLAG, true)]);
    assert_eq!(
        sent_changes(feed.next(&normal_cleared)),
        vec![flag(NORMAL_FLAG, false)]
    );
}

#[test]
fn the_guest_ignores_a_player_local_flag_it_receives_and_keeps_its_own() {
    let host = with(&[flag(SIDE_VIEW_FLAG, true)]);
    let mut mirror = StoryMirror::default();
    for part in [
        StoryArray::GameFlags,
        StoryArray::SnowFlags,
        StoryArray::GameCounters,
        StoryArray::SnowCounters,
        StoryArray::KeyFragments,
    ]
    .map(|array| rc_proto::StoryArrayPart {
        array,
        bytes: host.bytes(array).to_vec(),
    }) {
        assert!(mirror.receive_part(&part));
    }
    let host_copy = mirror.host().expect("complete");
    assert!(!host_copy.flag(StoryArray::GameFlags, SIDE_VIEW_FLAG));
    assert!(mirror.receive_changes(&[flag(SIDE_VIEW_FLAG, true)]));
    assert!(
        !mirror
            .host()
            .expect("complete")
            .flag(StoryArray::GameFlags, SIDE_VIEW_FLAG)
    );
    let own = with(&[flag(SIDE_VIEW_FLAG, true)]);
    let plan = mirror.plan(&own, STORY_DRIFT_GRACE_MS * 10);
    assert!(plan.fixes.is_empty(), "{:?}", plan.fixes);
    assert_eq!(plan.held, 0);
}

#[test]
fn a_normal_flag_still_follows_the_host() {
    let mut mirror = joined_mirror(&with(&[flag(NORMAL_FLAG, true)]));
    assert!(mirror.receive_changes(&[flag(SIDE_VIEW_FLAG, true)]));
    let plan = mirror.plan(&StoryArrays::default(), 0);
    assert_eq!(plan.fixes, vec![flag(NORMAL_FLAG, true)]);
}

#[test]
fn clearing_the_player_local_flags_leaves_every_other_flag() {
    let arrays = with(&[flag(SIDE_VIEW_FLAG, true), flag(NORMAL_FLAG, true)]);
    let shared = without_player_local_flags(&arrays);
    assert!(!shared.flag(StoryArray::GameFlags, SIDE_VIEW_FLAG));
    assert!(shared.flag(StoryArray::GameFlags, NORMAL_FLAG));
    assert_eq!(
        without_player_local_flags(&shared).digest(),
        shared.digest()
    );
}
