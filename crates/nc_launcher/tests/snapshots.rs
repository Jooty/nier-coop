use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Color32, ColorImage, UiBuilder, vec2};
use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use nc_launcher::app::{
    ConsoleRun, JoinDialog, Launcher, NetForms, Screen, Tab, apply_launcher_look, copy_line_id,
    draw, draw_parts_sheet, fixture_campaign_card, fixture_launcher, fixture_sending_the_save,
};
use nc_launcher::home::GameCard;
use nc_launcher::install::ModStatus;
use nc_launcher::net::{
    ConnectionReport, ConnectionState, LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LauncherNet,
    LinkStatus, LobbyReady, MemberStatus, NetError, NetState, NetStatus, SessionOptions,
};
use nc_launcher::saves::SlotSummary;
use nc_launcher::steam::GameInstall;
use nc_launcher_core::{
    ALL_PROFILES, AUTOMATA, ConsoleClock, ConsoleTask, FREE_ROAM_MODE, GameProfile,
    INVALID_TICKET_TEXT, LauncherEvent, PathKind, REPLICANT, banner_facts, judge_rejection_text,
};
use nc_net::{IrohConfig, IrohTransport, Transport};
use replicant_proto::{
    CampaignCard, GameReadiness, PartnerBody, RejectReason, SaveSummary, SecondSeatCard,
};

const WINDOW_SIZE_PX: (u32, u32) = (1180, 760);
const MINIMUM_WINDOW_SIZE_PX: (u32, u32) = (900, 600);
const NARROW_WINDOW_SIZE_PX: (u32, u32) = (900, 760);
const GENERATED_ART_SIZE_PX: (usize, usize) = (1920, 620);
const GENERATED_SUBJECT_SIZE_PX: (usize, usize) = (360, 420);
const REPLICANT_FIXTURE_SAVES: usize = 2;
const PARTS_SHEET_FRAMES: usize = 3;
const SNAPSHOT_FOLDER: &str = "launcher-snapshots";
const SCRATCH_FOLDER: &str = "test-scratch/launcher-snapshots";
const FIXTURE_TICKET: &str = "nc1ticketfixtureaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FIXTURE_CAMPAIGN_ID: &str = "c0ffee00c0ffee00";
const FIXTURE_READY_BUILD: &str = "7b04de2";
const FIXTURE_BUILD_SERVER: &str = "builds.example:7878";
const FIXTURE_PLAYER_NAME: &str = "Jay";
const FIXTURE_PARTNER_PING_MS: u32 = 38;
const AUTOMATA_NINE_S_INDEX: u8 = 1;
const TICKET_COPY_SALT: &str = "lobby-ticket";
const FREE_ROAM_PINGS_MS: [u32; 7] = [38, 64, 51, 112, 47, 29, 73];
const FREE_ROAM_IN_GAME_INDEX: usize = 3;
const THIRD_ROW_POINTER: egui::Pos2 = egui::pos2(520.0, 430.0);
const FIXTURE_LOG_LINES: [&str; 6] = [
    "launcher 0.1.0 started",
    "found the game folder",
    "mod installed, build 3f2a91c",
    "hosting a lobby",
    "Mira joined in slot 1",
    "the game started and linked to the launcher; the partner's game is loading the co-op save in slot 3, which can take a while on a cold disk",
];
const GUEST_NAMES: [&str; 15] = [
    "Mira", "Okabe", "Tess", "Rowan", "Sol", "Ivo", "Petra", "Quill", "Basil", "Juno", "Kit",
    "Lark", "Marlo", "Nell", "Otto",
];
const MOTION_FOLDER: &str = "motion";
const MOTION_FRAME_MS: f32 = 1000.0 / 60.0;
const MOTION_SETTLE_MAX_STEPS: u64 = 240;
const MOTION_SETTLED_STEPS: u64 = 1;
const COPY_HOLD_FRAMES_SECONDS: f32 = 1.6;
const DRIFT_FRAME_MS: f32 = 50.0;
const DRIFT_FRAMES: usize = 40;
const SEND_METER_FRAMES: usize = 60;
const SEND_METER_PARTS: u16 = 24;
const HOME_ROW_ART_POINT: egui::Pos2 = egui::pos2(900.0, 250.0);
const HOME_AWAY_POINT: egui::Pos2 = egui::pos2(150.0, 740.0);
const CONSOLE_MIDWAY_FRAMES: usize = 40;
const CONSOLE_DONE_FRAMES: usize = 50;
const CONSOLE_CLIP_FRAMES: usize = 80;
const CONSOLE_CLIP_WORK_DONE_FRAME: usize = 30;

struct FixtureNet {
    state: NetState,
}

impl LauncherNet for FixtureNet {
    fn host(&self) -> Result<String, NetError> {
        Err(NetError::Unavailable)
    }

    fn join(&self, _ticket: &str) -> Result<(), NetError> {
        Err(NetError::Unavailable)
    }

    fn test_connection(&self, _ticket: &str, _seconds: u32) -> Result<ConnectionReport, NetError> {
        Err(NetError::Unavailable)
    }

    fn leave(&self) {}

    fn status(&self) -> NetStatus {
        NetStatus {
            state: self.state.clone(),
        }
    }
}

struct Page {
    tab: Tab,
    launcher: Launcher,
    forms: NetForms,
    ticket_copied_once: bool,
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(SCRATCH_FOLDER)
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

fn found_card(profile: &'static GameProfile, root: &Path) -> GameCard {
    let game_dir = root.join(profile.install_folder_name);
    std::fs::create_dir_all(&game_dir).expect("game folder");
    std::fs::write(game_dir.join(profile.exe_names[0]), b"").expect("fake exe");
    let dll = game_dir.join(profile.hook_dll_name);
    std::fs::write(&dll, b"").expect("fake dll");
    GameCard {
        profile,
        install: Some(GameInstall {
            profile,
            game_dir,
            steamapps_dir: root.to_path_buf(),
            proton_prefix: None,
        }),
        dll_source: Some(dll),
        mod_status: ModStatus::Current,
        installed_build: Some("3f2a91c".to_owned()),
        ready_build: None,
        coop_saves: profile.support.saves.then_some(REPLICANT_FIXTURE_SAVES),
    }
}

fn both_games_found(root: &Path) -> Vec<GameCard> {
    ALL_PROFILES
        .iter()
        .map(|profile| found_card(profile, root))
        .collect()
}

fn generated_art(profile: &GameProfile) -> ColorImage {
    let (width, height) = GENERATED_ART_SIZE_PX;
    let focus = banner_facts(profile.game_id).focus;
    let subject_x = (focus.x * width as f32) as usize;
    let subject_y = (focus.y * height as f32) as usize;
    let (subject_width, subject_height) = GENERATED_SUBJECT_SIZE_PX;
    let in_subject = |x: usize, y: usize| {
        x + subject_width / 2 >= subject_x
            && x < subject_x + subject_width / 2
            && y + subject_height / 2 >= subject_y
            && y < subject_y + subject_height / 2
    };
    let pixels = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            if in_subject(x, y) {
                let stripe = ((x / 24 + y / 24) % 2) as u8;
                return Color32::from_rgb(200 + 30 * stripe, 190 + 30 * stripe, 170);
            }
            let across = x as f32 / width as f32;
            let down = y as f32 / height as f32;
            let block = ((x / 160 + y / 120) % 3) as f32 * 10.0;
            Color32::from_rgb(
                (60.0 + 90.0 * across + block) as u8,
                (70.0 + 60.0 * down + block) as u8,
                (110.0 + 80.0 * (1.0 - across) + block) as u8,
            )
        })
        .collect();
    ColorImage::new([width, height], pixels)
}

fn with_generated_art(mut launcher: Launcher) -> Launcher {
    for profile in ALL_PROFILES {
        launcher
            .banner_art
            .insert_image(profile.game_id, Some(generated_art(profile)));
    }
    launcher
}

fn without_art(mut launcher: Launcher) -> Launcher {
    for profile in ALL_PROFILES {
        launcher.banner_art.insert_image(profile.game_id, None);
    }
    launcher
}

fn lobby_link(guests: usize, as_guest: bool, max_players: u8) -> LinkStatus {
    let mut link = LinkStatus {
        state: ConnectionState::Connected,
        ticket: (!as_guest).then(|| FIXTURE_TICKET.to_owned()),
        game_linked: false,
        guest: as_guest,
        link_port: 7000,
        ..LinkStatus::default()
    };
    link.rules.max_players = max_players;
    link.own_slot = Some(if as_guest { guests as u8 } else { 0 });
    link.members = (0..guests)
        .map(|index| {
            MemberStatus::new(
                index as u8 + u8::from(!as_guest),
                GUEST_NAMES[index % GUEST_NAMES.len()].to_owned(),
            )
        })
        .collect();
    link
}

fn net_state_for(link: &LinkStatus, hosting_alone: bool) -> NetState {
    if hosting_alone {
        return NetState::Hosting {
            ticket: FIXTURE_TICKET.to_owned(),
        };
    }
    NetState::Connected {
        peer_name: link
            .members
            .first()
            .map_or_else(String::new, |member| member.name.clone()),
    }
}

fn game_page(launcher: Launcher, state: NetState) -> Page {
    let mut forms = NetForms::new(Arc::new(FixtureNet { state }));
    forms.session_options = SessionOptions::default();
    Page {
        tab: Tab::NewGame,
        launcher,
        forms,
        ticket_copied_once: false,
    }
}

fn render_page(name: &str, page: Page) {
    render_page_at(name, page, WINDOW_SIZE_PX);
}

fn render_page_at(name: &str, page: Page, size: (u32, u32)) {
    render_page_with_pointer(name, page, size, None);
}

fn render_page_hovered(name: &str, page: Page, pointer: egui::Pos2) {
    render_page_with_pointer(name, page, WINDOW_SIZE_PX, Some(pointer));
}

fn render_page_with_pointer(
    name: &str,
    mut page: Page,
    (width, height): (u32, u32),
    pointer: Option<egui::Pos2>,
) {
    page.launcher.new_game_name = FIXTURE_PLAYER_NAME.to_owned();
    let mut harness = Harness::builder()
        .with_size(vec2(width as f32, height as f32))
        .with_accessibility_check(false)
        .renderer(WgpuTestRenderer::new())
        .with_context_setup(apply_launcher_look)
        .build_ui_state(draw_page, page);
    harness.run();
    if let Some(pointer) = pointer {
        harness.hover_at(pointer);
        harness.run();
    }
    let image = harness.render().expect("off-screen render");
    assert_eq!((image.width(), image.height()), (width, height));
    save_snapshot(name, &image);
}

fn draw_page(ui: &mut egui::Ui, page: &mut Page) {
    // The harness wraps the page in an 8 px margin; draw over the whole window instead.
    let window = ui.ctx().content_rect();
    if page.ticket_copied_once {
        page.ticket_copied_once = false;
        let now = ui.input(|input| input.time);
        ui.data_mut(|data| data.insert_temp(copy_line_id(TICKET_COPY_SALT), now));
    }
    ui.scope_builder(UiBuilder::new().max_rect(window), |ui| {
        draw(ui, &mut page.tab, &mut page.launcher, &mut page.forms);
    });
}

fn save_snapshot(name: &str, image: &image::RgbaImage) {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(SNAPSHOT_FOLDER);
    std::fs::create_dir_all(&folder).expect("snapshot folder");
    image
        .save(folder.join(format!("{name}.png")))
        .expect("png written");
}

fn home_launcher(root: &Path) -> Launcher {
    home_launcher_for(&REPLICANT, root)
}

fn home_launcher_for(profile: &'static GameProfile, root: &Path) -> Launcher {
    let mut cards = both_games_found(root);
    for card in &mut cards {
        if card.profile.game_id == REPLICANT.game_id {
            card.mod_status = ModStatus::Outdated;
            card.ready_build = Some(FIXTURE_READY_BUILD.to_owned());
        }
    }
    fixture_launcher(profile, cards, LinkStatus::default(), Screen::Home)
}

#[test]
fn home_with_both_games_found() {
    let root = scratch_dir("home");
    let launcher = with_generated_art(home_launcher(&root));
    render_page("home-both-games", game_page(launcher, NetState::Idle));
}

#[test]
fn home_with_one_game_missing() {
    let root = scratch_dir("home-one-missing");
    let mut launcher = with_generated_art(home_launcher(&root));
    for card in &mut launcher.cards {
        if card.profile.game_id == AUTOMATA.game_id {
            card.install = None;
            card.mod_status = ModStatus::NotInstalled;
            card.installed_build = None;
            card.coop_saves = None;
        }
    }
    render_page("home-one-missing", game_page(launcher, NetState::Idle));
}

#[test]
fn home_without_art() {
    let root = scratch_dir("home-no-art");
    let launcher = without_art(home_launcher(&root));
    render_page("home-without-art", game_page(launcher, NetState::Idle));
}

#[test]
fn home_at_900_wide() {
    let root = scratch_dir("home-narrow");
    let launcher = with_generated_art(home_launcher(&root));
    render_page_at(
        "home-900-wide",
        game_page(launcher, NetState::Idle),
        NARROW_WINDOW_SIZE_PX,
    );
}

#[test]
fn home_at_the_minimum_window_size() {
    let root = scratch_dir("home-minimum");
    let launcher = with_generated_art(home_launcher(&root));
    render_page_at(
        "home-minimum-size",
        game_page(launcher, NetState::Idle),
        MINIMUM_WINDOW_SIZE_PX,
    );
}

fn ready_partner(link: &mut LinkStatus, ready: bool, character: Option<u8>) {
    for member in &mut link.members {
        member.lobby = Some(LobbyReady {
            ready,
            mode: LOBBY_MODE_NEW_GAME,
            slot: None,
        });
        member.rtt_ms = Some(FIXTURE_PARTNER_PING_MS);
        member.character = character;
    }
}

fn lobby_page(
    profile: &'static GameProfile,
    name: &str,
    link: LinkStatus,
    state: NetState,
) -> Page {
    let root = scratch_dir(name);
    let launcher = with_generated_art(fixture_launcher(
        profile,
        both_games_found(&root),
        link,
        Screen::Game,
    ));
    game_page(launcher, state)
}

fn hosting_alone_link() -> LinkStatus {
    LinkStatus {
        state: ConnectionState::Hosting,
        ticket: Some(FIXTURE_TICKET.to_owned()),
        own_slot: Some(0),
        ..LinkStatus::default()
    }
}

fn fixture_saves() -> Vec<SlotSummary> {
    vec![
        fixture_slot(0, "Tess", 8, 3.25),
        fixture_slot(1, "", 0, 0.0),
        fixture_slot(2, "Okabe", 31, 22.17),
        fixture_slot(3, "Rowan", 17, 11.5),
    ]
}

fn home_page(name: &str) -> Page {
    home_page_for(&REPLICANT, name)
}

fn home_page_for(profile: &'static GameProfile, name: &str) -> Page {
    let root = scratch_dir(name);
    game_page(
        with_generated_art(home_launcher_for(profile, &root)),
        NetState::Idle,
    )
}

#[test]
fn home_with_a_lobby_open() {
    let mut page = home_page("home-lobby-open");
    page.launcher.link = hosting_alone_link();
    set_net_state(
        &mut page,
        NetState::Hosting {
            ticket: FIXTURE_TICKET.to_owned(),
        },
    );
    page.forms.lobby_was_open = true;
    render_page_at("home-lobby-open", page, MINIMUM_WINDOW_SIZE_PX);
}

#[test]
fn home_lobby_failed() {
    let mut page = home_page("home-lobby-failed");
    set_net_state(
        &mut page,
        NetState::Failed("the host left the lobby".to_owned()),
    );
    page.forms.lobby_was_open = true;
    render_page("home-lobby-failed", page);
}

#[test]
fn the_second_games_home_after_a_failed_lobby() {
    let mut page = home_page_for(&AUTOMATA, "automata-home-lobby-failed");
    set_net_state(
        &mut page,
        NetState::Failed("the link to the host dropped".to_owned()),
    );
    page.forms.lobby_was_open = true;
    render_page("automata-home-lobby-failed", page);
}

fn join_dialog_page(name: &str, ticket: &str, refusal: Option<&'static str>) -> Page {
    let mut page = home_page(name);
    page.forms.join_ticket = ticket.to_owned();
    page.forms.join_dialog = Some(JoinDialog {
        refusal,
        focused_once: false,
    });
    page
}

#[test]
fn the_second_games_join_dialog() {
    let mut page = home_page_for(&AUTOMATA, "automata-join-dialog");
    page.forms.join_ticket = FIXTURE_TICKET.to_owned();
    page.forms.join_dialog = Some(JoinDialog::default());
    render_page("automata-join-dialog", page);
}

#[test]
fn the_join_dialog() {
    render_page(
        "join-dialog-empty",
        join_dialog_page("join-empty", "", None),
    );
    render_page(
        "join-dialog-ticket",
        join_dialog_page("join-ticket", FIXTURE_TICKET, None),
    );
    render_page(
        "join-dialog-refused",
        join_dialog_page("join-refused", FIXTURE_TICKET, Some(INVALID_TICKET_TEXT)),
    );
    render_page_at(
        "join-dialog-minimum-size",
        join_dialog_page("join-minimum", FIXTURE_TICKET, None),
        MINIMUM_WINDOW_SIZE_PX,
    );
}

fn readable_ticket() -> String {
    let mut transport = IrohTransport::new(IrohConfig {
        use_relays_and_lookup: false,
    })
    .expect("endpoint binds");
    transport.host().expect("invite").as_str().to_owned()
}

fn console_page(name: &str, task: ConsoleTask) -> Page {
    console_page_for(&REPLICANT, name, task)
}

fn console_page_for(profile: &'static GameProfile, name: &str, task: ConsoleTask) -> Page {
    let mut page = home_page_for(profile, name);
    if task == ConsoleTask::Joining {
        page.forms.join_ticket = readable_ticket();
    }
    page.forms.console = Some(ConsoleRun {
        task,
        clock: ConsoleClock::new(0.0),
    });
    page
}

fn host_answered(page: &mut Page) {
    page.forms.host_result = Some(Ok(FIXTURE_TICKET.to_owned()));
    page.launcher.link = hosting_alone_link();
    set_net_state(
        page,
        NetState::Hosting {
            ticket: FIXTURE_TICKET.to_owned(),
        },
    );
}

fn refused_report(reason: RejectReason) -> ConnectionReport {
    ConnectionReport {
        reachable: true,
        path: PathKind::Direct,
        rtt: None,
        loss_percent: 0.0,
        rejected: Some(reason),
        verdict: judge_rejection_text(reason).to_owned(),
    }
}

fn console_harness(mut page: Page) -> Harness<'static, Page> {
    // The console starts on the harness's own clock, after the harness has drawn its first frame.
    let console = page.forms.console.take();
    let mut harness = stepped_harness(page, MOTION_FRAME_MS);
    let now_ms = harness.ctx.input(|input| input.time) * 1000.0;
    harness.state_mut().forms.console = console.map(|run| ConsoleRun {
        task: run.task,
        clock: ConsoleClock::new(now_ms),
    });
    harness
}

fn render_after_frames(name: &str, page: Page, frames: usize) {
    let mut harness = console_harness(page);
    for _ in 0..frames {
        harness.step();
    }
    save_snapshot(name, &harness.render().expect("off-screen render"));
}

#[test]
fn the_console_while_hosting() {
    render_page(
        "console-hosting-midway",
        console_page("console-hosting-midway", ConsoleTask::Hosting),
    );
    let mut page = console_page("console-hosting-done", ConsoleTask::Hosting);
    host_answered(&mut page);
    render_after_frames("console-hosting-done", page, CONSOLE_DONE_FRAMES);
}

#[test]
fn the_second_games_console() {
    let page = console_page_for(&AUTOMATA, "automata-console-hosting", ConsoleTask::Hosting);
    render_after_frames(
        "automata-console-hosting-midway",
        page,
        CONSOLE_MIDWAY_FRAMES,
    );
    let mut page = console_page_for(&AUTOMATA, "automata-console-joining", ConsoleTask::Joining);
    page.forms.probe_result = Some(Ok(refused_report(RejectReason::VersionMismatch)));
    render_page("automata-console-joining-failed", page);
}

#[test]
fn the_console_while_joining() {
    let page = console_page("console-joining-midway", ConsoleTask::Joining);
    render_after_frames("console-joining-midway", page, CONSOLE_MIDWAY_FRAMES);
    let mut page = console_page("console-joining-failed", ConsoleTask::Joining);
    page.forms.probe_result = Some(Ok(refused_report(RejectReason::VersionMismatch)));
    render_page("console-joining-failed", page);
}

#[test]
fn the_console_with_motion_off() {
    let mut page = console_page("console-motion-off", ConsoleTask::Hosting);
    page.launcher.config.motion = false;
    let mut harness = stepped_harness(page, MOTION_FRAME_MS);
    harness.step();
    save_snapshot(
        "console-motion-off",
        &harness.render().expect("off-screen render"),
    );
}

#[test]
fn hosting_ends_in_the_lobby() {
    let mut page = console_page("console-to-lobby", ConsoleTask::Hosting);
    host_answered(&mut page);
    let mut harness = stepped_harness(page, MOTION_FRAME_MS);
    harness.run();
    let page = harness.state();
    assert!(page.forms.console.is_none());
    assert_eq!(page.launcher.screen, Screen::Game);
    assert_eq!(page.tab, Tab::NewGame);
}

#[test]
fn motion_of_the_boot_console() {
    let page = console_page("motion-console", ConsoleTask::Hosting);
    let mut harness = console_harness(page);
    let folder = motion_folder("boot-console");
    record_frames(
        &mut harness,
        &folder,
        0,
        CONSOLE_CLIP_FRAMES,
        |harness, index| {
            if index == CONSOLE_CLIP_WORK_DONE_FRAME {
                host_answered(harness.state_mut());
            }
        },
    );
    assert_settles(&mut harness);
}

#[test]
fn a_guest_joined_from_the_other_tab_follows_the_host() {
    for (tab, host_kind, name) in [
        (Tab::NewGame, LOBBY_MODE_CONTINUE, "from-new-game"),
        (Tab::Continue, LOBBY_MODE_NEW_GAME, "from-continue"),
    ] {
        let snapshot = format!("lobby-guest-{name}");
        let mut link = lobby_link(1, true, 2);
        ready_partner(&mut link, true, None);
        for member in &mut link.members {
            if let Some(lobby) = member.lobby.as_mut() {
                lobby.mode = host_kind;
            }
        }
        let state = net_state_for(&link, false);
        let mut page = lobby_page(&REPLICANT, &snapshot, link, state);
        page.tab = tab;
        page.launcher.lobby_mode = if tab == Tab::Continue {
            LOBBY_MODE_CONTINUE
        } else {
            LOBBY_MODE_NEW_GAME
        };
        page.forms.join_ticket = FIXTURE_TICKET.to_owned();
        page.launcher.follow_the_hosts_kind();
        assert_eq!(page.launcher.lobby_mode, host_kind);
        render_page(&snapshot, page);
    }
}

#[test]
fn automata_lobby_idle() {
    let page = lobby_page(
        &AUTOMATA,
        "automata-lobby-idle",
        LinkStatus::default(),
        NetState::Idle,
    );
    render_page("automata-lobby-idle", page);
}

#[test]
fn lobby_hosting_alone() {
    let state = NetState::Hosting {
        ticket: FIXTURE_TICKET.to_owned(),
    };
    let page = lobby_page(&AUTOMATA, "lobby-alone", hosting_alone_link(), state);
    render_page("lobby-hosting-alone", page);
}

#[test]
fn hosted_lobby_of_two() {
    let mut link = lobby_link(1, false, 2);
    ready_partner(&mut link, false, None);
    let state = net_state_for(&link, false);
    render_page(
        "lobby-hosted-two",
        lobby_page(&REPLICANT, "lobby-two", link, state),
    );
}

#[test]
fn automata_hosted_lobby_of_two() {
    let mut link = lobby_link(1, false, 2);
    ready_partner(&mut link, true, Some(AUTOMATA_NINE_S_INDEX));
    let state = net_state_for(&link, false);
    render_page(
        "automata-lobby-hosted-two",
        lobby_page(&AUTOMATA, "automata-lobby-two", link, state),
    );
}

#[test]
fn automata_lobby_with_the_ticket_copied() {
    let mut link = lobby_link(1, false, 2);
    ready_partner(&mut link, true, Some(AUTOMATA_NINE_S_INDEX));
    let state = net_state_for(&link, false);
    let mut page = lobby_page(&AUTOMATA, "automata-lobby-copied", link, state);
    page.ticket_copied_once = true;
    render_page("automata-lobby-ticket-copied", page);
}

#[test]
fn lobby_seen_by_the_guest() {
    let mut link = lobby_link(1, true, 2);
    ready_partner(&mut link, true, None);
    let state = net_state_for(&link, false);
    let mut page = lobby_page(&AUTOMATA, "lobby-guest", link, state);
    page.forms.join_ticket = FIXTURE_TICKET.to_owned();
    render_page("lobby-guest-view", page);
}

#[test]
fn replicant_guest_who_never_chose_a_body() {
    let mut link = lobby_link(1, true, 2);
    ready_partner(&mut link, false, None);
    let state = net_state_for(&link, false);
    let page = lobby_page(&REPLICANT, "replicant-guest-third-body", link, state);
    assert_eq!(page.launcher.config.body, None);
    render_page("replicant-guest-third-body", page);
}

fn free_roam_link(guests: usize, as_guest: bool) -> LinkStatus {
    let mut link = lobby_link(guests, as_guest, 16);
    link.rules.mode = FREE_ROAM_MODE;
    for (index, member) in link.members.iter_mut().enumerate() {
        member.lobby = Some(LobbyReady {
            ready: index % 3 != 1,
            mode: LOBBY_MODE_NEW_GAME,
            slot: None,
        });
        member.rtt_ms = Some(FREE_ROAM_PINGS_MS[index % FREE_ROAM_PINGS_MS.len()]);
        member.character = Some((index % 2) as u8);
    }
    if let Some(member) = link.members.get_mut(FREE_ROAM_IN_GAME_INDEX) {
        member.readiness = GameReadiness::InPlay;
    }
    link
}

fn free_roam_page(name: &str, guests: usize, as_guest: bool) -> Page {
    let link = free_roam_link(guests, as_guest);
    let state = net_state_for(&link, false);
    lobby_page(&AUTOMATA, name, link, state)
}

#[test]
fn free_roam_of_six_seen_by_the_host_with_a_row_hovered() {
    let page = free_roam_page("free-roam-six-host", 5, false);
    render_page_hovered("lobby-free-roam-six-host", page, THIRD_ROW_POINTER);
}

#[test]
fn free_roam_of_six_seen_by_a_guest_with_a_row_hovered() {
    let page = free_roam_page("free-roam-six-guest", 5, true);
    render_page_hovered("lobby-free-roam-six-guest", page, THIRD_ROW_POINTER);
}

#[test]
fn free_roam_of_sixteen_seen_by_the_host() {
    let page = free_roam_page("free-roam-sixteen-host", 15, false);
    render_page("lobby-free-roam-sixteen-host", page);
}

#[test]
fn free_roam_of_sixteen_seen_by_a_guest() {
    let page = free_roam_page("free-roam-sixteen-guest", 15, true);
    render_page("lobby-free-roam-sixteen-guest", page);
}

#[test]
fn a_party_too_big_for_the_campaign() {
    let mut link = lobby_link(2, false, 2);
    ready_partner(&mut link, true, Some(AUTOMATA_NINE_S_INDEX));
    let state = net_state_for(&link, false);
    render_page(
        "lobby-party-does-not-fit",
        lobby_page(&AUTOMATA, "lobby-misfit", link, state),
    );
}

#[test]
fn automata_lobby_at_the_minimum_window_size() {
    let link = lobby_link(1, false, 2);
    let state = net_state_for(&link, false);
    render_page_at(
        "automata-lobby-minimum-size",
        lobby_page(&AUTOMATA, "automata-lobby-minimum", link, state),
        MINIMUM_WINDOW_SIZE_PX,
    );
}

#[test]
fn shared_parts_sheet() {
    let (width, height) = WINDOW_SIZE_PX;
    let mut harness = Harness::builder()
        .with_size(vec2(width as f32, height as f32))
        .with_accessibility_check(false)
        .renderer(WgpuTestRenderer::new())
        .with_context_setup(apply_launcher_look)
        .build_ui(|ui| {
            let window = ui.ctx().content_rect();
            ui.scope_builder(UiBuilder::new().max_rect(window), draw_parts_sheet);
        });
    harness.run_steps(PARTS_SHEET_FRAMES);
    save_snapshot("parts-sheet", &harness.render().expect("off-screen render"));
}

fn fixture_slot(index: u8, name: &str, level: i32, hours: f64) -> SlotSummary {
    SlotSummary {
        index,
        used: !name.is_empty(),
        name: name.to_owned(),
        phase: "FIXTURE_PHASE".to_owned(),
        level,
        play_seconds: hours * 3600.0,
    }
}

fn settings_page(profile: &'static GameProfile, name: &str) -> Page {
    let root = scratch_dir(name);
    let mut cards = both_games_found(&root);
    for card in &mut cards {
        card.ready_build = Some(FIXTURE_READY_BUILD.to_owned());
    }
    let mut launcher = with_generated_art(fixture_launcher(
        profile,
        cards,
        LinkStatus::default(),
        Screen::Game,
    ));
    launcher.normal_slots = vec![
        fixture_slot(0, "", 0, 0.0),
        fixture_slot(1, "Mira", 12, 7.5),
        fixture_slot(2, "Okabe", 31, 22.17),
    ];
    launcher.copy_from_slot = Some(2);
    launcher.config.update_url = Some(FIXTURE_BUILD_SERVER.to_owned());
    launcher.backups = vec![
        "20261112-134000".to_owned(),
        "20261111-220500".to_owned(),
        "20261109-191200".to_owned(),
    ];
    launcher.selected_backup = launcher.backups.first().cloned();
    let mut page = game_page(launcher, NetState::Idle);
    page.tab = Tab::Settings;
    page
}

#[test]
fn settings_for_the_first_game() {
    render_page(
        "settings-replicant",
        settings_page(&REPLICANT, "settings-replicant"),
    );
}

#[test]
fn settings_for_the_second_game() {
    render_page(
        "settings-automata",
        settings_page(&AUTOMATA, "settings-automata"),
    );
}

#[test]
fn logs_of_the_first_game() {
    let root = scratch_dir("logs-replicant");
    let mut launcher = fixture_launcher(
        &REPLICANT,
        both_games_found(&root),
        LinkStatus::default(),
        Screen::Game,
    );
    launcher.log = FIXTURE_LOG_LINES
        .iter()
        .map(|line| (*line).to_owned())
        .collect();
    launcher.export_note = Some("Exported to ~/fixture/logs".to_owned());
    let mut page = game_page(with_generated_art(launcher), NetState::Idle);
    page.tab = Tab::Logs;
    render_page("logs-replicant", page);
}

fn continue_page(profile: &'static GameProfile, name: &str, as_guest: bool) -> Page {
    let mut link = lobby_link(1, as_guest, 2);
    ready_partner(&mut link, true, None);
    for member in &mut link.members {
        if let Some(lobby) = member.lobby.as_mut() {
            lobby.mode = LOBBY_MODE_CONTINUE;
            lobby.slot = Some(1);
        }
    }
    let state = net_state_for(&link, false);
    let mut page = lobby_page(profile, name, link, state);
    page.launcher.lobby_mode = LOBBY_MODE_CONTINUE;
    page.tab = Tab::Continue;
    if as_guest {
        page.forms.join_ticket = FIXTURE_TICKET.to_owned();
        return page;
    }
    page.launcher.slots = fixture_saves();
    page.launcher.selected_slot = Some(2);
    page
}

#[test]
fn continue_for_the_first_game_with_a_save_picked() {
    render_page(
        "continue-replicant",
        continue_page(&REPLICANT, "continue-replicant", false),
    );
}

#[test]
fn continue_for_the_first_game_as_the_guest_with_the_hosts_campaign() {
    let mut page = continue_page(&REPLICANT, "continue-replicant-guest", true);
    fixture_campaign_card(
        &mut page.launcher,
        CampaignCard {
            campaign_id: FIXTURE_CAMPAIGN_ID.to_owned(),
            host: SaveSummary {
                name: "Okabe".to_owned(),
                level: 31,
                play_seconds: 22.17 * 3600.0,
            },
            second_seat: Some(SecondSeatCard {
                summary: SaveSummary {
                    name: "Mira".to_owned(),
                    level: 28,
                    play_seconds: 19.5 * 3600.0,
                },
                body: Some(PartnerBody::PrologueCharacter),
            }),
        },
    );
    render_page("continue-replicant-guest", page);
}

#[test]
fn continue_for_the_first_game_as_the_guest_before_the_card_arrives() {
    render_page(
        "continue-replicant-guest-waiting",
        continue_page(&REPLICANT, "continue-replicant-guest-waiting", true),
    );
}

#[test]
fn continue_for_a_game_without_saves() {
    render_page(
        "continue-automata",
        continue_page(&AUTOMATA, "continue-automata", false),
    );
}

#[test]
fn continue_for_a_game_without_saves_as_the_guest() {
    render_page(
        "continue-automata-guest",
        continue_page(&AUTOMATA, "continue-automata-guest", true),
    );
}

#[test]
fn a_campaign_seat_whose_player_is_in_game() {
    let mut link = lobby_link(1, false, 2);
    ready_partner(&mut link, true, Some(AUTOMATA_NINE_S_INDEX));
    for member in &mut link.members {
        member.readiness = GameReadiness::InPlay;
    }
    let state = net_state_for(&link, false);
    render_page(
        "lobby-campaign-partner-in-game",
        lobby_page(&AUTOMATA, "lobby-in-game", link, state),
    );
}

#[test]
fn a_guest_board_at_its_real_seat_after_a_player_left() {
    let mut link = free_roam_link(4, true);
    let slots = [0, 1, 3, 4];
    for (member, slot) in link.members.iter_mut().zip(slots) {
        member.slot = slot;
    }
    link.own_slot = Some(6);
    let state = net_state_for(&link, false);
    render_page(
        "free-roam-guest-real-seat",
        lobby_page(&AUTOMATA, "free-roam-guest-seat", link, state),
    );
}

fn motion_harness(page: Page, frame_ms: f32) -> Harness<'static, Page> {
    let mut harness = stepped_harness(page, frame_ms);
    harness.run();
    harness
}

fn stepped_harness(mut page: Page, frame_ms: f32) -> Harness<'static, Page> {
    page.launcher.new_game_name = FIXTURE_PLAYER_NAME.to_owned();
    let (width, height) = WINDOW_SIZE_PX;
    Harness::builder()
        .with_size(vec2(width as f32, height as f32))
        .with_accessibility_check(false)
        .renderer(WgpuTestRenderer::new())
        .with_context_setup(apply_launcher_look)
        .with_step_dt(frame_ms / 1000.0)
        .with_max_steps(MOTION_SETTLE_MAX_STEPS)
        .build_ui_state(draw_page, page)
}

fn motion_folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(SNAPSHOT_FOLDER)
        .join(MOTION_FOLDER)
        .join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("motion folder");
    folder
}

fn record_frames(
    harness: &mut Harness<'static, Page>,
    folder: &Path,
    first: usize,
    frames: usize,
    mut before_frame: impl FnMut(&mut Harness<'static, Page>, usize),
) -> usize {
    for index in 0..frames {
        before_frame(harness, index);
        harness.step();
        let image = harness.render().expect("off-screen render");
        image
            .save(folder.join(format!("{:03}.png", first + index)))
            .expect("png written");
    }
    first + frames
}

fn assert_settles(harness: &mut Harness<'static, Page>) {
    // The first run lets egui finish its own layout passes; after it an idle page asks for no frame.
    harness.run();
    let steps = harness.run();
    assert_eq!(
        steps, MOTION_SETTLED_STEPS,
        "the page kept asking for frames once its motion ended"
    );
}

fn record_motion(
    name: &str,
    page: Page,
    frames: usize,
    change: impl FnOnce(&mut Page),
) -> Harness<'static, Page> {
    let mut harness = motion_harness(page, MOTION_FRAME_MS);
    let folder = motion_folder(name);
    change(harness.state_mut());
    record_frames(&mut harness, &folder, 0, frames, |_, _| {});
    assert_settles(&mut harness);
    harness
}

fn set_net_state(page: &mut Page, state: NetState) {
    page.forms.net = Arc::new(FixtureNet { state });
}

fn hosting_alone_page(name: &str) -> Page {
    let state = NetState::Hosting {
        ticket: FIXTURE_TICKET.to_owned(),
    };
    lobby_page(&REPLICANT, name, hosting_alone_link(), state)
}

#[test]
fn motion_of_the_selection_sweep() {
    let page = hosting_alone_page("motion-sweep");
    record_motion("selection-sweep", page, 18, |page| {
        page.tab = Tab::Continue;
    });
}

#[test]
fn motion_of_a_tab_change() {
    let page = hosting_alone_page("motion-tab");
    record_motion("tab-change", page, 30, |page| page.tab = Tab::Settings);
}

#[test]
fn motion_of_the_copy_line() {
    let mut link = lobby_link(1, false, 2);
    ready_partner(&mut link, true, None);
    let state = net_state_for(&link, false);
    let page = lobby_page(&REPLICANT, "motion-copy", link, state);
    let mut harness = motion_harness(page, MOTION_FRAME_MS);
    let folder = motion_folder("copy-line");
    harness.state_mut().ticket_copied_once = true;
    let hold_frames = (COPY_HOLD_FRAMES_SECONDS * 1000.0 / MOTION_FRAME_MS) as usize;
    record_frames(&mut harness, &folder, 0, hold_frames, |_, _| {});
    assert_settles(&mut harness);
}

#[test]
fn motion_of_entering_and_leaving_a_lobby() {
    let page = home_page("motion-lobby");
    let mut harness = record_motion("lobby-enter", page, 45, |page| {
        host_answered(page);
        page.launcher.screen = Screen::Game;
        page.tab = Tab::NewGame;
    });
    let folder = motion_folder("lobby-leave");
    let page = harness.state_mut();
    set_net_state(page, NetState::Idle);
    page.launcher.link = LinkStatus::default();
    record_frames(&mut harness, &folder, 0, 25, |_, _| {});
    assert_settles(&mut harness);
    assert_eq!(harness.state().launcher.screen, Screen::Home);
}

#[test]
fn motion_of_a_seat_filling_and_a_status_word() {
    let state = NetState::Hosting {
        ticket: FIXTURE_TICKET.to_owned(),
    };
    let page = lobby_page(&REPLICANT, "motion-seat", hosting_alone_link(), state);
    let mut harness = record_motion("seat-fill", page, 40, |page| {
        let mut link = lobby_link(1, false, 2);
        ready_partner(&mut link, false, None);
        set_net_state(page, net_state_for(&link, false));
        page.launcher.link = link;
    });
    let folder = motion_folder("status-word");
    ready_partner(&mut harness.state_mut().launcher.link, true, None);
    record_frames(&mut harness, &folder, 0, 16, |_, _| {});
    assert_settles(&mut harness);
}

#[test]
fn motion_of_the_open_seat_while_hosting() {
    let state = NetState::Hosting {
        ticket: FIXTURE_TICKET.to_owned(),
    };
    let page = lobby_page(&REPLICANT, "motion-drift", hosting_alone_link(), state);
    let mut harness = motion_harness(page, DRIFT_FRAME_MS);
    let folder = motion_folder("hatch-drift");
    record_frames(&mut harness, &folder, 0, DRIFT_FRAMES, |_, _| {});
    assert_settles(&mut harness);
}

#[test]
fn motion_of_the_save_being_sent() {
    let mut link = lobby_link(1, false, 2);
    ready_partner(&mut link, true, None);
    let state = net_state_for(&link, false);
    let mut page = lobby_page(&REPLICANT, "motion-meter", link, state);
    fixture_sending_the_save(&mut page.launcher, LOBBY_MODE_NEW_GAME);
    let mut harness = motion_harness(page, MOTION_FRAME_MS);
    let folder = motion_folder("send-meter");
    record_frames(
        &mut harness,
        &folder,
        0,
        SEND_METER_FRAMES,
        |harness, index| {
            let done = (index * usize::from(SEND_METER_PARTS) / (SEND_METER_FRAMES - 10))
                .min(usize::from(SEND_METER_PARTS)) as u16;
            let page = harness.state_mut();
            let net = page.forms.net.clone();
            page.launcher.note_events(
                &[LauncherEvent::SeatProgress {
                    slot: 1,
                    transfer: 1,
                    parts_done: done,
                    parts: SEND_METER_PARTS,
                    incoming: false,
                }],
                &*net,
            );
        },
    );
    assert_settles(&mut harness);
}

#[test]
fn motion_of_the_home_rows() {
    let root = scratch_dir("motion-home");
    let page = game_page(home_launcher(&root), NetState::Idle);
    let mut harness = motion_harness(page, MOTION_FRAME_MS);
    let folder = motion_folder("home-art-load");
    harness.state_mut().launcher = with_generated_art(home_launcher(&root));
    record_frames(&mut harness, &folder, 0, 20, |_, _| {});
    assert_settles(&mut harness);
    let folder = motion_folder("home-hover");
    let next = record_frames(&mut harness, &folder, 0, 16, |harness, index| {
        if index == 2 {
            harness.hover_at(HOME_ROW_ART_POINT);
        }
    });
    record_frames(&mut harness, &folder, next, 16, |harness, index| {
        if index == 2 {
            harness.hover_at(HOME_AWAY_POINT);
        }
    });
    assert_settles(&mut harness);
}

#[test]
fn no_motion_draws_the_end_state_in_the_first_frame() {
    let mut page = hosting_alone_page("motion-off");
    page.launcher.config.motion = false;
    let mut harness = motion_harness(page, MOTION_FRAME_MS);
    harness.state_mut().tab = Tab::Settings;
    harness.step();
    let first = harness.render().expect("off-screen render");
    assert_settles(&mut harness);
    let settled = harness.render().expect("off-screen render");
    assert!(
        first.as_raw() == settled.as_raw(),
        "a frame moved with motion off"
    );
}
