use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rc_bot::{BotArgs, parse_args};
use rc_launcher::net::{LauncherCore, LauncherNet, NetState};
use rc_launcher_core::{LauncherConfig, iroh_factory};
use rc_net::IrohConfig;

const WAIT_LIMIT: Duration = Duration::from_secs(20);
const POLL_STEP: Duration = Duration::from_millis(50);

fn local_core() -> Arc<LauncherCore> {
    let factory = iroh_factory(IrohConfig {
        use_relays_and_lookup: false,
    });
    Arc::new(LauncherCore::new(LauncherConfig::new("Hosty"), factory).expect("core"))
}

fn bot_joining(ticket: &str, password: &str) -> BotArgs {
    let line = format!("--join {ticket} --no-relay --seconds 8 --password {password}");
    parse_args(line.split_whitespace().map(str::to_owned)).expect("bot args")
}

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < WAIT_LIMIT {
        if condition() {
            return true;
        }
        thread::sleep(POLL_STEP);
    }
    false
}

#[test]
fn a_bot_with_the_right_password_joins_a_password_protected_launcher_host() {
    let core = local_core();
    let ticket = core.host(Some("hunter2")).expect("ticket");
    let args = bot_joining(&ticket, "hunter2");
    let bot = thread::spawn(move || rc_bot::run(&args));
    assert!(wait_until(|| matches!(
        core.status().state,
        NetState::Connected { .. }
    )));
    assert!(bot.join().expect("bot thread").is_ok());
}

#[test]
fn a_bot_with_the_wrong_password_is_rejected_by_the_launcher_host() {
    let core = local_core();
    let ticket = core.host(Some("hunter2")).expect("ticket");
    let args = bot_joining(&ticket, "nope");
    let result = thread::spawn(move || rc_bot::run(&args))
        .join()
        .expect("bot thread");
    assert!(result.is_err());
    assert!(!matches!(core.status().state, NetState::Connected { .. }));
}
