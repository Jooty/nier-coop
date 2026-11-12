#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LobbyPermissions {
    pub change_mode: bool,
    pub change_options: bool,
    pub remove_members: bool,
}

pub fn lobby_permissions(viewer_is_guest: bool) -> LobbyPermissions {
    let host = !viewer_is_guest;
    LobbyPermissions {
        change_mode: host,
        change_options: host,
        remove_members: host,
    }
}

/// Whether a board row offers REMOVE: only to the host, and never on the viewer's own row.
pub fn remove_offered(viewer_is_guest: bool, row_is_own: bool) -> bool {
    lobby_permissions(viewer_is_guest).remove_members && !row_is_own
}
