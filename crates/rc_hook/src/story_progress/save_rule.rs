use std::fs;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use rc_game::save_files::COOP_GAMEDATA_FILE_NAME;
use rc_game::story_progress::{
    PLAYER_NAME_BYTES, RANK_SIDECAR_FILE_NAME, SAVE_FILE_BYTES, SlotOutcome, SlotStory, StoryRank,
    StoryRule, apply_story_rule, decode_rank_sidecar, encode_rank_sidecar,
};

use crate::diagnostics::log_line;

static ARMED: Mutex<Option<ArmedRule>> = Mutex::new(None);

struct ArmedRule {
    rule: StoryRule,
    sidecar_read: bool,
}

pub(super) fn is_armed() -> bool {
    ARMED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_some()
}

pub(super) fn arm(own: SlotStory, own_name: [u8; PLAYER_NAME_BYTES]) {
    let rank = own.rank();
    log_line(&format!(
        "story: guest's own story at join: part {:?}, {} milestone flags",
        rank.part, rank.milestones
    ));
    if rank.is_empty() {
        log_line(
            "story: the guest's own save has no story progress yet; its co-op save takes the session's",
        );
    }
    *ARMED.lock().unwrap_or_else(PoisonError::into_inner) = Some(ArmedRule {
        rule: StoryRule::new(own, own_name, None),
        sidecar_read: false,
    });
}

/// Runs on the game's save thread for every write of the co-op file. Returns the file to write
/// instead when the guest's rule changed a slot's story.
pub fn rewrite_coop_save(folder: &Path, data: &[u8]) -> Option<Vec<u8>> {
    let mut armed = ARMED.lock().unwrap_or_else(PoisonError::into_inner);
    let armed = armed.as_mut()?;
    if data.len() != SAVE_FILE_BYTES {
        log_line(&format!(
            "story: co-op save of {} bytes is not a whole save file, left alone",
            data.len()
        ));
        return None;
    }
    let sidecar = folder.join(RANK_SIDECAR_FILE_NAME);
    if !armed.sidecar_read {
        armed.sidecar_read = true;
        armed.rule.high_mark = fs::read(&sidecar)
            .ok()
            .and_then(|bytes| decode_rank_sidecar(&bytes));
        log_line(&format!(
            "story: high mark from the sidecar: {:?}",
            armed
                .rule
                .high_mark
                .map(|mark| (mark.part, mark.milestones))
        ));
    }
    let on_disk = fs::read(folder.join(COOP_GAMEDATA_FILE_NAME))
        .ok()
        .filter(|bytes| bytes.len() == SAVE_FILE_BYTES);
    let mark_before = armed.rule.high_mark;
    let mut file = data.to_vec();
    let outcomes = apply_story_rule(&mut file, on_disk.as_deref(), &mut armed.rule);
    outcomes.iter().for_each(log_outcome);
    if let Some(mark) = armed.rule.high_mark
        && armed.rule.high_mark != mark_before
    {
        match fs::write(&sidecar, encode_rank_sidecar(mark)) {
            Ok(()) => log_line(&format!(
                "story: high mark is now part {:?}, {} milestone flags",
                mark.part, mark.milestones
            )),
            Err(error) => log_line(&format!("story: cannot write the rank sidecar ({error})")),
        }
    }
    (file != data).then_some(file)
}

fn log_outcome(outcome: &SlotOutcome) {
    let line = match *outcome {
        SlotOutcome::Written {
            slot,
            source,
            live,
            written,
            below_high_mark,
        } => format!(
            "story: co-op save slot {slot} keeps the {source:?} story ({}; live was {}){}",
            describe(written),
            describe(live),
            if below_high_mark {
                ", below the high mark of an earlier session"
            } else {
                ""
            }
        ),
        SlotOutcome::KeptAsOnDisk { slot } => {
            format!("story: co-op save slot {slot} differs only in story, kept as on disk")
        }
        SlotOutcome::OtherCharacter { slot } => {
            format!("story: co-op save slot {slot} holds another character, rule skipped")
        }
    };
    log_line(&line);
}

fn describe(rank: StoryRank) -> String {
    format!("part {:?}, {} milestone flags", rank.part, rank.milestones)
}
