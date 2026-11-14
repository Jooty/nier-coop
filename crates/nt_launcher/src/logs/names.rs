use crate::app::Role;

pub fn game_log_name(role: Role) -> &'static str {
    match role {
        Role::Host => "game-host.log",
        Role::Guest => "game-guest.log",
    }
}

pub fn launcher_log_name(role: Role) -> &'static str {
    match role {
        Role::Host => "launcher-host.log",
        Role::Guest => "launcher-guest.log",
    }
}
