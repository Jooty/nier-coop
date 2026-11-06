use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::panic;
use std::sync::Arc;
use std::thread;

use serde_json::json;

use crate::bridge::commands::Bridge;
use crate::bridge::{BRIDGE_BIND_ADDRESS, BRIDGE_PORT, MAX_REQUEST_LINE_BYTES};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;

pub fn start(build: VerifiedBuild) {
    let listener = match TcpListener::bind((BRIDGE_BIND_ADDRESS, BRIDGE_PORT)) {
        Ok(listener) => listener,
        Err(error) => {
            log_line(&format!(
                "bridge: cannot listen on port {BRIDGE_PORT} ({error})"
            ));
            return;
        }
    };
    let bridge = Arc::new(Bridge::new(build));
    let spawned = thread::Builder::new()
        .name("rc-bridge".to_owned())
        .spawn(move || accept_connections(&listener, &bridge));
    match spawned {
        Ok(_) => log_line(&format!(
            "bridge: listening on {BRIDGE_BIND_ADDRESS}:{BRIDGE_PORT}"
        )),
        Err(error) => log_line(&format!("bridge: cannot start thread ({error})")),
    }
}

fn accept_connections(listener: &TcpListener, bridge: &Arc<Bridge>) {
    for stream in listener.incoming().flatten() {
        let bridge = Arc::clone(bridge);
        let _ = thread::Builder::new()
            .name("rc-bridge-client".to_owned())
            .spawn(move || {
                if panic::catch_unwind(|| serve_connection(stream, &bridge)).is_err() {
                    log_line("bridge: client thread panicked");
                }
            });
    }
}

fn serve_connection(stream: TcpStream, bridge: &Bridge) {
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_half);
    let mut writer = stream;
    loop {
        let mut line = String::new();
        let bytes_read = reader
            .by_ref()
            .take(MAX_REQUEST_LINE_BYTES)
            .read_line(&mut line);
        let Ok(bytes_read) = bytes_read else {
            return;
        };
        if bytes_read == 0 {
            return;
        }
        let too_long = !line.ends_with('\n') && bytes_read as u64 >= MAX_REQUEST_LINE_BYTES;
        let reply = if too_long {
            json!({"ok": false, "error": "request line too long"})
        } else {
            bridge.handle_line(line.trim())
        };
        if writeln!(writer, "{reply}").is_err() || too_long {
            return;
        }
    }
}
