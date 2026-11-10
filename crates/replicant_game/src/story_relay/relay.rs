use replicant_proto::StoryRefusalReason;

use crate::stay_together::FollowRole;
use crate::story_calls::{SceneScope, StoryCallKind};
use crate::story_relay::{CallKey, InjectedMatch, StoryRelayLedger};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireFacts {
    pub enabled: bool,
    pub connected: bool,
    pub role: FollowRole,
    pub local_in_play: bool,
    pub partner_in_same_phase: bool,
    pub kind: Option<StoryCallKind>,
    pub scope: SceneScope,
    pub partner_here: bool,
    pub owned_elsewhere: bool,
    pub args_relayable: bool,
    pub partner_too_far: bool,
    pub can_gather_partner: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireAction {
    RunLocally,
    RunLocalSceneOnly,
    RunWithoutPartner,
    HoldForGatheredPartner,
    RunInjected(InjectedMatch),
    RunAndAnnounce,
    SuppressAndAsk,
    DropDuplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunFacts {
    pub enabled: bool,
    pub role: FollowRole,
    pub local_in_play: bool,
    pub local_phase: Option<u16>,
    pub call_phase: u16,
    pub known_story_call: bool,
    pub scope: SceneScope,
    pub partner_here: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropReason {
    Disabled,
    WrongRole,
    NotInPlay,
    WrongPhase,
    NotAStoryCall,
    LocalScene,
    Elsewhere,
    Duplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerAction {
    InjectAndBroadcast,
    Drop(DropReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunAction {
    Inject,
    IgnoreLate,
    Drop(DropReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldTimeoutVerdict {
    RunHere,
    KeepWaiting,
    Drop,
}

impl From<DropReason> for StoryRefusalReason {
    fn from(reason: DropReason) -> Self {
        match reason {
            DropReason::Disabled => Self::Disabled,
            DropReason::WrongRole => Self::WrongRole,
            DropReason::NotInPlay => Self::NotInPlay,
            DropReason::WrongPhase => Self::WrongPhase,
            DropReason::NotAStoryCall | DropReason::LocalScene => Self::NotAStoryCall,
            DropReason::Elsewhere => Self::WrongPhase,
            DropReason::Duplicate => Self::Duplicate,
        }
    }
}

impl StoryRelayLedger {
    pub fn on_fire(
        &mut self,
        argument_pointer: u64,
        facts: &FireFacts,
        key: CallKey,
        now_ms: u64,
    ) -> FireAction {
        if let Some(how) = self.take_injected(argument_pointer, key, now_ms) {
            if matches!(how, InjectedMatch::Key { .. }) {
                match facts.role {
                    FollowRole::Host => self.take_accepted(key, now_ms),
                    FollowRole::Guest => self.take_ran_there(key, now_ms),
                    FollowRole::Unknown => false,
                };
            }
            return FireAction::RunInjected(how);
        }
        let relayed_kind = facts.kind.is_some_and(StoryCallKind::is_relayed);
        if relayed_kind && facts.enabled && facts.connected && !facts.owned_elsewhere {
            if facts.scope.is_local() {
                return FireAction::RunLocalSceneOnly;
            }
            if facts.scope == SceneScope::SharedScene && !facts.partner_here {
                let can_pull_partner = facts.role == FollowRole::Host
                    && facts.local_in_play
                    && facts.partner_in_same_phase
                    && facts.partner_too_far
                    && facts.args_relayable
                    && facts.can_gather_partner;
                if can_pull_partner {
                    return FireAction::HoldForGatheredPartner;
                }
                return FireAction::RunWithoutPartner;
            }
        }
        let can_relay = facts.enabled
            && facts.connected
            && facts.local_in_play
            && facts.partner_in_same_phase
            && facts.args_relayable
            && !facts.owned_elsewhere
            && relayed_kind;
        if !can_relay {
            return FireAction::RunLocally;
        }
        match facts.role {
            FollowRole::Host => {
                if self.take_accepted(key, now_ms) {
                    return FireAction::DropDuplicate;
                }
                self.note_ran_here(key, now_ms);
                FireAction::RunAndAnnounce
            }
            FollowRole::Guest => {
                if self.take_ran_there(key, now_ms) {
                    return FireAction::DropDuplicate;
                }
                self.hold_call(key, now_ms);
                FireAction::SuppressAndAsk
            }
            FollowRole::Unknown => FireAction::RunLocally,
        }
    }

    pub fn on_trigger(&mut self, facts: &RunFacts, key: CallKey, now_ms: u64) -> TriggerAction {
        if let Some(reason) = refusal(facts, FollowRole::Host) {
            return TriggerAction::Drop(reason);
        }
        if !self.accept_trigger(key, now_ms) {
            return TriggerAction::Drop(DropReason::Duplicate);
        }
        TriggerAction::InjectAndBroadcast
    }

    pub fn on_run(&mut self, facts: &RunFacts, key: CallKey, now_ms: u64) -> RunAction {
        if self.take_ran_locally(key, now_ms) {
            return RunAction::IgnoreLate;
        }
        self.resolve_held(key);
        if let Some(reason) = refusal(facts, FollowRole::Guest) {
            return RunAction::Drop(reason);
        }
        self.note_ran_there(key, now_ms);
        RunAction::Inject
    }
}

pub fn held_timeout_verdict(
    local_in_play: bool,
    local_phase: Option<u16>,
    call_phase: u16,
) -> HeldTimeoutVerdict {
    match local_phase {
        Some(phase) if phase != call_phase => HeldTimeoutVerdict::Drop,
        Some(_) if local_in_play => HeldTimeoutVerdict::RunHere,
        _ => HeldTimeoutVerdict::KeepWaiting,
    }
}

fn refusal(facts: &RunFacts, wanted_role: FollowRole) -> Option<DropReason> {
    if !facts.enabled {
        return Some(DropReason::Disabled);
    }
    if facts.role != wanted_role {
        return Some(DropReason::WrongRole);
    }
    if !facts.local_in_play {
        return Some(DropReason::NotInPlay);
    }
    if facts.local_phase != Some(facts.call_phase) {
        return Some(DropReason::WrongPhase);
    }
    if !facts.known_story_call {
        return Some(DropReason::NotAStoryCall);
    }
    if facts.scope.is_local() {
        return Some(DropReason::LocalScene);
    }
    if facts.scope == SceneScope::SharedScene && !facts.partner_here {
        return Some(DropReason::Elsewhere);
    }
    None
}
