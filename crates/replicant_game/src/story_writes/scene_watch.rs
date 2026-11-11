use replicant_proto::{MAX_STORY_CHANGES_PER_MESSAGE, StoryArray, StoryChange};

use crate::stay_together::FollowRole;
use crate::story_calls::SceneScope;
use crate::story_progress::StoryArrays;
use crate::story_relay::FireAction;
use crate::story_writes::{
    AREA_SCRATCH_GAME_FLAG_IDS, FLAGS_A_LOCAL_SCENE_ALSO_CLEARS,
    LOCAL_SCENE_CALL_START_WAIT_MAX_MS, LOCAL_SCENE_WRITE_WATCH_MAX_MS,
};

/// The guest's story state from just before a local scene started here, kept while the scene
/// runs, so what the scene writes can be sent to the host.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneWriteWatch {
    watched: Option<WatchedScene>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WatchedScene {
    name: String,
    baseline: StoryArrays,
    popped_ms: u64,
    call_started: bool,
}

impl SceneWriteWatch {
    pub const fn new() -> Self {
        Self { watched: None }
    }

    pub fn begin(&mut self, name: &str, baseline: StoryArrays, now_ms: u64) {
        self.watched = Some(WatchedScene {
            name: name.to_owned(),
            baseline,
            popped_ms: now_ms,
            call_started: false,
        });
    }

    /// True when a call other than the watched scene starts, which ends the scene's run.
    pub fn another_call_started(&mut self, started_name: &[u8]) -> bool {
        let Some(watched) = self.watched.as_mut() else {
            return false;
        };
        if watched.name.as_bytes() == started_name && !watched.call_started {
            watched.call_started = true;
            return false;
        }
        true
    }

    pub fn watched_scene(&self) -> Option<&str> {
        self.watched.as_ref().map(|watched| watched.name.as_str())
    }

    /// Changes the host's mirror applied here are the host's own, never sent back to it.
    pub fn rebaseline_over_mirror_fixes(&mut self, applied: &[StoryChange]) {
        if let Some(watched) = self.watched.as_mut() {
            for &change in applied {
                watched.baseline.apply(change);
            }
        }
    }

    /// What the scene wrote since the last look, in message-sized batches. The watch ends when
    /// the scene no longer runs (a popped scene whose call has not started yet still counts as
    /// running for a short while) or has run past [`LOCAL_SCENE_WRITE_WATCH_MAX_MS`].
    pub fn take_writes(
        &mut self,
        live: &StoryArrays,
        scene_still_runs: bool,
        now_ms: u64,
    ) -> Vec<Vec<StoryChange>> {
        let Some(watched) = self.watched.as_mut() else {
            return Vec::new();
        };
        let batches = writes_for_the_host(&watched.baseline, live);
        let watched_ms = now_ms.saturating_sub(watched.popped_ms);
        let waiting_for_the_call =
            !watched.call_started && watched_ms < LOCAL_SCENE_CALL_START_WAIT_MAX_MS;
        let keep_watching = (scene_still_runs || waiting_for_the_call)
            && watched_ms < LOCAL_SCENE_WRITE_WATCH_MAX_MS;
        if keep_watching {
            watched.baseline = live.clone();
        } else {
            self.watched = None;
        }
        batches
    }

    pub fn take_last_writes(&mut self, live: &StoryArrays) -> Vec<Vec<StoryChange>> {
        self.watched
            .take()
            .map(|watched| writes_for_the_host(&watched.baseline, live))
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.watched = None;
    }
}

fn writes_for_the_host(baseline: &StoryArrays, live: &StoryArrays) -> Vec<Vec<StoryChange>> {
    let for_the_host: Vec<StoryChange> = baseline
        .changes_to(live)
        .into_iter()
        .filter(|&change| is_scene_write_for_the_host(change))
        .collect();
    for_the_host
        .chunks(MAX_STORY_CHANGES_PER_MESSAGE)
        .map(<[StoryChange]>::to_vec)
        .collect()
}

/// A local scene's write that is story progress: a counter, or a flag turned on that is neither
/// area scratch nor a flag local scenes also turn off. Every other write stays led by the host.
pub fn is_scene_write_for_the_host(change: StoryChange) -> bool {
    match change {
        StoryChange::Flag { on: false, .. } => false,
        StoryChange::Flag {
            array,
            id,
            on: true,
        } => {
            let area_scratch =
                array == StoryArray::GameFlags && AREA_SCRATCH_GAME_FLAG_IDS.contains(&id);
            !area_scratch && !FLAGS_A_LOCAL_SCENE_ALSO_CLEARS.contains(&(array, id))
        }
        StoryChange::Counter { .. } => true,
    }
}

/// Only a connected guest's own run of a local scene is watched: the host's writes are sent by
/// its story feed, and a call the relay holds, drops or runs for the partner is led by the host.
pub fn watches_scene_writes(
    role: FollowRole,
    connected: bool,
    scope: SceneScope,
    action: FireAction,
) -> bool {
    role == FollowRole::Guest
        && connected
        && scope.is_local()
        && matches!(
            action,
            FireAction::RunLocalSceneOnly | FireAction::RunLocally
        )
}
