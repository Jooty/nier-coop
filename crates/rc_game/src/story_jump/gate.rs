use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryJumpView {
    pub host: bool,
    pub map_manager_active: bool,
    pub free_to_act: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryJumpRefusal {
    NotHost,
    NotInTheField,
    SceneOrTalk,
}

impl fmt::Display for StoryJumpRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NotHost => "only the host can jump",
            Self::NotInTheField => "not in the field yet",
            Self::SceneOrTalk => "wait for the scene or talk to end",
        };
        formatter.write_str(text)
    }
}

pub fn check_story_jump(view: &StoryJumpView) -> Result<(), StoryJumpRefusal> {
    if !view.host {
        return Err(StoryJumpRefusal::NotHost);
    }
    if !view.map_manager_active {
        return Err(StoryJumpRefusal::NotInTheField);
    }
    if !view.free_to_act {
        return Err(StoryJumpRefusal::SceneOrTalk);
    }
    Ok(())
}
