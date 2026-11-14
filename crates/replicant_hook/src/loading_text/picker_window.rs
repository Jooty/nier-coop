use replicant_game::loading_lines::PICKER_FRESH_SCREEN_CURSOR;

use crate::loading_text::getter_swap::{is_installed, offer, withdraw};
use crate::loading_text::screen_choice::choose_for_this_screen;

pub(crate) fn offer_for_this_picker_call(cursor: *mut i32) -> bool {
    if !is_installed() {
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "reads the picker's own cursor argument before the picker does"
    )]
    // SAFETY: the picker dereferences this pointer itself on every call; it is a live stack int.
    let fresh_screen =
        unsafe { cursor.as_ref() }.is_some_and(|cursor| *cursor == PICKER_FRESH_SCREEN_CURSOR);
    if !fresh_screen {
        return false;
    }
    let Some(choice) = choose_for_this_screen() else {
        return false;
    };
    offer(&choice.text)
}

pub(crate) fn withdraw_after_picker_call() {
    withdraw();
}
