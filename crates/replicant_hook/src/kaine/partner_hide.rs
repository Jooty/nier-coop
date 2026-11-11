use std::sync::{Mutex, PoisonError};

use replicant_game::GameMemory;
use replicant_game::kaine::{
    PARTNER_EXECUTES_FLAG_OFFSET, PartnerHideStep, PartnerKaineHide, partner_kaine_hidden_wanted,
};
use replicant_game::offsets::{ACTOR_DRAWN_FLAG_OFFSET, PARTNER_KAINE_ACTOR_RVA};
use replicant_proto::PartnerBody;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::player_body::real_body;

static HIDE: Mutex<PartnerKaineHide> = Mutex::new(PartnerKaineHide::new());

/// Hides the AI partner Kaine while the local player or the partner plays as her, and shows her
/// again when that stops; called once per ghost tick.
pub(crate) fn hide_partner_kaine(build: &VerifiedBuild, remote_body: Option<PartnerBody>) {
    let mut memory = InProcessMemory::new(build);
    let actor = memory.module_base() + u64::from(PARTNER_KAINE_ACTOR_RVA);
    let (Ok(executes), Ok(drawn)) = (
        memory.read_u8(actor + PARTNER_EXECUTES_FLAG_OFFSET),
        memory.read_u8(actor + ACTOR_DRAWN_FLAG_OFFSET),
    ) else {
        return;
    };
    let wanted = partner_kaine_hidden_wanted(real_body().map(|real| real.body), remote_body);
    let mut hide = HIDE.lock().unwrap_or_else(PoisonError::into_inner);
    let was_hidden = hide.hidden_by_us();
    match hide.step(wanted, executes != 0, drawn != 0) {
        PartnerHideStep::Hide => {
            if memory.write_u8(actor + ACTOR_DRAWN_FLAG_OFFSET, 0).is_err() {
                log_line("kaine: cannot write the AI partner's drawn byte");
            } else if !was_hidden {
                log_line(
                    "kaine: AI partner hidden (drawn byte 0, her AI keeps running), \
                     re-applied while the game shows her",
                );
            }
        }
        PartnerHideStep::ShowAgain => {
            let _ = memory.write_u8(actor + ACTOR_DRAWN_FLAG_OFFSET, 1);
            log_line("kaine: nobody plays as her any more, AI partner shown again");
        }
        PartnerHideStep::Leave => {
            if was_hidden && !hide.hidden_by_us() {
                log_line("kaine: hide over, the game had already stopped the AI partner");
            }
        }
    }
}
