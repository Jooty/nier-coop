/// What the setState detour does with a state change: let the game write it, or refuse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepVerdict {
    Pass,
    Refuse,
}
