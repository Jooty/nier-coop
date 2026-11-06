use crate::npcs::NpcEvent;

/// Who moves one NPC on this machine this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcControl {
    /// Its own AI runs: this machine's talk or script owns it, or (on the host) nothing else does.
    OwnLogic,
    /// Held, and posed as the partner's game shows it, because the partner is talking to it.
    PartnerTalk,
    /// Held, and posed as the host's game shows it (guest only).
    Host,
    /// Not matched with the host yet (guest only): left to the planner, which hides it later.
    Unmatched,
}

/// The hold rules. A talk or script on this machine always wins, so a conversation started here
/// can never wait on an NPC the sync is holding; then the partner's talk; then the host's state.
pub fn npc_control(
    is_host: bool,
    local_event: Option<NpcEvent>,
    partner_talking: bool,
    matched_with_host: bool,
) -> NpcControl {
    if local_event.is_some() {
        return NpcControl::OwnLogic;
    }
    if partner_talking {
        return NpcControl::PartnerTalk;
    }
    if is_host {
        return NpcControl::OwnLogic;
    }
    if matched_with_host {
        NpcControl::Host
    } else {
        NpcControl::Unmatched
    }
}
