pub fn cell_of(position: [f32; 3]) -> [i32; 3] {
    position.map(|axis| (axis + 0.5).floor() as i32)
}

pub fn distance_between(from: [f32; 3], to: [f32; 3]) -> f32 {
    (to[0] - from[0])
        .hypot(to[1] - from[1])
        .hypot(to[2] - from[2])
}
