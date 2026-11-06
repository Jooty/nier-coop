use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::story_progress::{
    ENDING_ACCOMPLISHED_HEADER_OFFSET, ENDING_CLEAR_BITS_HEADER_OFFSET,
    ENDING_COSTUMES_HEADER_OFFSET, ENDING_RECORDS_LIVE_READ_BYTES, EndingRecordsSession,
    FINISH_HOUR_ACCOMPLISHED_INDEX, LIVE_HEADER_RVA, apply_header_records, merge_ending_records,
    read_ending_records,
};
use rc_proto::{EndingRecords, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::memory::InProcessMemory;
use crate::net::NetThread;

static SESSION: LazyLock<Mutex<EndingRecordsSession>> = LazyLock::new(Mutex::default);
static SAVED_NOTICE_WAITING: AtomicBool = AtomicBool::new(false);

fn lock_session() -> std::sync::MutexGuard<'static, EndingRecordsSession> {
    SESSION.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn read_live_records(build: &VerifiedBuild) -> Option<EndingRecords> {
    let memory = InProcessMemory::new(build);
    let mut header = vec![0_u8; ENDING_RECORDS_LIVE_READ_BYTES];
    memory
        .read(
            build.module_base() + u64::from(LIVE_HEADER_RVA),
            &mut header,
        )
        .ok()?;
    read_ending_records(&header)
}

fn write_live_records(build: &VerifiedBuild, wanted: &EndingRecords, live: &EndingRecords) {
    let mut memory = InProcessMemory::new(build);
    let base = build.module_base() + u64::from(LIVE_HEADER_RVA);
    let mut write = |offset: usize, bytes: &[u8]| {
        if memory.write(base + offset as u64, bytes).is_err() {
            log_line(&format!(
                "story: could not store ending records at header offset {offset:#x}"
            ));
        }
    };
    if wanted.clear_bits != live.clear_bits {
        write(
            ENDING_CLEAR_BITS_HEADER_OFFSET,
            &wanted.clear_bits.to_le_bytes(),
        );
    }
    if wanted.costumes != live.costumes {
        write(ENDING_COSTUMES_HEADER_OFFSET, &wanted.costumes);
    }
    for (index, (&value, &current)) in wanted
        .accomplished
        .iter()
        .zip(&live.accomplished)
        .enumerate()
    {
        if value != current && index != FINISH_HOUR_ACCOMPLISHED_INDEX {
            write(ENDING_ACCOMPLISHED_HEADER_OFFSET + index, &[value]);
        }
    }
}

pub(super) fn host_send(
    build: &VerifiedBuild,
    net: &NetThread,
    last_sent: &mut Option<EndingRecords>,
    force: bool,
) {
    let Some(live) = read_live_records(build) else {
        return;
    };
    if !force && last_sent.as_ref() == Some(&live) {
        return;
    }
    log_line(&format!(
        "story: host sent its ending records (clear bits {:#x})",
        live.clear_bits
    ));
    *last_sent = Some(live.clone());
    net.send_relayed(Message::EndingRecords(live));
}

pub(super) fn arm_once(build: &VerifiedBuild) {
    let mut session = lock_session();
    if session.is_armed() {
        return;
    }
    let Some(own) = read_live_records(build) else {
        return;
    };
    log_line(&format!(
        "story: the guest's own ending records at join: clear bits {:#x}, {} costume bytes set, {} accomplished bytes set",
        own.clear_bits,
        count_set(&own.costumes),
        count_set(&own.accomplished)
    ));
    session.arm(own);
}

pub(super) fn receive_host(records: EndingRecords, together: bool) {
    let bits = records.clear_bits;
    let earned = lock_session().receive_host(records, together);
    log_line(&format!(
        "story: host's ending records arrived (clear bits {bits:#x}, together {together}, earned {earned})"
    ));
    if earned {
        SAVED_NOTICE_WAITING.store(true, Ordering::Release);
    }
}

pub(super) fn sync_live(build: &VerifiedBuild) {
    let Some(live) = read_live_records(build) else {
        return;
    };
    let wanted = {
        let mut session = lock_session();
        if session.observe_live(&live) {
            SAVED_NOTICE_WAITING.store(true, Ordering::Release);
            log_line("story: the guest's own script set an ending record, it is kept");
        }
        session.live_target()
    };
    if let Some(wanted) = wanted {
        let merged = merge_ending_records(&live, &wanted);
        if merged != live {
            write_live_records(build, &merged, &live);
        }
    }
    if SAVED_NOTICE_WAITING.swap(false, Ordering::AcqRel) {
        post_event("Ending records saved to your co-op save");
    }
}

pub(super) fn end_session(build: &VerifiedBuild) {
    let Some(live) = read_live_records(build) else {
        return;
    };
    let kept = {
        let mut session = lock_session();
        session.observe_live(&live);
        session.end_session()
    };
    let Some(kept) = kept else {
        return;
    };
    log_line(&format!(
        "story: session over, the guest's live ending records go back to its own (clear bits {:#x})",
        kept.clear_bits
    ));
    write_live_records(build, &kept, &live);
}

pub(super) fn apply_to_save(file: &mut [u8]) {
    let mut session = lock_session();
    if !session.is_armed() {
        return;
    }
    let Some(written) = read_ending_records(file) else {
        return;
    };
    if session.observe_live(&written) {
        SAVED_NOTICE_WAITING.store(true, Ordering::Release);
    }
    let Some(saved) = session.saved_records() else {
        return;
    };
    match apply_header_records(file, &saved) {
        Ok(true) => log_line(&format!(
            "story: co-op save header ending records set (clear bits {:#x} was {:#x}), header checksum fixed",
            saved.clear_bits, written.clear_bits
        )),
        Ok(false) => {}
        Err(_) => log_line("story: co-op save header too short for ending records, left alone"),
    }
}

fn count_set(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&byte| byte != 0).count()
}
