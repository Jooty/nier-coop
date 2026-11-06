use rc_proto::{ENDING_ACCOMPLISHED_BYTES, EndingRecords};

use crate::story_progress::{
    ending_records_are_empty, ending_records_gained, merge_ending_records,
};

#[derive(Debug, Default)]
pub struct EndingRecordsSession {
    own: Option<EndingRecords>,
    host: Option<EndingRecords>,
    earned: EndingRecords,
}

impl EndingRecordsSession {
    pub fn is_armed(&self) -> bool {
        self.own.is_some()
    }

    pub fn arm(&mut self, own: EndingRecords) {
        self.own = Some(own);
        self.host = None;
        self.earned = EndingRecords {
            accomplished: vec![0; ENDING_ACCOMPLISHED_BYTES],
            ..EndingRecords::default()
        };
    }

    pub fn receive_host(&mut self, records: EndingRecords, together: bool) -> bool {
        if !self.is_armed() {
            return false;
        }
        let gained = match &self.host {
            Some(previous) => ending_records_gained(previous, &records),
            None => ending_records_gained(&records, &records),
        };
        self.host = Some(records);
        if !together || ending_records_are_empty(&gained) {
            return false;
        }
        self.earned = merge_ending_records(&self.earned, &gained);
        true
    }

    pub fn observe_live(&mut self, live: &EndingRecords) -> bool {
        let Some(known) = self.known_records() else {
            return false;
        };
        let gained = ending_records_gained(&known, live);
        if ending_records_are_empty(&gained) {
            return false;
        }
        self.earned = merge_ending_records(&self.earned, &gained);
        true
    }

    pub fn live_target(&self) -> Option<EndingRecords> {
        self.known_records()
    }

    pub fn saved_records(&self) -> Option<EndingRecords> {
        self.own
            .as_ref()
            .map(|own| merge_ending_records(own, &self.earned))
    }

    pub fn end_session(&mut self) -> Option<EndingRecords> {
        self.host.take()?;
        self.saved_records()
    }

    fn known_records(&self) -> Option<EndingRecords> {
        let kept = self.saved_records()?;
        Some(match &self.host {
            Some(host) => merge_ending_records(&kept, host),
            None => kept,
        })
    }
}
