use crate::callback_gate::GateRole;
use crate::message_handlers::RelayedHandlerPasses;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandlerFireFacts {
    pub host_driven_handler: bool,
    pub role: Option<GateRole>,
    pub both_in_phase: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerFire {
    NotAHandler,
    RunHereOnly,
    HostRunsAndSends,
    GuestRunsTheHostsCall,
    GuestHoldsForTheHost,
}

pub fn handler_fire(
    facts: &HandlerFireFacts,
    passes: &mut RelayedHandlerPasses,
    name: &str,
    now_ms: u64,
) -> HandlerFire {
    if !facts.host_driven_handler {
        return HandlerFire::NotAHandler;
    }
    let Some(role) = facts.role.filter(|_| facts.both_in_phase) else {
        return HandlerFire::RunHereOnly;
    };
    match role {
        GateRole::Host => HandlerFire::HostRunsAndSends,
        GateRole::Guest if passes.take(name, now_ms) => HandlerFire::GuestRunsTheHostsCall,
        GateRole::Guest => HandlerFire::GuestHoldsForTheHost,
    }
}
