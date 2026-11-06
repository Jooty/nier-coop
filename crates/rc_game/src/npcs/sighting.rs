use crate::npcs::NpcEvent;
use crate::puppet::LayerMotion;

/// One NPC as this machine sees it in one scan. `actor` and `spawn_serial` are local to this
/// machine and never leave it; the roster turns them into a network id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NpcSighting {
    pub actor: u64,
    pub spawn_serial: i32,
    pub group: u16,
    pub class: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<LayerMotion>,
    pub visible: bool,
    pub event: Option<NpcEvent>,
}
