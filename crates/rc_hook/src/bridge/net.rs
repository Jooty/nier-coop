use rc_game::PHASE_TABLE;
use serde_json::{Value, json};

use crate::bridge::commands::error_reply;
use crate::net::{self, NetCommand, NetShared};

pub(super) fn net_reply() -> Value {
    let Some(net) = net::handle() else {
        return error_reply("the net thread is not running");
    };
    net_view_json(&net.view())
}

pub(super) fn net_host_reply() -> Value {
    let Some(net) = net::handle() else {
        return error_reply("the net thread is not running");
    };
    net.send_command(NetCommand::Host);
    json!({"ok": true, "queued": true})
}

pub(super) fn net_join_reply(request: &Value) -> Value {
    let Some(ticket) = request.get("ticket").and_then(Value::as_str) else {
        return error_reply("missing string field \"ticket\"");
    };
    let Some(net) = net::handle() else {
        return error_reply("the net thread is not running");
    };
    net.send_command(NetCommand::Join(ticket.to_owned()));
    json!({"ok": true, "queued": true})
}

fn net_view_json(view: &NetShared) -> Value {
    let remote = view.remote.as_ref().map(|remote| {
        let phase = PHASE_TABLE
            .get(usize::from(remote.state.phase))
            .map(|entry| entry.name);
        json!({
            "name": remote.name,
            "slot": remote.slot,
            "phase": phase,
            "position": remote.state.position,
            "yaw": remote.state.yaw_radians,
            "hp": remote.state.hp,
            "hp_max": remote.state.max_hp,
            "animation_id": remote.state.animation_id,
            "animation_frame": remote.state.animation_seconds,
            "age_ms": remote.age().as_millis() as u64,
        })
    });
    json!({
        "ok": true,
        "state": view.state.label(),
        "ticket": view.ticket,
        "peer": view.peer_name,
        "rtt_ms": view.rtt.map(|rtt| rtt.as_secs_f64() * 1000.0),
        "path": view.direct_path.map(|direct| if direct { "direct" } else { "relay" }),
        "remote": remote,
    })
}
