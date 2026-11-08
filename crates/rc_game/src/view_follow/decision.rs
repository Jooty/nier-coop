use crate::view_follow::{VIEW_RESEND_MS, view_chain};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorView {
    pub chain: String,
    pub inside: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewFollow {
    Nothing,
    Enter { chain: String },
    Leave { chain: String },
}

#[derive(Debug, Default)]
pub struct ViewAnnouncer {
    last: Option<(DoorView, u64)>,
}

impl ViewAnnouncer {
    pub const fn new() -> Self {
        Self { last: None }
    }

    pub fn reset(&mut self) {
        self.last = None;
    }

    pub fn is_due(&mut self, current: Option<&DoorView>, now_ms: u64) -> bool {
        let Some(current) = current else {
            return false;
        };
        let due = match &self.last {
            Some((sent, at)) => sent != current || now_ms.saturating_sub(*at) >= VIEW_RESEND_MS,
            None => true,
        };
        if due {
            self.last = Some((current.clone(), now_ms));
        }
        due
    }
}

pub fn decide_view_follow(
    host: Option<&DoorView>,
    own: Option<&DoorView>,
    scene_moved_player: bool,
) -> ViewFollow {
    if !scene_moved_player {
        return ViewFollow::Nothing;
    }
    let own_inside = own.filter(|view| view.inside);
    match host {
        Some(host) if host.inside => {
            let already_there = own_inside.is_some_and(|view| view.chain == host.chain);
            if already_there || view_chain(&host.chain).is_none() {
                ViewFollow::Nothing
            } else {
                ViewFollow::Enter {
                    chain: host.chain.clone(),
                }
            }
        }
        _ => match own_inside {
            Some(view) if view_chain(&view.chain).is_some() => ViewFollow::Leave {
                chain: view.chain.clone(),
            },
            _ => ViewFollow::Nothing,
        },
    }
}
