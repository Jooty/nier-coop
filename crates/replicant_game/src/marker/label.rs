pub fn marker_label(name: &str, distance: Option<f32>, ping_ms: Option<u32>) -> String {
    let mut label = name.to_owned();
    if let Some(distance) = distance {
        label.push_str(&format!(" \u{b7} {distance:.0} m"));
    }
    if let Some(ping) = ping_ms {
        label.push_str(&format!(" \u{b7} {ping} ms"));
    }
    label
}
