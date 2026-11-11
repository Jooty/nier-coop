use core::ffi::c_void;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use replicant_game::Replicant;
use replicant_game::offsets::{PLAYER_PARAM_ADD_EXPERIENCE_FUNCTION_RVA, PLAYER_PARAM_ARRAY_RVA};
use replicant_game::prologue_levels::{LevelCatchUp, LocalLevel, PartnerLevel};
use replicant_proto::{Message, PrologueLevel};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net;

static CATCH_UP: LazyLock<Mutex<LevelCatchUp>> = LazyLock::new(Mutex::default);
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

const SHOW_LEVEL_UP: u8 = 1;

type AddExperience = unsafe extern "system" fn(*mut c_void, i32, u8);

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let mut catch_up = CATCH_UP.lock().unwrap_or_else(PoisonError::into_inner);
    for message in net.take_relayed(|message| matches!(message, Message::PrologueLevel(_))) {
        let Message::PrologueLevel(reported) = message else {
            continue;
        };
        let known = catch_up.partner();
        if known.is_none_or(|known| reported.experience >= known.experience) {
            catch_up.partner_reported(PartnerLevel {
                level: reported.level,
                experience: reported.experience,
            });
        }
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let (Ok(phase), Ok(level), Ok(experience)) =
        (game.current_phase(), game.level(), game.experience())
    else {
        return;
    };
    let local = LocalLevel { level, experience };
    let now_ms = u64::try_from(EPOCH.elapsed().as_millis()).unwrap_or(u64::MAX);
    let step = catch_up.step(now_ms, &phase, local);
    if let Some(sent) = step.send {
        net.send_relayed(Message::PrologueLevel(PrologueLevel {
            level: sent.level,
            experience: sent.experience,
        }));
    }
    let (Some(gain), Some(partner)) = (step.gain, catch_up.partner()) else {
        return;
    };
    add_experience_through_game(build, gain);
    let after = game.level().unwrap_or(level);
    log_line(&format!(
        "prologue levels: caught up +{gain} experience (local level {level} exp {experience}, partner level {} exp {}, local level now {after})",
        partner.level, partner.experience
    ));
}

#[expect(
    unsafe_code,
    reason = "calls the game's own experience adder at its verified address"
)]
fn add_experience_through_game(build: &VerifiedBuild, amount: i32) {
    let player_param = (build.module_base() + u64::from(PLAYER_PARAM_ARRAY_RVA)) as usize;
    let function =
        (build.module_base() + u64::from(PLAYER_PARAM_ADD_EXPERIENCE_FUNCTION_RVA)) as usize;
    // SAFETY: the RVA starts CPlayerParam::AddExperience (this, amount, redo level-up flag) in the
    // verified build; the array start is slot 0, the local player's CPlayerParam; this runs on the
    // game thread in the script tick.
    unsafe {
        core::mem::transmute::<usize, AddExperience>(function)(
            player_param as *mut c_void,
            amount,
            SHOW_LEVEL_UP,
        );
    }
}
