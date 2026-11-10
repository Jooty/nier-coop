use crate::address::{KEYBOARD_INPUT_SOURCE, MOUSE_INPUT_SOURCE, PAD_INPUT_SOURCE};
use crate::exe::ExeVariant;
use crate::input_block::{
    KEYBOARD_SOURCE_LEN, MOUSE_SOURCE_LEN, PAD_SOURCE_LEN, POLL_MOUSE_INACTIVE_VALUE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub rva: u32,
    pub len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBlockPlan {
    pub zero_after_poll_devices: Vec<SourceSpan>,
    pub mouse_active_override: Option<i32>,
    pub skip_cursor_recentre: bool,
}

pub fn input_block_plan(menu_open: bool, variant: ExeVariant) -> InputBlockPlan {
    if !menu_open {
        return InputBlockPlan {
            zero_after_poll_devices: Vec::new(),
            mouse_active_override: None,
            skip_cursor_recentre: false,
        };
    }
    InputBlockPlan {
        zero_after_poll_devices: vec![
            SourceSpan {
                rva: PAD_INPUT_SOURCE.rva(variant),
                len: PAD_SOURCE_LEN,
            },
            SourceSpan {
                rva: KEYBOARD_INPUT_SOURCE.rva(variant),
                len: KEYBOARD_SOURCE_LEN,
            },
            SourceSpan {
                rva: MOUSE_INPUT_SOURCE.rva(variant),
                len: MOUSE_SOURCE_LEN,
            },
        ],
        mouse_active_override: Some(POLL_MOUSE_INACTIVE_VALUE),
        skip_cursor_recentre: true,
    }
}
