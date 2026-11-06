use std::sync::LazyLock;

use rc_proto::{MAX_STORY_FLAG_ID, StoryArray};

static GAME_FLAG_MILESTONES: LazyLock<Vec<u16>> =
    LazyLock::new(|| parse_ids(include_str!("../../data/game_flag_milestones.txt")));
static SNOW_FLAG_MILESTONES: LazyLock<Vec<u16>> =
    LazyLock::new(|| parse_ids(include_str!("../../data/snow_flag_milestones.txt")));

/// Flag ids that the shipped scripts set and never clear, so a set one marks progress made.
/// The lists are data (`crates/rc_game/data/`); counters have none.
pub fn milestone_ids(array: StoryArray) -> &'static [u16] {
    match array {
        StoryArray::GameFlags => &GAME_FLAG_MILESTONES,
        StoryArray::SnowFlags => &SNOW_FLAG_MILESTONES,
        StoryArray::GameCounters | StoryArray::SnowCounters | StoryArray::KeyFragments => &[],
    }
}

fn parse_ids(text: &str) -> Vec<u16> {
    text.split_whitespace()
        .filter_map(|word| word.parse::<u16>().ok())
        .filter(|&id| id <= MAX_STORY_FLAG_ID)
        .collect()
}
