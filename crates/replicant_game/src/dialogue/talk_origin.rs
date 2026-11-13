use crate::dialogue::SCRIPT_TALK_START_OPENS_THE_WINDOW_WITHIN_MS;
use crate::story_calls::is_event_callback_name;
use replicant_proto::DialogueUpdate;

/// How a talk opened on this player's game. Only a story step's talk is shown to the partner;
/// a talk the player started, or one that cannot be classed, stays the player's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TalkOrigin {
    TalkCommand,
    /// Opened by a story event callback (`_Start` or `_End`) with no talk command before it.
    StoryStep(String),
    /// The script call that started it, if a talk start native ran just before the window.
    NotClassed(Option<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScriptTalkStart {
    call: String,
    at_ms: u64,
}

/// Classes each talk this player's window opens, once, from the last talk start native the
/// game ran and whether the talk command came before it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TalkOriginTracker {
    last_start: Option<ScriptTalkStart>,
    talk: Option<(u32, TalkOrigin)>,
}

impl TalkOrigin {
    pub const fn is_story_step(&self) -> bool {
        matches!(self, Self::StoryStep(_))
    }

    pub fn describe(&self) -> String {
        match self {
            Self::TalkCommand => {
                "your own (started with the talk command), the partner gets only its result"
                    .to_owned()
            }
            Self::StoryStep(call) => format!("a story step ({call}), the partner is shown it too"),
            Self::NotClassed(Some(call)) => {
                format!("not classed (script call {call} is not a story event), kept as your own")
            }
            Self::NotClassed(None) => {
                "not classed (no talk start native just before it), kept as your own".to_owned()
            }
        }
    }
}

impl TalkOriginTracker {
    pub const fn new() -> Self {
        Self {
            last_start: None,
            talk: None,
        }
    }

    /// A talk start native ran (and was not skipped) inside the script call `call`.
    pub fn note_script_talk_start(&mut self, call: &str, now_ms: u64) {
        self.last_start = Some(ScriptTalkStart {
            call: call.to_owned(),
            at_ms: now_ms,
        });
    }

    /// Once per tick with the talk this player's own window shows. Returns the class of a talk
    /// seen for the first time; a talk chained in the same window with no fresh start keeps the
    /// class of the talk before it.
    pub fn observe(
        &mut self,
        own_talk_id: Option<u32>,
        by_talk_command: bool,
        now_ms: u64,
    ) -> Option<(u32, TalkOrigin)> {
        let Some(talk_id) = own_talk_id else {
            self.talk = None;
            return None;
        };
        if self.talk.as_ref().is_some_and(|(seen, _)| *seen == talk_id) {
            return None;
        }
        let fresh_start = self
            .last_start
            .as_ref()
            .filter(|start| {
                now_ms.saturating_sub(start.at_ms) <= SCRIPT_TALK_START_OPENS_THE_WINDOW_WITHIN_MS
            })
            .map(|start| start.call.clone());
        let origin = if by_talk_command {
            TalkOrigin::TalkCommand
        } else if let Some(call) = fresh_start {
            if is_event_callback_name(&call) {
                TalkOrigin::StoryStep(call)
            } else {
                TalkOrigin::NotClassed(Some(call))
            }
        } else if let Some((_, TalkOrigin::StoryStep(call))) = &self.talk {
            TalkOrigin::StoryStep(call.clone())
        } else {
            TalkOrigin::NotClassed(None)
        };
        self.talk = Some((talk_id, origin.clone()));
        Some((talk_id, origin))
    }

    pub fn current_is_story_step(&self) -> bool {
        self.talk
            .as_ref()
            .is_some_and(|(_, origin)| origin.is_story_step())
    }

    /// A line of this player's talk as the partner must get it: a story step's lines go as
    /// `StoryLine`, so the partner shows them too.
    pub fn class_update(&self, update: DialogueUpdate) -> DialogueUpdate {
        match update {
            DialogueUpdate::Line(line) if self.current_is_story_step() => {
                DialogueUpdate::StoryLine(line)
            }
            other => other,
        }
    }
}
