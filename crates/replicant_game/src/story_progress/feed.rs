use replicant_proto::{
    MAX_STORY_CHANGES_PER_MESSAGE, StoryArray, StoryArrayPart, StoryChange, StoryChanges,
};

use crate::story_progress::{
    STORY_CHANGES_BEFORE_A_FULL_RESEND, StoryArrays, without_player_local_flags,
};

/// The host's side of story sync: what the guest was last sent, and what to send next.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryFeed {
    sent: Option<StoryArrays>,
    last_changes: Vec<StoryChange>,
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

    /// Player-local flags are never sent: the copy sent and kept has them cleared.
    pub fn whole(&mut self, current: &StoryArrays) -> Vec<StoryArrayPart> {
        let current = without_player_local_flags(current);
        let parts = whole_arrays(&current);
        self.sent = Some(current);
        parts
    }

    pub fn next(&mut self, current: &StoryArrays) -> FeedSend {
        let Some(sent) = &self.sent else {
            return FeedSend::Nothing;
        };
        let current = &without_player_local_flags(current);
        let changes = sent.changes_to(current);
        if changes.is_empty() {
            return FeedSend::Nothing;
        }
        self.sent = Some(current.clone());
        self.last_changes.clone_from(&changes);
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

    pub fn last_changes(&self) -> &[StoryChange] {
        &self.last_changes
    }

    pub fn sent_digest(&self) -> Option<u64> {
        self.sent.as_ref().map(StoryArrays::digest)
    }

    pub fn reset(&mut self) {
        self.sent = None;
        self.last_changes.clear();
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
