use core::ffi::c_void;

use replicant_game::GameMemory;
use replicant_game::dialogue::{
    CONTROL_MASK_SET_SLOT_ZERO_RVA, GAME_STATE_RVA, TALK_PLAYER_NEXT_LINE_RVA,
    TALK_WINDOW_CLOSE_RVA, TALK_WINDOW_CURRENT_LINE_OFFSET, TALK_WINDOW_MASK_BEFORE_OPEN_OFFSET,
    TALK_WINDOW_OPEN_RVA, TALK_WINDOW_PLAYER_OFFSET, TALK_WINDOW_RVA,
};
use replicant_proto::DialogueLine;

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

type OpenTalk = unsafe extern "system" fn(*mut c_void, i32);
type SetMaskSlot = unsafe extern "system" fn(*mut c_void, u32);
type NextLine = unsafe extern "system" fn(*mut c_void) -> i32;
type CloseTalk = unsafe extern "system" fn(*mut c_void, u8);

/// Opens this game's talk window on the partner's talk without taking this player's controls,
/// then moves it to the partner's line. Runs on the game thread. False when the game refused.
#[expect(
    unsafe_code,
    reason = "calls TalkWindow::Open and the control mask setter by address"
)]
pub(super) fn open_mirror(build: &VerifiedBuild, line: DialogueLine) -> bool {
    let mut memory = InProcessMemory::new(build);
    let base = memory.module_base();
    let window = base + u64::from(TALK_WINDOW_RVA);
    let Ok(talk_id) = i32::try_from(line.talk_id) else {
        return false;
    };
    // SAFETY: TalkWindow::Open(window, talkId) in the verified build, called on the game thread
    // like `_StartTalk` does.
    unsafe {
        core::mem::transmute::<usize, OpenTalk>(address(base, TALK_WINDOW_OPEN_RVA))(
            window as *mut c_void,
            talk_id,
        );
    }
    if memory.read_i32(window).is_ok_and(|state| state == 0) {
        return false;
    }
    // Open ORs 0x20 into mask slot 0 (pausing this player's systems); put back what it saved.
    if let Ok(mask_before) = memory.read_i32(window + TALK_WINDOW_MASK_BEFORE_OPEN_OFFSET) {
        // SAFETY: the slot 0 setter (game state, value) in the verified build, game thread.
        unsafe {
            core::mem::transmute::<usize, SetMaskSlot>(address(
                base,
                CONTROL_MASK_SET_SLOT_ZERO_RVA,
            ))(
                (base + u64::from(GAME_STATE_RVA)) as *mut c_void,
                mask_before.cast_unsigned(),
            );
        }
    }
    if line.line > 0 {
        show_line(&mut memory, line.line);
    }
    true
}

/// Moves the open talk to `line` the way the game's own "next line" step does.
#[expect(unsafe_code, reason = "calls TalkPlayer::NextLine by address")]
pub(super) fn show_line(memory: &mut InProcessMemory, line: u16) {
    let base = memory.module_base();
    let window = base + u64::from(TALK_WINDOW_RVA);
    if memory
        .write_i32(
            window + TALK_WINDOW_CURRENT_LINE_OFFSET,
            i32::from(line) - 1,
        )
        .is_err()
    {
        return;
    }
    // SAFETY: TalkPlayer::NextLine(player) in the verified build starts line (index + 1);
    // the player is the window's own talk player, called on the game thread.
    unsafe {
        core::mem::transmute::<usize, NextLine>(address(base, TALK_PLAYER_NEXT_LINE_RVA))(
            (window + TALK_WINDOW_PLAYER_OFFSET) as *mut c_void,
        );
    }
}

#[expect(unsafe_code, reason = "calls TalkPlayer::NextLine by address")]
pub(super) fn step_past_line(build: &VerifiedBuild) {
    let base = build.module_base();
    let window = base + u64::from(TALK_WINDOW_RVA);
    // SAFETY: TalkPlayer::NextLine(player) in the verified build, called on the game thread; past
    // the last line it starts the talk that follows, as the talker's own step does.
    unsafe {
        core::mem::transmute::<usize, NextLine>(address(base, TALK_PLAYER_NEXT_LINE_RVA))(
            (window + TALK_WINDOW_PLAYER_OFFSET) as *mut c_void,
        );
    }
}

#[expect(unsafe_code, reason = "calls TalkWindow::Close by address")]
pub(super) fn close_mirror(build: &VerifiedBuild) {
    let base = build.module_base();
    // SAFETY: TalkWindow::Close(window, immediately) in the verified build, game thread.
    unsafe {
        core::mem::transmute::<usize, CloseTalk>(address(base, TALK_WINDOW_CLOSE_RVA))(
            (base + u64::from(TALK_WINDOW_RVA)) as *mut c_void,
            1,
        );
    }
}

fn address(base: u64, rva: u32) -> usize {
    (base + u64::from(rva)) as usize
}
