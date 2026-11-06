use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::fade_sync::{
    FADE_ACTIVE_OFFSET, FADE_ALPHA_OFFSET, FADE_LAYER_RVA, FadeEdge, FadeWatch, GuestFadeState,
};
use rc_game::stay_together::FollowRole;
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fade_sync::hooks::{call_fade, now_ms};
use crate::memory::InProcessMemory;
use crate::net;

const BACKSTOP_FADE_IN_KIND: i32 = 2;
const BACKSTOP_FADE_IN_SECONDS: f32 = 1.0;
const BACKSTOP_FADE_IN_DEPTH: u32 = 0x20;

static GUEST: Mutex<GuestFadeState> = Mutex::new(GuestFadeState::new());
static WATCH: Mutex<FadeWatch> = Mutex::new(FadeWatch::new());
static GUEST_HOLDS_FADE: AtomicBool = AtomicBool::new(false);

pub fn guest_holds_fade() -> bool {
    GUEST_HOLDS_FADE.load(Ordering::Relaxed)
}

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let role = FollowRole::from_slot(view.local_slot);
    watch_layer(build, role);
    if role != FollowRole::Guest {
        return;
    }
    let mut guest = GUEST.lock().unwrap_or_else(PoisonError::into_inner);
    for message in net.take_relayed(|message| matches!(message, Message::Fade(_))) {
        let Message::Fade(fade) = message else {
            continue;
        };
        if !call_fade(
            fade.fading_out,
            i32::from(fade.kind),
            fade.seconds,
            fade.depth,
        ) {
            continue;
        }
        log_line(&format!(
            "fade: guest mirrors the host's fade {} (kind {}, {:.2} s, depth {:#x})",
            if fade.fading_out { "out" } else { "in" },
            fade.kind,
            fade.seconds,
            fade.depth
        ));
        if fade.fading_out {
            guest.started_fade_out(now_ms());
        } else {
            guest.started_fade_in();
        }
    }
    if guest.backstop_due(now_ms())
        && call_fade(
            false,
            BACKSTOP_FADE_IN_KIND,
            BACKSTOP_FADE_IN_SECONDS,
            BACKSTOP_FADE_IN_DEPTH,
        )
    {
        log_line("fade: guest fade-in backstop, the host's fade-in never arrived");
    }
    GUEST_HOLDS_FADE.store(guest.holds_fade(), Ordering::Relaxed);
}

fn watch_layer(build: &VerifiedBuild, role: FollowRole) {
    let memory = InProcessMemory::new(build);
    let layer = build.module_base() + u64::from(FADE_LAYER_RVA);
    let Ok(active) = memory.read_u8(layer + FADE_ACTIVE_OFFSET) else {
        return;
    };
    let alpha = memory.read_f32(layer + FADE_ALPHA_OFFSET).unwrap_or(0.0);
    let edge = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(active != 0);
    let side = if role == FollowRole::Guest {
        "guest"
    } else {
        "host"
    };
    match edge {
        FadeEdge::Started => log_line(&format!(
            "fade: {side} screen fade started (alpha {alpha:.0})"
        )),
        FadeEdge::Finished => log_line(&format!(
            "fade: {side} screen fade finished (alpha {alpha:.0})"
        )),
        FadeEdge::None => {}
    }
}
