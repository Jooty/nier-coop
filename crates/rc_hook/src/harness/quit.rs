use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use windows_sys::Win32::System::Threading::ExitProcess;
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

use crate::diagnostics::log_line;
use crate::harness::window_messages::game_window_handle;

const REPLY_FLUSH_DELAY: Duration = Duration::from_millis(300);
const CLOSE_GRACE: Duration = Duration::from_secs(4);
const CLEAN_EXIT_CODE: u32 = 0;

pub(super) fn quit_reply(request: &Value) -> Value {
    let force = request.get("force").and_then(Value::as_bool) == Some(true);
    let window = game_window_handle().map(|window| window as usize);
    let spawned = thread::Builder::new()
        .name("rc-quit".to_owned())
        .spawn(move || quit_game(window, force));
    match spawned {
        Ok(_) => json!({
            "ok": true,
            "quitting": true,
            "method": if force { "exit_process" } else { "wm_close_then_exit_process" },
        }),
        Err(error) => {
            json!({"ok": false, "error": format!("cannot start the quit thread ({error})")})
        }
    }
}

fn quit_game(window: Option<usize>, force: bool) {
    thread::sleep(REPLY_FLUSH_DELAY);
    if !force && let Some(window) = window {
        log_line("harness: quit requested, posting WM_CLOSE");
        #[expect(unsafe_code, reason = "asks the game window to close itself")]
        // SAFETY: PostMessageW only queues the message.
        unsafe {
            PostMessageW(window as _, WM_CLOSE, 0, 0)
        };
        thread::sleep(CLOSE_GRACE);
    }
    log_line("harness: quit, exiting the process");
    #[expect(
        unsafe_code,
        reason = "ends the game process when it did not close itself"
    )]
    // SAFETY: ExitProcess has no preconditions and does not return.
    unsafe {
        ExitProcess(CLEAN_EXIT_CODE)
    };
}
