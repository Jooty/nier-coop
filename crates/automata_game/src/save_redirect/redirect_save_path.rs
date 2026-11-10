use crate::save_redirect::{
    COOP_SAVE_FOLDER_SEGMENT, NORMAL_SAVE_FOLDER_SEGMENT, SAVE_PATH_BUFFER_LEN, SaveFileKind,
    SaveRole,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveRedirect {
    Redirected { path: String, folder: String },
    AlreadyRedirected,
    NotASavePath,
    TooLong,
}

pub fn redirect_save_path(path: &str, role: SaveRole) -> SaveRedirect {
    let lowered = path.to_ascii_lowercase();
    if lowered.contains(&COOP_SAVE_FOLDER_SEGMENT.to_ascii_lowercase()) {
        return SaveRedirect::AlreadyRedirected;
    }
    let Some(segment_start) = lowered.find(&NORMAL_SAVE_FOLDER_SEGMENT.to_ascii_lowercase()) else {
        return SaveRedirect::NotASavePath;
    };
    let file_name = &path[segment_start + NORMAL_SAVE_FOLDER_SEGMENT.len()..];
    if SaveFileKind::of_file_name(file_name).is_none() {
        return SaveRedirect::NotASavePath;
    }
    let folder = format!(
        "{}{COOP_SAVE_FOLDER_SEGMENT}{}",
        &path[..segment_start],
        role.folder_name()
    );
    let redirected = format!("{folder}\\{file_name}");
    if redirected.len() >= SAVE_PATH_BUFFER_LEN {
        return SaveRedirect::TooLong;
    }
    SaveRedirect::Redirected {
        path: redirected,
        folder,
    }
}
