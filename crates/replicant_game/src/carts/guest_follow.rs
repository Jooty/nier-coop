use replicant_proto::CartRide;

use crate::carts::{
    CART_BOARD_GIVE_UP_MS, CART_BOARD_SETTLE_MS, CART_END_GRACE_MS, CART_RIDER_LIFT_UNITS,
    CartSighting, CartState, cart_state, match_cart,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OwnCartFacts<'a> {
    pub now_ms: u64,
    pub phase: Option<u16>,
    pub carts: &'a [CartSighting],
    pub may_place_rider: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CartFollowStep {
    Nothing,
    /// Put this game's cart at the end the host's ride starts from.
    Park {
        actor: u64,
        state: u8,
    },
    /// Stand this game's player on its cart, where the host stands on the host's.
    PlaceRider {
        actor: u64,
        spot: [f32; 3],
    },
    /// Run this game's cart along the rail the host rides.
    Start {
        actor: u64,
        state: u8,
    },
    /// The host's ride ended and this game's cart is not at that end: put it there.
    ParkAtEnd {
        actor: u64,
        state: u8,
    },
    NoCartHere {
        ride: CartRide,
    },
    GaveUpBoarding {
        waited_ms: u64,
    },
    OtherMap,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Follow {
    Idle,
    Boarding {
        ride: CartRide,
        arrived_ms: u64,
        placed_ms: Option<u64>,
    },
    Riding {
        actor: u64,
        phase: u16,
    },
    Ending {
        ride: CartRide,
        actor: Option<u64>,
        due_ms: u64,
    },
}

/// The guest's side of a cart ride the host started: its own cart, parked where the host's was,
/// carries its own player along the same rail, and ends where the host's ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuestCartFollow {
    follow: Follow,
}

impl Default for GuestCartFollow {
    fn default() -> Self {
        Self::new()
    }
}

impl GuestCartFollow {
    pub const fn new() -> Self {
        Self {
            follow: Follow::Idle,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn host_rode(&mut self, ride: CartRide, now_ms: u64) {
        let started = matches!(cart_state(i32::from(ride.to_state)), CartState::Running(_));
        if started {
            self.follow = Follow::Boarding {
                ride,
                arrived_ms: now_ms,
                placed_ms: None,
            };
            return;
        }
        let actor = match self.follow {
            Follow::Riding { actor, .. } => Some(actor),
            _ => None,
        };
        self.follow = Follow::Ending {
            ride,
            actor,
            due_ms: now_ms.saturating_add(CART_END_GRACE_MS),
        };
    }

    pub fn step(&mut self, facts: &OwnCartFacts) -> CartFollowStep {
        match self.follow {
            Follow::Idle => CartFollowStep::Nothing,
            Follow::Boarding {
                ride,
                arrived_ms,
                placed_ms,
            } => self.board(facts, ride, arrived_ms, placed_ms),
            Follow::Riding { actor, phase } => {
                let still_running = facts.phase == Some(phase)
                    && facts.carts.iter().any(|cart| {
                        cart.actor == actor && matches!(cart.state, CartState::Running(_))
                    });
                if !still_running {
                    self.follow = Follow::Idle;
                }
                CartFollowStep::Nothing
            }
            Follow::Ending {
                ride,
                actor,
                due_ms,
            } => {
                if facts.now_ms < due_ms {
                    return CartFollowStep::Nothing;
                }
                self.follow = Follow::Idle;
                if facts.phase != Some(ride.phase) {
                    return CartFollowStep::OtherMap;
                }
                let cart = actor
                    .and_then(|actor| facts.carts.iter().find(|cart| cart.actor == actor).copied())
                    .or_else(|| match_cart(facts.carts, ride.layout_id, ride.cart_position));
                match cart {
                    Some(cart) if cart.state != CartState::Parked(ride.to_state) => {
                        CartFollowStep::ParkAtEnd {
                            actor: cart.actor,
                            state: ride.to_state,
                        }
                    }
                    _ => CartFollowStep::Nothing,
                }
            }
        }
    }

    pub fn riding_actor(&self) -> Option<u64> {
        match self.follow {
            Follow::Riding { actor, .. } => Some(actor),
            _ => None,
        }
    }

    fn board(
        &mut self,
        facts: &OwnCartFacts,
        ride: CartRide,
        arrived_ms: u64,
        placed_ms: Option<u64>,
    ) -> CartFollowStep {
        if facts.phase != Some(ride.phase) {
            self.follow = Follow::Idle;
            return CartFollowStep::OtherMap;
        }
        let Some(cart) = match_cart(facts.carts, ride.layout_id, ride.cart_position) else {
            self.follow = Follow::Idle;
            return CartFollowStep::NoCartHere { ride };
        };
        if let CartState::Running(_) = cart.state {
            self.follow = Follow::Riding {
                actor: cart.actor,
                phase: ride.phase,
            };
            return CartFollowStep::Nothing;
        }
        let waited_ms = facts.now_ms.saturating_sub(arrived_ms);
        if placed_ms.is_none() && waited_ms >= CART_BOARD_GIVE_UP_MS {
            self.follow = Follow::Idle;
            return CartFollowStep::GaveUpBoarding { waited_ms };
        }
        if cart.state != CartState::Parked(ride.from_state) {
            return CartFollowStep::Park {
                actor: cart.actor,
                state: ride.from_state,
            };
        }
        let Some(placed_ms) = placed_ms else {
            if !facts.may_place_rider {
                return CartFollowStep::Nothing;
            }
            self.follow = Follow::Boarding {
                ride,
                arrived_ms,
                placed_ms: Some(facts.now_ms),
            };
            let [x, y, z] = cart.position;
            let [dx, dy, dz] = ride.rider_offset;
            return CartFollowStep::PlaceRider {
                actor: cart.actor,
                spot: [x + dx, y + dy + CART_RIDER_LIFT_UNITS, z + dz],
            };
        };
        if facts.now_ms.saturating_sub(placed_ms) < CART_BOARD_SETTLE_MS {
            return CartFollowStep::Nothing;
        }
        self.follow = Follow::Riding {
            actor: cart.actor,
            phase: ride.phase,
        };
        CartFollowStep::Start {
            actor: cart.actor,
            state: ride.to_state,
        }
    }
}
