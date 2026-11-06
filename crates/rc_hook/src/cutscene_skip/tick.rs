use std::sync::PoisonError;

use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::state::{
    TRACKER, is_connected, now_ms, observe_scene, partner_in_cutscene, partner_name, send_ask,
};
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::net;

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    observe_scene(build);
    let view = net.view();
    let asks = net.take_relayed(|message| matches!(message, Message::CutsceneSkip(_)));
    let now = now_ms();
    let mut tracker = TRACKER.lock().unwrap_or_else(PoisonError::into_inner);
    if !is_connected(&view) {
        tracker.partner_left();
        return;
    }
    tracker.observe_partner_in_cutscene(partner_in_cutscene(&view));
    for _ in asks {
        if tracker.partner_asked(now) {
            log_line("cutscene skip: the partner asks to skip");
            post_event(&format!(
                "{} wants to skip this cutscene",
                partner_name(&view)
            ));
        }
    }
    if tracker.resend_due(now) {
        send_ask(net);
    }
}
