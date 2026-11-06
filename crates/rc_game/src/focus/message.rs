pub const WM_ACTIVATE: u32 = 0x0006;
const ACTIVATION_STATE_MASK: usize = 0xFFFF;
const WA_INACTIVE: usize = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivateKind {
    NotActivation,
    Lost,
    Gained,
}

pub fn classify_activate(message: u32, wparam: usize) -> ActivateKind {
    if message != WM_ACTIVATE {
        return ActivateKind::NotActivation;
    }
    if wparam & ACTIVATION_STATE_MASK == WA_INACTIVE {
        ActivateKind::Lost
    } else {
        ActivateKind::Gained
    }
}
