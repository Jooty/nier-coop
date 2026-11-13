/// Whether the launcher goes back to home this frame: the lobby was open and is not any more, and no boot
/// console is holding a failure on screen.
pub fn lobby_closed_goes_home(was_open: bool, open_now: bool, console_shown: bool) -> bool {
    was_open && !open_now && !console_shown
}
