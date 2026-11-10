use automata_proto::{Message, PathInfo};

use crate::bridge::{BridgeMessage, LauncherPayload};

impl BridgeMessage for Message {
    fn kind_name(&self) -> &'static str {
        Message::kind_name(self)
    }

    fn path_info(rtt_micros: u32, direct: bool) -> Self {
        Message::PathInfo(PathInfo { rtt_micros, direct })
    }

    fn from_launcher(_payload: LauncherPayload) -> Option<Self> {
        None
    }

    fn into_launcher(self) -> Result<LauncherPayload, Self> {
        Err(self)
    }
}
