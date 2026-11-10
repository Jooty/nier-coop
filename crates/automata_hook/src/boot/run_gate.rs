use nc_hook::diagnostics::log_line;

use crate::boot::exe_path::running_exe_path;
use crate::exe_facts::{facts_from_image, gate_report_line};

pub fn run_gate() {
    log_line("automata hook attached");
    let Some(path) = running_exe_path() else {
        log_line("exe path unknown; nothing installed");
        return;
    };
    let image = match std::fs::read(&path) {
        Ok(image) => image,
        Err(error) => {
            log_line(&format!(
                "cannot read {}: {error}; nothing installed",
                path.display()
            ));
            return;
        }
    };
    match facts_from_image(&image) {
        Ok(facts) => log_line(&gate_report_line(&facts)),
        Err(error) => log_line(&format!("exe header refused: {error}; nothing installed")),
    }
}
