use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Color32, ColorImage, UiBuilder, vec2};
use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use nc_launcher::app::{
    Launcher, NetForms, Screen, Tab, apply_launcher_look, copy_line_id, draw, draw_parts_sheet,
    fixture_launcher,
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
    ALL_PROFILES, AUTOMATA, FREE_ROAM_MODE, GameProfile, REPLICANT, banner_facts,
};
use replicant_proto::GameReadiness;

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
        .build_ui_state(
            |ui: &mut egui::Ui, page: &mut Page| {
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
            },
            page,
        );
    harness.run();
    if let Some(pointer) = pointer {
        harness.hover_at(pointer);
        harness.run();
    }
    let image = harness.render().expect("off-screen render");
    assert_eq!((image.width(), image.height()), (width, height));
    save_snapshot(name, &image);
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
    let mut cards = both_games_found(root);
    for card in &mut cards {
        if card.profile.game_id == REPLICANT.game_id {
            card.mod_status = ModStatus::Outdated;
            card.ready_build = Some(FIXTURE_READY_BUILD.to_owned());
        }
    }
    fixture_launcher(&REPLICANT, cards, LinkStatus::default(), Screen::Home)
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

#[test]
fn lobby_idle() {
    let page = lobby_page(
        &REPLICANT,
        "lobby-idle",
        LinkStatus::default(),
        NetState::Idle,
    );
    render_page("lobby-idle", page);
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
        fixture_slot(1, "Nier", 12, 7.5),
        fixture_slot(2, "Nier", 31, 22.17),
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

fn continue_page(profile: &'static GameProfile, name: &str) -> Page {
    let mut link = lobby_link(1, false, 2);
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
    page.launcher.slots = vec![
        fixture_slot(0, "Nier", 8, 3.25),
        fixture_slot(1, "", 0, 0.0),
        fixture_slot(2, "Nier", 31, 22.17),
        fixture_slot(3, "Nier", 17, 11.5),
    ];
    page.launcher.selected_slot = Some(2);
    page.tab = Tab::Continue;
    page
}

#[test]
fn continue_for_the_first_game_with_a_save_picked() {
    render_page(
        "continue-replicant",
        continue_page(&REPLICANT, "continue-replicant"),
    );
}

#[test]
fn continue_for_a_game_without_saves() {
    render_page(
        "continue-automata",
        continue_page(&AUTOMATA, "continue-automata"),
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
