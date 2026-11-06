use crate::puppet::LayerMotion;

/// One enemy as this machine sees it in one scan. `actor` and `spawn_serial` are local to this
/// machine and never leave it; the roster turns them into a network id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemySighting {
    pub actor: u64,
    pub spawn_serial: i32,
    pub group: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<LayerMotion>,
    pub hp: i32,
    pub max_hp: i32,
}
