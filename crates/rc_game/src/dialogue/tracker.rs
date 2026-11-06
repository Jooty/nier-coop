use rc_proto::{DialogueLine, DialogueUpdate};

use crate::dialogue::TalkCursor;

/// Decides what to tell the partner about this player's talk window: a line when it changes,
/// `Ended` when the window closes, and the current line again after the partner (re)joins.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DialogueTracker {
    resolved_talk: Option<(u64, Option<u32>)>,
    known_to_partner: Option<DialogueLine>,
}

impl DialogueTracker {
    pub const fn new() -> Self {
        Self {
            resolved_talk: None,
            known_to_partner: None,
        }
    }

    /// `find_talk_id` runs only when the window starts a different talk.
    pub fn observe(
        &mut self,
        cursor: Option<TalkCursor>,
        partner_present: bool,
        find_talk_id: impl FnOnce(u64) -> Option<u32>,
    ) -> Option<DialogueUpdate> {
        let current = cursor.and_then(|cursor| {
            let talk_id = match self.resolved_talk {
                Some((address, talk_id)) if address == cursor.talk_address => talk_id,
                _ => {
                    let talk_id = find_talk_id(cursor.talk_address);
                    self.resolved_talk = Some((cursor.talk_address, talk_id));
                    talk_id
                }
            };
            talk_id.map(|talk_id| DialogueLine {
                talk_id,
                line: cursor.line,
            })
        });
        if cursor.is_none() {
            self.resolved_talk = None;
        }
        if !partner_present {
            self.known_to_partner = None;
            return None;
        }
        if current == self.known_to_partner {
            return None;
        }
        self.known_to_partner = current;
        Some(current.map_or(DialogueUpdate::Ended, DialogueUpdate::Line))
    }
}
