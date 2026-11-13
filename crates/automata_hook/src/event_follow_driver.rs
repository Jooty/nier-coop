mod driver;
mod shared_machine;

pub use driver::{install, tick};
pub use shared_machine::{
    consult_the_event_machine_on_a_play, consult_the_event_machine_on_a_skip,
    tell_the_event_machine_of_a_start, tell_the_event_machine_of_an_end,
};
