use rc_proto::{MAX_STORY_CHANGES_PER_MESSAGE, StoryArray, StoryArrayPart, StoryChanges};

use crate::story_progress::{STORY_CHANGES_BEFORE_A_FULL_RESEND, StoryArrays};

/// The host's side of story sync: what the guest was last sent, and what to send next.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryFeed {
    sent: Option<StoryArrays>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedSend {
    Nothing,
    Changes(Vec<StoryChanges>),
    WholeArrays(Vec<StoryArrayPart>),
}

impl StoryFeed {
    pub fn is_active(&self) -> bool {
        self.sent.is_some()
    }

    pub fn whole(&mut self, current: &StoryArrays) -> Vec<StoryArrayPart> {
        self.sent = Some(current.clone());
        whole_arrays(current)
    }

    pub fn next(&mut self, current: &StoryArrays) -> FeedSend {
        let Some(sent) = &self.sent else {
            return FeedSend::Nothing;
        };
        let changes = sent.changes_to(current);
        if changes.is_empty() {
            return FeedSend::Nothing;
        }
        self.sent = Some(current.clone());
        if changes.len() > STORY_CHANGES_BEFORE_A_FULL_RESEND {
            return FeedSend::WholeArrays(whole_arrays(current));
        }
        FeedSend::Changes(
            changes
                .chunks(MAX_STORY_CHANGES_PER_MESSAGE)
                .map(|chunk| StoryChanges {
                    changes: chunk.to_vec(),
                })
                .collect(),
        )
    }

    pub fn sent_digest(&self) -> Option<u64> {
        self.sent.as_ref().map(StoryArrays::digest)
    }

    pub fn reset(&mut self) {
        self.sent = None;
    }
}

fn whole_arrays(current: &StoryArrays) -> Vec<StoryArrayPart> {
    StoryArray::ALL
        .into_iter()
        .map(|array| StoryArrayPart {
            array,
            bytes: current.bytes(array).to_vec(),
        })
        .collect()
}
