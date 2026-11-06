use crate::story_progress::{
    PHASE_NAME_BLOCK_OFFSET, PLAYER_NAME_BYTES, SAVE_FILE_SLOT_COUNT, SLOT_BYTES,
    SLOT_CHECKSUM_OFFSET, SLOT_VERSION_OFFSET, SLOT_VERSIONS_IN_USE, SLOTS_FILE_OFFSET, SlotStory,
    StoryRank, fix_slot_checksum, slot_player_name,
};

/// What a guest's co-op save keeps of the story: the live (host-driven) story unless the guest's
/// own story from the moment it joined, or a higher one already written this session, ranks above it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryRule {
    pub own: SlotStory,
    pub own_name: [u8; PLAYER_NAME_BYTES],
    pub session_best: Option<SlotStory>,
    pub high_mark: Option<StoryRank>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorySource {
    Live,
    SessionBest,
    Own,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOutcome {
    Written {
        slot: usize,
        source: StorySource,
        live: StoryRank,
        written: StoryRank,
        below_high_mark: bool,
    },
    KeptAsOnDisk {
        slot: usize,
    },
    OtherCharacter {
        slot: usize,
    },
}

impl StoryRule {
    pub fn new(
        own: SlotStory,
        own_name: [u8; PLAYER_NAME_BYTES],
        high_mark: Option<StoryRank>,
    ) -> Self {
        Self {
            own,
            own_name,
            session_best: None,
            high_mark,
        }
    }

    /// Picks the story to save beside `live`: the highest rank wins, a tie goes to the live story,
    /// then to this session's best.
    pub fn choose(&self, live: &SlotStory) -> (StorySource, SlotStory) {
        let mut best = (StorySource::Live, live.clone());
        let candidates = [
            (StorySource::SessionBest, self.session_best.as_ref()),
            (StorySource::Own, Some(&self.own)),
        ];
        for (source, candidate) in candidates {
            if let Some(candidate) = candidate
                && candidate.rank().outranks(&best.1.rank())
            {
                best = (source, candidate.clone());
            }
        }
        best
    }

    fn remember_written(&mut self, written: &SlotStory) {
        let rank = written.rank();
        if self
            .session_best
            .as_ref()
            .is_none_or(|best| !best.rank().outranks(&rank))
        {
            self.session_best = Some(written.clone());
        }
        if self.high_mark.is_none_or(|mark| rank.outranks(&mark)) {
            self.high_mark = Some(rank);
        }
    }
}

/// Applies the guest's save rule to a whole save file image about to be written. Only slots that
/// differ from `on_disk` are touched; a slot that differs only in the story the rule wrote before
/// is put back as it is on disk.
pub fn apply_story_rule(
    file: &mut [u8],
    on_disk: Option<&[u8]>,
    rule: &mut StoryRule,
) -> Vec<SlotOutcome> {
    let mut outcomes = Vec::new();
    for index in 0..SAVE_FILE_SLOT_COUNT {
        let start = SLOTS_FILE_OFFSET + index * SLOT_BYTES;
        let Some(slot) = file.get_mut(start..start + SLOT_BYTES) else {
            break;
        };
        if !slot_in_use(slot) {
            continue;
        }
        let disk = on_disk
            .and_then(|disk| disk.get(start..start + SLOT_BYTES))
            .filter(|disk| slot_in_use(disk));
        if disk.is_some_and(|disk| disk == &*slot) {
            continue;
        }
        if let Some(disk) = disk
            && differs_only_in_story(slot, disk)
        {
            slot.copy_from_slice(disk);
            outcomes.push(SlotOutcome::KeptAsOnDisk { slot: index });
            continue;
        }
        if slot_player_name(slot) != Some(&rule.own_name[..]) {
            outcomes.push(SlotOutcome::OtherCharacter { slot: index });
            continue;
        }
        let Some(live) = SlotStory::read(slot) else {
            continue;
        };
        let (source, chosen) = rule.choose(&live);
        let written = chosen.rank();
        if source != StorySource::Live {
            chosen.write(slot);
            fix_slot_checksum(slot);
        }
        let below_high_mark = rule.high_mark.is_some_and(|mark| mark.outranks(&written));
        rule.remember_written(&chosen);
        outcomes.push(SlotOutcome::Written {
            slot: index,
            source,
            live: live.rank(),
            written,
            below_high_mark,
        });
    }
    outcomes
}

fn slot_in_use(slot: &[u8]) -> bool {
    let version = slot
        .get(SLOT_VERSION_OFFSET..SLOT_VERSION_OFFSET + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes);
    let named_phase = slot
        .get(PHASE_NAME_BLOCK_OFFSET)
        .is_some_and(|&first| first != 0);
    version.is_some_and(|version| SLOT_VERSIONS_IN_USE.contains(&version)) && named_phase
}

fn differs_only_in_story(slot: &[u8], disk: &[u8]) -> bool {
    let Some(disk_story) = SlotStory::read(disk) else {
        return false;
    };
    let mut probe = slot.to_vec();
    if !disk_story.write(&mut probe) {
        return false;
    }
    let checksum = SLOT_CHECKSUM_OFFSET..SLOT_CHECKSUM_OFFSET + 4;
    match (probe.get_mut(checksum.clone()), disk.get(checksum)) {
        (Some(field), Some(on_disk)) => field.copy_from_slice(on_disk),
        _ => return false,
    }
    probe == disk
}
