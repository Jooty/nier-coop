mod frame;
mod hooks;

pub use frame::{
    FrameSource, OFFSET_AUTO, camera_offset, camera_stats, frame_for_marker, log_summary_if_due,
    note_marker_source, note_position_source, set_camera_offset, set_tag_sync_uses_drawn_frame,
    stats_text, tag_sync_uses_drawn_frame,
};
pub use hooks::install;
