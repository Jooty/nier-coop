use replicant_proto::CartRide;

use crate::carts::{
    CART_END_PULL_DELAY_MS, CART_PARTNER_NEAR_UNITS, CART_RIDER_REACH_UNITS, CartSighting,
    CartState, parked_state_after, run_state_from,
};
use crate::stay_together::{JumpPartner, JumpStep, distance_between};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartRideFacts<'a> {
    pub now_ms: u64,
    pub phase: Option<u16>,
    pub carts: &'a [CartSighting],
    pub rider_on_cart: bool,
    pub rider_position: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CartRideEvent {
    Started(CartRide),
    Ended(CartRide),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Ridden {
    actor: u64,
    started_at: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OwedPull {
    due_ms: u64,
    from: [f32; 3],
    phase: u16,
}

/// The host's side of a cart ride: sees its own player start and end a ride, and a moment after
/// the end asks for the partner to be brought along when it did not arrive on its own cart.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostRideWatch {
    phase: Option<u16>,
    last_seen: Vec<CartSighting>,
    ridden: Option<Ridden>,
    owed_pull: Option<OwedPull>,
}

impl HostRideWatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn observe(&mut self, facts: &CartRideFacts) -> Vec<CartRideEvent> {
        if facts.phase != self.phase {
            self.reset();
            self.phase = facts.phase;
            self.last_seen = facts.carts.to_vec();
            return Vec::new();
        }
        let Some(phase) = facts.phase else {
            return Vec::new();
        };
        let mut events = Vec::new();
        for cart in facts.carts {
            let Some(before) = self
                .last_seen
                .iter()
                .find(|seen| seen.actor == cart.actor)
                .copied()
            else {
                continue;
            };
            if let Some(event) = self.transition(phase, facts, &before, cart) {
                events.push(event);
            }
        }
        self.last_seen = facts.carts.to_vec();
        events
    }

    /// Due once `CART_END_PULL_DELAY_MS` after a ride ended: bring the partner over when it is in
    /// this map and more than `CART_PARTNER_NEAR_UNITS` from the host.
    pub fn pull_due(
        &mut self,
        now_ms: u64,
        position: Option<[f32; 3]>,
        partner: Option<JumpPartner>,
    ) -> JumpStep {
        let Some(owed) = self.owed_pull.filter(|owed| now_ms >= owed.due_ms) else {
            return JumpStep::Nothing;
        };
        self.owed_pull = None;
        let Some(to) = position else {
            return JumpStep::Nothing;
        };
        let Some(partner) = partner.filter(|partner| partner.phase == owed.phase) else {
            return JumpStep::Nothing;
        };
        match distance_between(partner.position, to) {
            Some(partner_away) if partner_away > CART_PARTNER_NEAR_UNITS => {
                JumpStep::BringPartner {
                    from: owed.from,
                    to,
                    partner_away,
                }
            }
            _ => JumpStep::Nothing,
        }
    }

    pub fn riding(&self) -> bool {
        self.ridden.is_some()
    }

    fn transition(
        &mut self,
        phase: u16,
        facts: &CartRideFacts,
        before: &CartSighting,
        now: &CartSighting,
    ) -> Option<CartRideEvent> {
        match (before.state, now.state) {
            (CartState::Parked(from), CartState::Running(to))
                if run_state_from(from) == Some(to) && facts.rider_on_cart =>
            {
                self.ridden = Some(Ridden {
                    actor: now.actor,
                    started_at: before.position,
                });
                Some(CartRideEvent::Started(CartRide {
                    phase,
                    layout_id: now.layout_id,
                    cart_position: before.position,
                    from_state: from,
                    to_state: to,
                    rider_offset: rider_offset(facts.rider_position, now.position),
                }))
            }
            (CartState::Running(from), CartState::Parked(to))
                if parked_state_after(from) == Some(to)
                    && self.ridden.is_some_and(|ridden| ridden.actor == now.actor) =>
            {
                let started_at = self.ridden.take().map_or(now.position, |r| r.started_at);
                self.owed_pull = Some(OwedPull {
                    due_ms: facts.now_ms.saturating_add(CART_END_PULL_DELAY_MS),
                    from: started_at,
                    phase,
                });
                Some(CartRideEvent::Ended(CartRide {
                    phase,
                    layout_id: now.layout_id,
                    cart_position: now.position,
                    from_state: from,
                    to_state: to,
                    rider_offset: rider_offset(facts.rider_position, now.position),
                }))
            }
            _ => None,
        }
    }
}

fn rider_offset(rider: Option<[f32; 3]>, cart: [f32; 3]) -> [f32; 3] {
    let Some(rider) = rider else {
        return [0.0; 3];
    };
    let offset = [rider[0] - cart[0], rider[1] - cart[1], rider[2] - cart[2]];
    // A rider reported far from the cart did not stand on it: the partner goes on its middle.
    if distance_between(rider, cart).is_some_and(|away| away <= CART_RIDER_REACH_UNITS) {
        offset
    } else {
        [0.0; 3]
    }
}
