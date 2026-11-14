use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::dialogue::{
    CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS, ChoiceFollowInput, ChoiceFollower,
    ChoiceStep, DialogueTracker, FollowClock, HostLedScriptTalk, MirrorAction, MirrorInput,
    OwnTalkWindow, PartnerTalkMemory, PartnerTalkMirror, ScreenReport,
    TALK_SCREEN_REPORT_AFTER_OPEN_MS, TALK_WINDOW_RVA, TalkLeader, TalkOriginTracker,
    WATCHER_HOLD_MASK, WatcherHold, WatcherHoldAction, WatcherHoldInput, find_talk_id,
    host_answer_fits, leader_for_own_talk, own_talk_started, put_may_show_flag_back,
    read_choice_prompt, read_talk_cursor, read_talk_window_screen, within_talk_range,
};
use replicant_game::marker::{WorldFlags, read_world_flags};
use replicant_game::readiness::{classify, read_play_checks};
use replicant_game::stay_together::{FollowRole, local_phase_index};
use replicant_game::{GameMemory, Replicant};
use replicant_proto::{DialogueLine, DialogueUpdate, Message, PartnerActivity};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::dialogue::choice_hover::menu_covers_the_talk_window;
use crate::dialogue::hooks::{FOLLOWING_HOST_TALK, MIRRORING, take_draw_pass_counts};
use crate::dialogue::tags::MirrorActionGuard;
use crate::dialogue::talker_name::begin_talk;
use crate::dialogue::watcher_hold::{
    forget_mirror_wrote_slot_one, read_slot_one, slot_one_written_by_mirror,
    take_other_code_wrote_slot_one, write_slot_one,
};
use crate::dialogue::window_calls::{close_mirror, open_mirror, show_line, step_past_line};
use crate::memory::InProcessMemory;
use crate::net::NetSharedExt;
use crate::net::{self, NetShared, NetThread, net_clock_ms};
use crate::stay_together::pause_menu_open;

static TRACKER: Mutex<DialogueTracker> = Mutex::new(DialogueTracker::new());
static MIRROR: Mutex<PartnerTalkMirror> = Mutex::new(PartnerTalkMirror::new());
static OWN_TALK_WINDOW_WAS_OPEN: AtomicBool = AtomicBool::new(false);
static PARTNER_TALK: Mutex<PartnerTalkMemory> = Mutex::new(PartnerTalkMemory::new());
static OWN_TALK_STARTED_BY_PLAYER_COMMAND: AtomicBool = AtomicBool::new(false);
static CHOICE_FOLLOW: Mutex<ChoiceFollower> = Mutex::new(ChoiceFollower::new());
static HOLD: Mutex<WatcherHold> = Mutex::new(WatcherHold::new());
static FOLLOW_CLOCK: Mutex<FollowClock> = Mutex::new(FollowClock::new());
static HOST_LED_SCRIPT_TALK: Mutex<HostLedScriptTalk> = Mutex::new(HostLedScriptTalk::new());
static TALK_ORIGIN: Mutex<TalkOriginTracker> = Mutex::new(TalkOriginTracker::new());
static PARTNER_STORY_TALK_NOTED: Mutex<Option<u32>> = Mutex::new(None);
static SCREEN_REPORT: Mutex<ScreenReport> = Mutex::new(ScreenReport::new());
static OWN_TALK_WINDOW: Mutex<OwnTalkWindow> = Mutex::new(OwnTalkWindow::new());

pub fn tick_dialogue(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        release_hold_without_link(build);
        return;
    };
    let view = net.view();
    let game = Replicant::new(InProcessMemory::new(build));
    let memory = game.memory();
    let base = memory.module_base();
    let flags = read_world_flags(memory).ok();
    let follow_now_ms = FOLLOW_CLOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .now(net_clock_ms(), menu_covers_the_talk_window(memory));
    crate::dialogue::choices::receive_partner_choices(
        net,
        &mut CHOICE_FOLLOW.lock().unwrap_or_else(PoisonError::into_inner),
        follow_now_ms,
    );
    crate::dialogue::choice_hover::receive_partner_hovers(net);
    receive_partner_lines(net);
    crate::dialogue::choice_hover::partner_moved_to(
        PARTNER_TALK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .current_line(),
    );
    let window_open = memory
        .read_i32(base + u64::from(TALK_WINDOW_RVA))
        .is_ok_and(|state| state != 0);
    let slot_one_mask = read_slot_one(build);
    let taken_over = take_other_code_wrote_slot_one();
    let holding = HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_holding();
    let started_own_talk = own_talk_started(
        slot_one_mask,
        slot_one_written_by_mirror(),
        holding,
        taken_over,
    );
    let own_window_open = window_open && (started_own_talk || !MIRRORING.load(Ordering::Acquire));
    if OWN_TALK_WINDOW_WAS_OPEN.swap(own_window_open, Ordering::Relaxed) != own_window_open {
        let by_player_command = own_window_open && crate::cutscene_skip::take_player_talk_command();
        OWN_TALK_STARTED_BY_PLAYER_COMMAND.store(by_player_command, Ordering::Release);
        log_line(&format!(
            "dialogue: your talk window {} (own talk started: {started_own_talk})",
            if own_window_open { "opened" } else { "closed" }
        ));
    }
    let own_copy_open = OWN_TALK_WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(own_window_open, started_own_talk);
    let own_talk_line = share_own_talk(net, &view, memory, own_window_open);
    let mut mirror = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);
    let role = FollowRole::from_slot(view.local_slot);
    let same_phase = view
        .partner_state()
        .is_some_and(|remote| local_phase_index(&game) == Some(remote.state.phase));
    let partner_story_line = PARTNER_TALK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .story_line()
        .filter(|_| role == FollowRole::Guest);
    let in_range = view.partner_state().is_some_and(|remote| {
        same_phase
            && game.position().is_ok_and(|local| {
                let [x, y, z] = remote.state.position;
                let distance =
                    ((local.x - x).powi(2) + (local.y - y).powi(2) + (local.z - z).powi(2)).sqrt();
                within_talk_range(distance, mirror.is_showing())
            })
    });
    let free_to_show = flags
        .is_some_and(|flags| !flags.event_scene && !flags.main_menu_open && !flags.world_frozen())
        && classify(read_play_checks(&game)).is_in_play();
    // A story step's talk is shown from any distance in the same area.
    let can_show = (in_range || (partner_story_line.is_some() && same_phase)) && free_to_show;
    let own_talk = own_talk_line.filter(|_| own_copy_open);
    let now_ms = net_clock_ms();
    let started_by_player_command = OWN_TALK_STARTED_BY_PLAYER_COMMAND.load(Ordering::Acquire);
    let host_leads_from_afar = host_leads_script_talk_from_afar(
        &game,
        &view,
        role,
        own_talk,
        started_by_player_command,
        now_ms,
    );
    let leader = match role {
        _ if !view.partner_in_play() || !(in_range || host_leads_from_afar) => TalkLeader::Either,
        FollowRole::Host => TalkLeader::Local,
        FollowRole::Guest => TalkLeader::Partner,
        FollowRole::Unknown => TalkLeader::Either,
    };
    let leader = leader_for_own_talk(leader, started_by_player_command);
    let (answer_open, choice_gave_up) = follow_host_answer(
        &game,
        own_talk,
        leader == TalkLeader::Partner,
        follow_now_ms,
    );
    let (partner_line, partner_ended_talk) = {
        let memory = PARTNER_TALK.lock().unwrap_or_else(PoisonError::into_inner);
        (
            memory.for_mirror(own_talk, leader, role == FollowRole::Guest),
            memory.ended_for_follow(own_talk, leader, now_ms),
        )
    };
    let action = mirror.step(MirrorInput {
        partner_line,
        partner_ended_talk,
        can_show,
        window_open,
        own_talk,
        leader,
        answer_open,
        now_ms,
    });
    if choice_gave_up {
        mirror.release_followed_talk();
    }
    note_partner_story_talk(
        partner_story_line,
        own_talk,
        action,
        window_open,
        same_phase,
        free_to_show,
    );
    if let Some(action) = action {
        apply(build, action);
    }
    FOLLOWING_HOST_TALK.store(
        own_talk.is_some() && mirror.holds_own_talk(),
        Ordering::Release,
    );
    crate::dialogue::choice_hover::share_own_hover(net, memory, own_talk, view.partner_in_play());
    let window_still_open = memory
        .read_i32(base + u64::from(TALK_WINDOW_RVA))
        .is_ok_and(|state| state != 0);
    let mirror_showing = mirror.is_showing() && window_still_open;
    if !window_still_open {
        forget_mirror_wrote_slot_one();
    }
    MIRRORING.store(mirror_showing, Ordering::Release);
    drop(mirror);
    let talk_on_screen = if mirror_showing {
        PARTNER_TALK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .current_line()
    } else {
        own_talk_line.filter(|_| own_window_open && window_still_open)
    };
    report_what_is_on_screen(
        build,
        talk_on_screen.map(|line| line.talk_id),
        mirror_showing,
    );
    let hold_action = HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .step(WatcherHoldInput {
            mirror_showing,
            slot_one_mask,
            taken_over,
        });
    apply_hold(build, hold_action, view.partner_name());
}

fn report_what_is_on_screen(build: &VerifiedBuild, talk_id: Option<u32>, mirrored: bool) {
    let mut memory = InProcessMemory::new(build);
    let Ok(screen) = read_talk_window_screen(&memory) else {
        return;
    };
    if let Some(talk_id) = talk_id
        && screen.may_show_flag_left_off(pause_menu_open(build))
        && put_may_show_flag_back(&mut memory).is_ok()
    {
        log_line(&format!(
            "dialogue: talk {talk_id} was open but hidden: the window's may-show flag was off with no menu up (a menu's exit never ran), put back on"
        ));
    }
    let report = SCREEN_REPORT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(
            talk_id,
            screen.verdict(),
            net_clock_ms(),
            TALK_SCREEN_REPORT_AFTER_OPEN_MS,
        );
    if talk_id.is_none() {
        // The counts of a report cover the frames of that talk only.
        take_draw_pass_counts();
    }
    if let Some((talk_id, _)) = report {
        let draws = take_draw_pass_counts();
        log_line(&format!(
            "dialogue: on screen here, talk {talk_id} ({}): {}; the UI draw pass ran {} frames, asked the window {} times, was told a talk is up {} times",
            if mirrored {
                "the partner's"
            } else {
                "your window's"
            },
            screen.describe(),
            draws.passes,
            draws.talk_checks,
            draws.talk_busy
        ));
    }
}

pub(crate) fn partner_talk_state(talk_id: u32) -> (bool, bool) {
    let memory = PARTNER_TALK.lock().unwrap_or_else(PoisonError::into_inner);
    (
        memory.is_on(talk_id),
        memory.ended_within_follow_wait(talk_id, net_clock_ms()),
    )
}

/// Whether the partner's talk window shows any talk now, and whether it closed within the follow wait.
pub(crate) fn partner_talk_window_state() -> (bool, bool) {
    let memory = PARTNER_TALK.lock().unwrap_or_else(PoisonError::into_inner);
    (
        memory.any_talk_on(),
        memory.any_talk_ended_within_follow_wait(net_clock_ms()),
    )
}

pub(crate) fn mirror_showing() -> bool {
    MIRRORING.load(Ordering::Acquire)
}

pub(crate) fn watcher_hold_owns_talk_slot() -> bool {
    HOLD.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_holding()
}

fn host_leads_script_talk_from_afar(
    game: &Replicant<InProcessMemory>,
    view: &NetShared,
    role: FollowRole,
    own_talk: Option<DialogueLine>,
    started_by_player_command: bool,
    now_ms: u64,
) -> bool {
    let host_on_same_talk = role == FollowRole::Guest
        && view
            .partner_state()
            .is_some_and(|remote| local_phase_index(game) == Some(remote.state.phase))
        && own_talk.is_some_and(|own| {
            PARTNER_TALK
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_or_just_was_on(own.talk_id, now_ms)
        });
    let mut led = HOST_LED_SCRIPT_TALK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let was_led = led.is_led();
    let is_led = led.observe(own_talk, started_by_player_command, host_on_same_talk);
    if is_led
        && !was_led
        && let Some(own) = own_talk
    {
        log_line(&format!(
            "dialogue: a script opened talk {} here while the host is on it too, the host leads your copy from any distance",
            own.talk_id
        ));
    }
    is_led
}

fn note_partner_story_talk(
    partner_story_line: Option<DialogueLine>,
    own_talk: Option<DialogueLine>,
    action: Option<MirrorAction>,
    window_open: bool,
    same_phase: bool,
    free_to_show: bool,
) {
    let mut noted = PARTNER_STORY_TALK_NOTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let Some(story) = partner_story_line else {
        *noted = None;
        return;
    };
    if *noted == Some(story.talk_id) {
        return;
    }
    let opened = matches!(action, Some(MirrorAction::Open(line)) if line.talk_id == story.talk_id);
    let shown = if opened {
        "it opens in your talk window and follows the host's lines".to_owned()
    } else if let Some(own) = own_talk {
        if own.talk_id == story.talk_id {
            "your game opened its own copy, it follows the host's lines".to_owned()
        } else {
            format!(
                "not shown, your own talk {} is open and stays yours",
                own.talk_id
            )
        }
    } else if window_open {
        "not shown yet, a talk window is open here".to_owned()
    } else if !same_phase {
        "not shown, you are in another area".to_owned()
    } else if !free_to_show {
        "not shown yet, a menu, scene or loading is up here or you are not in play".to_owned()
    } else {
        "not shown yet, the window opens on a later tick".to_owned()
    };
    log_line(&format!(
        "dialogue: the host's story talk {}: {shown}",
        story.talk_id
    ));
    *noted = Some(story.talk_id);
}

/// A talk start native ran inside the script call `call`; `shared_step` when the call is a step
/// of a sequence both players are in, whose talk the partner is shown.
pub(crate) fn note_script_talk_start(call: &str, shared_step: bool) {
    let mut origin = TALK_ORIGIN.lock().unwrap_or_else(PoisonError::into_inner);
    if shared_step {
        origin.note_shared_step_talk_start(call, net_clock_ms());
    } else {
        origin.note_script_talk_start(call, net_clock_ms());
    }
}

fn follow_host_answer(
    game: &Replicant<InProcessMemory>,
    own_talk: Option<DialogueLine>,
    follows_host: bool,
    now_ms: u64,
) -> (bool, bool) {
    let memory = game.memory();
    let prompt = read_choice_prompt(memory)
        .filter(|_| own_talk.is_some())
        .map(|prompt| prompt.kind);
    let mut follower = CHOICE_FOLLOW.lock().unwrap_or_else(PoisonError::into_inner);
    let step = follower.step(ChoiceFollowInput {
        follows_host,
        own_talk,
        prompt,
        now_ms,
    });
    let mut gave_up = false;
    match step {
        Some(ChoiceStep::Take(choice)) => {
            log_line(&format!(
                "choice: the host picked answer {} of talk {}, taking it here",
                choice.index, choice.talk_id
            ));
            if !host_answer_fits(memory, &choice) {
                follower.forget();
                gave_up = true;
                log_line(&format!(
                    "choice: answer {} of talk {} is not an option of your prompt, your buttons are back",
                    choice.index, choice.talk_id
                ));
            } else {
                crate::dialogue::choices::arm_answer(choice);
            }
        }
        Some(ChoiceStep::GiveUp(choice)) => {
            crate::dialogue::choices::disarm_answer();
            gave_up = true;
            log_line(&format!(
                "choice: could not take answer {} of talk {} within {CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS} ms, your buttons are back",
                choice.index, choice.talk_id
            ));
        }
        Some(ChoiceStep::Drop(choice)) => {
            crate::dialogue::choices::disarm_answer();
            log_line(&format!(
                "choice: dropped answer {} of talk {}, not taking it here",
                choice.index, choice.talk_id
            ));
        }
        None => {}
    }
    if !follower.is_waiting() {
        crate::dialogue::choices::disarm_answer();
    }
    (prompt.is_some() || follower.is_waiting(), gave_up)
}

fn release_hold_without_link(build: &VerifiedBuild) {
    CHOICE_FOLLOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget();
    crate::dialogue::choices::disarm_answer();
    crate::dialogue::choice_hover::forget_hovers();
    PARTNER_TALK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget();
    FOLLOWING_HOST_TALK.store(false, Ordering::Release);
    let action = HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .release_on_link_loss(read_slot_one(build));
    apply_hold(build, action, None);
}

fn apply_hold(build: &VerifiedBuild, action: Option<WatcherHoldAction>, partner: Option<&str>) {
    let partner = partner.unwrap_or("your partner");
    match action {
        Some(WatcherHoldAction::Hold) => {
            if !write_slot_one(build, WATCHER_HOLD_MASK) {
                HOLD.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .abandon();
                log_line(
                    "dialogue: could not hold you during the partner's talk, the game's mask setter is not hooked",
                );
                return;
            }
            log_line(&format!("dialogue: holding you during {partner}'s talk"));
        }
        Some(WatcherHoldAction::Release) => {
            write_slot_one(build, 0);
            log_line(&format!("dialogue: released from {partner}'s talk"));
        }
        None => {}
    }
}

/// This player's own talk window is open (a window that only mirrors the partner's talk is not).
pub(crate) fn own_talk_window_open() -> bool {
    OWN_TALK_WINDOW_WAS_OPEN.load(Ordering::Relaxed)
}

pub(super) fn own_talk_line() -> Option<DialogueLine> {
    TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .current_line()
}

/// This player's activity for the snapshot: a window that only mirrors the partner's talk
/// does not make this player "in dialogue".
pub fn local_activity(flags: WorldFlags) -> PartnerActivity {
    WorldFlags {
        talk_window_open: flags.talk_window_open && !MIRRORING.load(Ordering::Acquire),
        ..flags
    }
    .activity()
}

fn receive_partner_lines(net: &NetThread) {
    for message in net.take_relayed(|message| matches!(message, Message::Dialogue(_))) {
        let Message::Dialogue(update) = message else {
            continue;
        };
        PARTNER_TALK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .receive(update, net_clock_ms());
        crate::kaine::scene_stand_in_partner_talk(update);
        match update {
            DialogueUpdate::Line(line) => log_line(&format!(
                "dialogue: partner on talk {} line {}",
                line.talk_id, line.line
            )),
            DialogueUpdate::StoryLine(line) => log_line(&format!(
                "dialogue: partner on talk {} line {} (story step)",
                line.talk_id, line.line
            )),
            DialogueUpdate::Ended => log_line("dialogue: partner talk ended"),
        }
    }
}

fn share_own_talk(
    net: &NetThread,
    view: &NetShared,
    memory: &InProcessMemory,
    own_window_open: bool,
) -> Option<DialogueLine> {
    let cursor = read_talk_cursor(memory)
        .ok()
        .flatten()
        .filter(|_| own_window_open);
    let mut tracker = TRACKER.lock().unwrap_or_else(PoisonError::into_inner);
    let update = tracker.observe(cursor, view.partner_in_play(), |talk_address| {
        let talk_id = find_talk_id(memory, talk_address).ok().flatten();
        if talk_id.is_none() {
            log_line("dialogue: the open talk is not in the talk table, not shared");
        }
        talk_id
    });
    let current = tracker.current_line();
    drop(tracker);
    let mut origin = TALK_ORIGIN.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((call, at_ms)) = crate::game_hooks::last_pop() {
        origin.note_popped_call(&call, at_ms);
    }
    if let Some((talk_id, class)) = origin.observe(
        current.map(|line| line.talk_id),
        OWN_TALK_STARTED_BY_PLAYER_COMMAND.load(Ordering::Acquire),
        net_clock_ms(),
    ) {
        log_line(&format!(
            "dialogue: talk {talk_id} here is {}",
            class.describe()
        ));
    }
    if let Some(update) = update.map(|update| origin.class_update(update)) {
        match update {
            DialogueUpdate::Line(line) => log_line(&format!(
                "dialogue: sharing talk {} line {}",
                line.talk_id, line.line
            )),
            DialogueUpdate::StoryLine(line) => log_line(&format!(
                "dialogue: sharing talk {} line {} (story step)",
                line.talk_id, line.line
            )),
            DialogueUpdate::Ended => log_line("dialogue: sharing talk ended"),
        }
        net.send_relayed(Message::Dialogue(update));
    }
    current
}

fn apply(build: &VerifiedBuild, action: MirrorAction) {
    match action {
        MirrorAction::FollowStarted(talk_id) => {
            log_line(&format!(
                "dialogue: the host leads talk {talk_id}, your copy waits on the host's lines"
            ));
            return;
        }
        MirrorAction::FollowLine(line) => {
            // The guest's own copy of the talk: its line tags run here as the script's own.
            show_line(&mut InProcessMemory::new(build), line);
            log_line(&format!(
                "dialogue: your copy follows the host to line {line}"
            ));
            return;
        }
        MirrorAction::ChainOwnTalk(next) => {
            step_past_line(build);
            log_line(&format!(
                "dialogue: the host went on to talk {}, your copy steps on with it",
                next.talk_id
            ));
            return;
        }
        MirrorAction::FinishOwnTalk(talk_id) => {
            close_mirror(build);
            log_line(&format!(
                "dialogue: the host left talk {talk_id}, your copy is closed so your game moves on with the host"
            ));
            return;
        }
        MirrorAction::ChoiceGaveUp(talk_id) => {
            log_line(&format!(
                "choice: the host left talk {talk_id} with no answer for your prompt, your buttons are back"
            ));
            return;
        }
        MirrorAction::FollowGaveUp(talk_id) => {
            log_line(&format!(
                "dialogue: the host never showed talk {talk_id}, yours goes on by your own buttons"
            ));
            return;
        }
        MirrorAction::SkipEndedTalk(talk_id) => {
            log_line(&format!(
                "dialogue: not mirroring talk {talk_id}, you just finished it (the partner's is not shown back)"
            ));
            return;
        }
        _ => {}
    }
    let _skip_tags = MirrorActionGuard::begin();
    match action {
        MirrorAction::Open(line) => {
            begin_talk();
            let opened = open_mirror(build, line);
            log_line(&format!(
                "dialogue: mirror open talk {} at line {} ({})",
                line.talk_id,
                line.line,
                if opened {
                    "opened, the on-screen line follows"
                } else {
                    "the game refused"
                }
            ));
        }
        MirrorAction::ShowLine(line) => {
            show_line(&mut InProcessMemory::new(build), line);
            log_line(&format!("dialogue: mirror line {line}"));
        }
        MirrorAction::Close => {
            close_mirror(build);
            log_line("dialogue: mirror closed");
        }
        MirrorAction::JumpOwnTalk(line) => {
            show_line(&mut InProcessMemory::new(build), line);
            log_line(&format!(
                "dialogue: own talk joined the partner's at line {line}"
            ));
        }
        MirrorAction::FollowStarted(_)
        | MirrorAction::FollowLine(_)
        | MirrorAction::FinishOwnTalk(_)
        | MirrorAction::ChainOwnTalk(_)
        | MirrorAction::FollowGaveUp(_)
        | MirrorAction::ChoiceGaveUp(_)
        | MirrorAction::SkipEndedTalk(_) => {}
    }
}
