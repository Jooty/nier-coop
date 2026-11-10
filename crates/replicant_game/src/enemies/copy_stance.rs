use std::collections::{BTreeMap, BTreeSet};

use replicant_proto::EnemyId;

/// A decision the guest made about one copy of a host enemy, logged once per copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CopyStance {
    HeldAgainstOwnStep,
    DeadStopped,
    HiddenWithHostBody,
    BossFollowsHostMotion,
    RunsOwnMotion,
}

/// Remembers which stances were already announced for each copy in this phase.
#[derive(Debug, Default)]
pub struct CopyStanceLog {
    phase: Option<u16>,
    announced: BTreeMap<EnemyId, BTreeSet<CopyStance>>,
}

impl CopyStanceLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// True the first time `id` takes `stance` in `phase`; a new phase forgets every copy.
    pub fn first_time(&mut self, phase: u16, id: EnemyId, stance: CopyStance) -> bool {
        if self.phase != Some(phase) {
            self.announced.clear();
            self.phase = Some(phase);
        }
        self.announced.entry(id).or_default().insert(stance)
    }
}

impl CopyStance {
    /// The log line for this stance; `detail` is the cut step length, or the time since the death.
    pub fn text(self, id: &str, detail: f32) -> String {
        match self {
            Self::HeldAgainstOwnStep => format!(
                "copy {id} tried to move on its own ({detail:.2} units in one step, a down, hit or death motion); holding it where the host's is"
            ),
            Self::DeadStopped => format!(
                "copy {id} dead, stopped where the host's died; it can fall but not slide until the host's body goes"
            ),
            Self::HiddenWithHostBody => format!(
                "copy {id} removed: the host's body is gone, {detail:.0} ms after its death"
            ),
            Self::BossFollowsHostMotion => format!(
                "boss copy {id} follows motion {detail:.0} from the host (held, the host's motions played on it)"
            ),
            Self::RunsOwnMotion => format!(
                "copy {id} follows motion: none, the host's body motion is unreadable; your copy runs its own motions and AI at the host's place (its hits on you stay blocked)"
            ),
        }
    }
}
