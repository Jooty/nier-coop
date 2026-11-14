use crate::character::Character;
use crate::guest_watches::WatchedStretch;
use crate::load_gate::StoryPlace;

/// The hero a start writes when it goes to `target`: the host's character where the guest
/// will watch, none (the start's own choice stays) everywhere else.
pub fn hero_written_for_a_start(
    guest_watches_in_this_session: bool,
    stretch: &WatchedStretch,
    target: &StoryPlace,
    host: Character,
) -> Option<Character> {
    (guest_watches_in_this_session && stretch.holds(target)).then_some(host)
}
