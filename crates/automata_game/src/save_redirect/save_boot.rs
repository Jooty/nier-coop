#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveBoot {
    UnknownExe,
    Redirected,
    NotRedirected,
}

/// Only an exe this mod knows and could not redirect is stopped; an unknown exe is the plain game.
pub fn game_may_start(boot: SaveBoot) -> bool {
    boot != SaveBoot::NotRedirected
}
