use std::fs;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use rc_game::save_files::{COOP_GAMEDATA_BACKUP_FILE_NAME, COOP_GAMEDATA_FILE_NAME};
use rc_game::story_progress::{
    PLAYER_NAME_BYTES, RANK_SIDECAR_FILE_NAME, SAVE_FILE_BYTES, SlotOutcome, SlotStory, StoryRank,
    StoryRule, apply_story_rule, best_story_for_name, decode_high_water_sidecar,
    encode_high_water_sidecar,
};

use crate::diagnostics::log_line;
use crate::saves::write_atomically;

static ARMED: Mutex<Option<ArmedRule>> = Mutex::new(None);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct SnapshotKey {
    pub slot: Option<u32>,
    pub name: [u8; PLAYER_NAME_BYTES],
}

struct ArmedRule {
    rule: StoryRule,
    key: SnapshotKey,
    high_water_loaded: bool,
    pending: Option<StoryRule>,
}

pub(super) fn armed_key() -> Option<SnapshotKey> {
    ARMED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(|armed| armed.key)
}

pub(super) fn arm(own: SlotStory, key: SnapshotKey) {
    let rank = own.rank();
    log_line(&format!(
        "story: guest's own story at join: cycle {}, part {:?}, {} milestone flags, slot {:?}",
        rank.cycle, rank.part, rank.milestones, key.slot
    ));
    if rank.is_empty() {
        log_line(
            "story: the guest's own save has no story progress yet; its co-op save takes the session's",
        );
    }
    *ARMED.lock().unwrap_or_else(PoisonError::into_inner) = Some(ArmedRule {
        rule: StoryRule::new(own, key.name, None),
        key,
        high_water_loaded: false,
        pending: None,
    });
}

/// Runs on the game's save thread for every write of the co-op file. Returns the file to write
/// instead when the guest's rule changed a slot's story. The rule's new state only takes effect in
/// [`finish_coop_save`], once the game's write succeeded.
pub fn rewrite_coop_save(folder: &Path, data: &[u8]) -> Option<Vec<u8>> {
    let mut armed = ARMED.lock().unwrap_or_else(PoisonError::into_inner);
    let armed = armed.as_mut()?;
    armed.pending = None;
    if data.len() != SAVE_FILE_BYTES {
        log_line(&format!(
            "story: co-op save of {} bytes is not a whole save file, left alone",
            data.len()
        ));
        return None;
    }
    if !armed.high_water_loaded {
        armed.high_water_loaded = true;
        armed.rule.high_water = load_high_water(folder, &armed.rule.own_name);
        log_line(&format!(
            "story: high water at the start of the session: {:?}",
            armed.rule.high_mark().map(describe)
        ));
    }
    let on_disk = fs::read(folder.join(COOP_GAMEDATA_FILE_NAME))
        .ok()
        .filter(|bytes| bytes.len() == SAVE_FILE_BYTES);
    let mut rule = armed.rule.clone();
    let mut file = data.to_vec();
    let outcomes = apply_story_rule(&mut file, on_disk.as_deref(), &mut rule);
    outcomes.iter().for_each(log_outcome);
    armed.pending = Some(rule);
    (file != data).then_some(file)
}

/// Runs after the game's write of the co-op file returned.
pub fn finish_coop_save(folder: &Path, succeeded: bool) {
    let mut armed = ARMED.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(armed) = armed.as_mut() else {
        return;
    };
    let Some(rule) = armed.pending.take() else {
        return;
    };
    if !succeeded {
        log_line("story: the game's co-op write failed, so the high water is not raised");
        return;
    }
    let raised = rule.high_water != armed.rule.high_water;
    armed.rule = rule;
    if !raised {
        return;
    }
    let Some(story) = &armed.rule.high_water else {
        return;
    };
    let bytes = encode_high_water_sidecar(story, &armed.rule.own_name);
    match write_atomically(&folder.join(RANK_SIDECAR_FILE_NAME), &bytes) {
        Ok(()) => log_line(&format!(
            "story: high water is now {}",
            describe(story.rank())
        )),
        Err(error) => log_line(&format!(
            "story: cannot write the high water sidecar ({error})"
        )),
    }
}

fn load_high_water(folder: &Path, name: &[u8; PLAYER_NAME_BYTES]) -> Option<SlotStory> {
    let sidecar = folder.join(RANK_SIDECAR_FILE_NAME);
    match fs::read(&sidecar) {
        Ok(bytes) => match decode_high_water_sidecar(&bytes) {
            Some((story, stored_name)) if stored_name == *name => return Some(story),
            Some(_) => {
                log_line("story: the high water sidecar belongs to another character, ignored");
                return None;
            }
            None => log_line("story: the high water sidecar is damaged, using the backup file"),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log_line(&format!(
            "story: cannot read the high water sidecar ({error})"
        )),
    }
    let backup = fs::read(folder.join(COOP_GAMEDATA_BACKUP_FILE_NAME)).ok()?;
    best_story_for_name(&backup, name)
}

fn log_outcome(outcome: &SlotOutcome) {
    let line = match *outcome {
        SlotOutcome::Written {
            slot,
            source,
            live,
            written,
        } => format!(
            "story: co-op save slot {slot} keeps the {source:?} story ({}; live was {})",
            describe(written),
            describe(live)
        ),
        SlotOutcome::KeptAsOnDisk { slot } => {
            format!("story: co-op save slot {slot} differs only in story, kept as on disk")
        }
        SlotOutcome::OtherCharacter { slot } => {
            format!("story: co-op save slot {slot} holds another character, rule skipped")
        }
        SlotOutcome::StoryWriteFailed { slot } => {
            format!(
                "story: co-op save slot {slot}: the story write failed, slot left as the game made it"
            )
        }
    };
    log_line(&line);
}

fn describe(rank: StoryRank) -> String {
    format!(
        "cycle {}, part {:?}, {} milestone flags",
        rank.cycle, rank.part, rank.milestones
    )
}
