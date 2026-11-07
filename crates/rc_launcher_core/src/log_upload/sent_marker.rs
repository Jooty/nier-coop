pub fn parse_sent_marker(text: &str) -> Option<u64> {
    text.trim().parse().ok()
}

pub fn render_sent_marker(sent_at_secs: u64) -> String {
    format!("{sent_at_secs}\n")
}
