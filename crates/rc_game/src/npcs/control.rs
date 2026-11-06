use crate::npcs::NpcEvent;

/// Who moves one NPC on this machine this tick. Every variant but `OwnLogic` holds the NPC's AI;
/// `PartnerTalk` poses it as the talker's game shows it, `Host` as the host's does (guest only),
/// and `Unmatched` leaves it to the planner, which hides it after the grace time (guest only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcControl {
    OwnLogic,
    PartnerTalk,
    Host,
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
