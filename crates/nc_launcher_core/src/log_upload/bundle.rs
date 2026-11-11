use crate::log_upload::BUNDLE_TITLE_LINE;

pub struct BundleHeader {
    pub install_id: String,
    pub player_name: String,
    pub role: String,
    pub build_commit: String,
    pub os: String,
    pub launcher_started: String,
}

pub struct BundleFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

pub fn build_bundle(header: &BundleHeader, files: &[BundleFile], tail_cap_bytes: usize) -> String {
    let mut text = String::new();
    text.push_str(BUNDLE_TITLE_LINE);
    text.push('\n');
    push_header_line(&mut text, "install id", &header.install_id);
    push_header_line(&mut text, "player name", &header.player_name);
    push_header_line(&mut text, "role", &header.role);
    push_header_line(&mut text, "build commit", &header.build_commit);
    push_header_line(&mut text, "os", &header.os);
    push_header_line(&mut text, "launcher started", &header.launcher_started);
    for file in files {
        text.push_str(&format!(
            "\n===== {} ({} bytes) =====\n",
            file.name,
            file.bytes.len()
        ));
        let start = file.bytes.len().saturating_sub(tail_cap_bytes);
        text.push_str(&String::from_utf8_lossy(&file.bytes[start..]));
        if !text.ends_with('\n') {
            text.push('\n');
        }
    }
    text
}

fn push_header_line(text: &mut String, label: &str, value: &str) {
    let one_line: String = value
        .chars()
        .map(|letter| if letter.is_control() { ' ' } else { letter })
        .collect();
    text.push_str(&format!("{label}: {one_line}\n"));
}
