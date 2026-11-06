pub fn marker_label(name: &str, distance: f32, ping_ms: Option<u32>) -> String {
    match ping_ms {
        Some(ping) => format!("{name} \u{b7} {distance:.0} m \u{b7} {ping} ms"),
        None => format!("{name} \u{b7} {distance:.0} m"),
    }
}
