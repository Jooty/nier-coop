use rc_game::kaine::sync_kaine_stats;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

pub(crate) fn match_kaine_stats_to_protagonist(build: &VerifiedBuild) {
    let mut memory = InProcessMemory::new(build);
    match sync_kaine_stats(&mut memory) {
        Ok(Some(synced)) => log_line(&synced.log_line()),
        Ok(None) => {}
        Err(error) => log_line(&format!("kaine: cannot match her stats ({error})")),
    }
}
