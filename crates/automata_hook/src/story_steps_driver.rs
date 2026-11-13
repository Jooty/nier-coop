mod driver;
mod shared_machine;

pub use driver::{install, tick};
pub use shared_machine::{
    consult_the_steps_machine_on_a_done, consult_the_steps_machine_on_leaving_done,
    tell_the_steps_machine_of_a_reset,
};
