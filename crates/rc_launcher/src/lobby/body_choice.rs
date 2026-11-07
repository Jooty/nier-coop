pub fn body_choice_shown(hosting: bool, connected: bool, guest: bool) -> bool {
    !hosting && !(connected && !guest)
}
