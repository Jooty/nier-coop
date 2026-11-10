use crate::load_gate::LOADING_SCREEN_SHOWN_STATE;

/// True when the loading screen is fully shown, its opening part is gone and only the request for
/// the load's end keeps it up. Values that cannot be read count as true, so nothing waits on them.
pub fn loading_screen_awaits_the_request(state: Option<i32>, opening_part: Option<u64>) -> bool {
    match (state, opening_part) {
        (Some(state), Some(opening_part)) => {
            state == LOADING_SCREEN_SHOWN_STATE && opening_part == 0
        }
        _ => true,
    }
}
