use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RequestedRead {
    object_id: u32,
    phase_id: i32,
}

/// The object reads asked for so that extra bodies can be spawned, one per object and place.
#[derive(Debug, Default)]
pub struct DataRequests {
    requested: Vec<RequestedRead>,
}

impl DataRequests {
    pub fn ask(&mut self, calls: &GameCalls, object_id: u32, phase_id: i32) {
        let request = RequestedRead {
            object_id,
            phase_id,
        };
        if self.requested.contains(&request) {
            return;
        }
        let taken = calls.request_object_read(object_id);
        log_line(&format!(
            "extra bodies: asked the game to read object {object_id:#X} (taken: {taken})"
        ));
        if taken {
            self.requested.push(request);
        }
    }

    /// Gives back the read of the object once a body holds its own data.
    pub fn give_back(&mut self, calls: &GameCalls, object_id: u32, phase_id: i32) {
        let before = self.requested.len();
        self.requested
            .retain(|read| !(read.object_id == object_id && read.phase_id == phase_id));
        if self.requested.len() != before {
            calls.release_object_read(object_id);
        }
    }

    /// Forgets the reads of another place: after a scene change the read system holds other
    /// counts, and a count is left rather than taken twice.
    pub fn forget_other_places(&mut self, phase_id: i32) {
        self.requested.retain(|read| read.phase_id == phase_id);
    }
}
