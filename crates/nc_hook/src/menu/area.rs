use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaChangeRefusal {
    NotHost,
    AlreadyThere,
    SceneRunning,
    TalkRunning,
    PopUpRunning,
    Other(String),
}

impl fmt::Display for AreaChangeRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHost => write!(formatter, "only the host may change the area"),
            Self::AlreadyThere => write!(formatter, "you are already in that area"),
            Self::SceneRunning => write!(formatter, "refused while a scene runs"),
            Self::TalkRunning => write!(formatter, "refused while a talk runs"),
            Self::PopUpRunning => write!(formatter, "refused while a pop-up runs"),
            Self::Other(reason) => write!(formatter, "{reason}"),
        }
    }
}
