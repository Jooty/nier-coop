use crate::save_redirect::{
    COOP_SAVE_FOLDER_SEGMENT, NORMAL_SAVE_FOLDER_SEGMENT, SAVE_PATH_BUFFER_LEN, SaveFileKind,
    SaveRole,
};

const FOLDER_LEVELS_BELOW_DOCUMENTS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveRedirect {
    Redirected { path: Vec<u8>, folder: Vec<u8> },
    AlreadyRedirected,
    NotASavePath,
    TooLong,
}

/// The path is in the system's ANSI code page, so it is matched and spliced as bytes.
pub fn redirect_save_path(path: &[u8], role: SaveRole) -> SaveRedirect {
    let lowered = path.to_ascii_lowercase();
    if find(
        &lowered,
        COOP_SAVE_FOLDER_SEGMENT.to_ascii_lowercase().as_bytes(),
    )
    .is_some()
    {
        return SaveRedirect::AlreadyRedirected;
    }
    let Some(segment_start) = find(
        &lowered,
        NORMAL_SAVE_FOLDER_SEGMENT.to_ascii_lowercase().as_bytes(),
    ) else {
        return SaveRedirect::NotASavePath;
    };
    let file_name = &path[segment_start + NORMAL_SAVE_FOLDER_SEGMENT.len()..];
    let is_save_file = core::str::from_utf8(file_name)
        .ok()
        .and_then(SaveFileKind::of_file_name)
        .is_some();
    if !is_save_file {
        return SaveRedirect::NotASavePath;
    }
    let mut folder = path[..segment_start].to_vec();
    folder.extend_from_slice(COOP_SAVE_FOLDER_SEGMENT.as_bytes());
    folder.extend_from_slice(role.folder_name().as_bytes());
    let mut redirected = folder.clone();
    redirected.push(b'\\');
    redirected.extend_from_slice(file_name);
    if redirected.len() >= SAVE_PATH_BUFFER_LEN {
        return SaveRedirect::TooLong;
    }
    SaveRedirect::Redirected {
        path: redirected,
        folder,
    }
}

/// The folders to create, outermost first: `My Games`, the co-op folder, the role folder.
pub fn coop_folder_levels(folder: &[u8]) -> Vec<Vec<u8>> {
    let mut ends: Vec<usize> = folder
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, byte)| **byte == b'\\')
        .map(|(index, _)| index)
        .take(FOLDER_LEVELS_BELOW_DOCUMENTS - 1)
        .collect();
    ends.reverse();
    ends.push(folder.len());
    ends.into_iter().map(|end| folder[..end].to_vec()).collect()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
