#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoryRule {
    #[default]
    Hold,
    Allow,
}

impl StoryRule {
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "hold" => Some(Self::Hold),
            "allow" => Some(Self::Allow),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Allow => "allow",
        }
    }
}

pub fn story_status_line(rule: StoryRule, gate_armed: bool) -> String {
    match (rule, gate_armed) {
        (StoryRule::Hold, true) => "story: held (scene changes that are not ours are refused)",
        (StoryRule::Hold, false) => "story: hold is set, the gate is not armed",
        (StoryRule::Allow, true) => "story: allowed (scene changes are only logged)",
        (StoryRule::Allow, false) => "story: allow is set, the gate is not armed",
    }
    .to_owned()
}
