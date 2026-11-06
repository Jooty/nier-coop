use std::path::Path;

pub fn path_as_the_game_sees_it(path: &Path) -> String {
    let text = path.display().to_string();
    if cfg!(windows) || !text.starts_with('/') {
        return text;
    }
    format!("Z:{}", text.replace('/', "\\"))
}
