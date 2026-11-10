use core::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use rc_game::GameMemory;
use rc_game::marker::UI_DIALOG_STATE_RVA;
use rc_game::offsets::{
    CLOSE_MAIN_MENU_FUNCTION_RVA, CLOSE_SHORT_SYSTEM_MENU_FUNCTION_RVA, FIND_UI_SCENE_FUNCTION_RVA,
    POP_UI_SCENE_SEQUENCE_FUNCTION_RVA,
};
use rc_game::ui_scenes::{PAUSE_MENU_SCENE_ID, UI_MANAGER_RVA, read_open_ui_scenes};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

type CloseMainMenu = unsafe extern "system" fn(*mut c_void, u8);
type CloseShortSystemMenu = unsafe extern "system" fn();
type FindUiScene = unsafe extern "system" fn(*mut c_void, u32) -> *mut c_void;
type PopUiSceneSequence = unsafe extern "system" fn(*mut c_void);

static CLOSE_RESULT_NOT_REPORTED_YET: AtomicBool = AtomicBool::new(false);

pub(crate) fn main_menu_open(build: &VerifiedBuild) -> bool {
    InProcessMemory::new(build)
        .read_i32(build.module_base() + u64::from(UI_DIALOG_STATE_RVA))
        .is_ok_and(|state| state != 0)
}

pub(crate) fn player_in_menu(build: &VerifiedBuild) -> bool {
    main_menu_open(build) || pause_menu_open(build)
}

pub(crate) fn close_main_menu(build: &VerifiedBuild, why: &str) {
    call_main_menu_close(build);
    log_line(&format!("menu: closed the pause menu ({why})"));
}

pub(crate) fn close_script_prompt_page(build: &VerifiedBuild) {
    call_main_menu_close(build);
}

fn call_main_menu_close(build: &VerifiedBuild) {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "calls the game's own main menu close at its verified address"
    )]
    // SAFETY: the build gate passed, so the RVA starts the menu close function (menu object, flag)
    // and the object address is the menu object; this runs on the game thread in the script tick.
    unsafe {
        let close: CloseMainMenu = core::mem::transmute::<usize, CloseMainMenu>(
            (base + u64::from(CLOSE_MAIN_MENU_FUNCTION_RVA)) as usize,
        );
        close((base + u64::from(UI_DIALOG_STATE_RVA)) as *mut c_void, 1);
    }
}

pub(crate) fn pause_menu_open(build: &VerifiedBuild) -> bool {
    read_open_ui_scenes(&InProcessMemory::new(build)).is_ok_and(|scenes| scenes.pause_menu_open())
}

fn open_scene_ids(build: &VerifiedBuild) -> Vec<u8> {
    read_open_ui_scenes(&InProcessMemory::new(build))
        .map(|scenes| scenes.scene_ids())
        .unwrap_or_default()
}

pub(crate) fn close_pause_menu(build: &VerifiedBuild, why: &str) {
    log_line(&format!(
        "pause menu: close asked (scenes before {:?}, {why})",
        open_scene_ids(build)
    ));
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "calls the game's own UI scene close functions at their verified addresses"
    )]
    // SAFETY: the build gate passed, so these RVAs start the short system menu close, the scene
    // lookup (manager, id) and the sequence pop (scene); this runs on the game thread.
    unsafe {
        let close_short_menu = (base + u64::from(CLOSE_SHORT_SYSTEM_MENU_FUNCTION_RVA)) as usize;
        let close_short_menu: CloseShortSystemMenu = core::mem::transmute(close_short_menu);
        close_short_menu();
        let find_scene = (base + u64::from(FIND_UI_SCENE_FUNCTION_RVA)) as usize;
        let find_scene: FindUiScene = core::mem::transmute(find_scene);
        let scene = find_scene(
            (base + u64::from(UI_MANAGER_RVA)) as *mut c_void,
            u32::from(PAUSE_MENU_SCENE_ID),
        );
        if !scene.is_null() {
            let pop_sequence = (base + u64::from(POP_UI_SCENE_SEQUENCE_FUNCTION_RVA)) as usize;
            let pop_sequence: PopUiSceneSequence = core::mem::transmute(pop_sequence);
            pop_sequence(scene);
        }
    }
    CLOSE_RESULT_NOT_REPORTED_YET.store(true, Ordering::Relaxed);
}

pub(crate) fn report_pause_menu_close_result(build: &VerifiedBuild) {
    if !CLOSE_RESULT_NOT_REPORTED_YET.swap(false, Ordering::Relaxed) {
        return;
    }
    let scenes = open_scene_ids(build);
    if pause_menu_open(build) {
        log_line(&format!(
            "pause menu: still open after the close call (scenes {scenes:?})"
        ));
    } else {
        log_line(&format!("pause menu: closed (scenes now {scenes:?})"));
    }
}
