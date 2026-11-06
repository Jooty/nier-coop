use std::sync::Mutex;

use rc_game::story_calls::StoryCallKind;

use crate::diagnostics::log_line;

#[derive(Default)]
struct PackCounts {
    pack_hash: Option<u64>,
    fired_calls: u32,
    pack_known: u32,
    story_calls: u32,
}

static COUNTS: Mutex<PackCounts> = Mutex::new(PackCounts {
    pack_hash: None,
    fired_calls: 0,
    pack_known: 0,
    story_calls: 0,
});

pub(super) fn note_pack_lookup(pack_hash: Option<u64>, kind: Option<StoryCallKind>) {
    let Ok(mut counts) = COUNTS.lock() else {
        return;
    };
    if counts.pack_hash != pack_hash {
        if let Some(previous) = counts.pack_hash {
            log_line(&format!(
                "story relay: pack {previous:016x} had {} fired calls, {} with a known pack, {} story",
                counts.fired_calls, counts.pack_known, counts.story_calls
            ));
        }
        *counts = PackCounts {
            pack_hash,
            ..PackCounts::default()
        };
        if let Some(pack) = pack_hash {
            log_line(&format!(
                "story relay: phase pack {pack:016x} is {} the story-call table",
                if kind.is_some() { "in" } else { "not in" }
            ));
        }
    }
    counts.fired_calls += 1;
    counts.pack_known += u32::from(kind.is_some());
    counts.story_calls += u32::from(kind == Some(StoryCallKind::Story));
}
