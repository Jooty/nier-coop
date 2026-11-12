use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Color32, ColorImage, UiBuilder, vec2};
use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use nc_launcher::app::{
    Launcher, NetForms, Screen, Tab, apply_launcher_look, draw, draw_parts_sheet, fixture_launcher,
};
use nc_launcher::home::GameCard;
use nc_launcher::install::ModStatus;
use nc_launcher::net::{
    ConnectionReport, ConnectionState, LauncherNet, LinkStatus, MemberStatus, NetError, NetState,
    NetStatus, SessionOptions,
};
use nc_launcher::steam::GameInstall;
use nc_launcher_core::{ALL_PROFILES, AUTOMATA, GameProfile, REPLICANT, banner_facts};

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
const FIXTURE_PLAYER_NAME: &str = "Jay";
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
    link.members = (0..guests)
        .map(|index| {
            MemberStatus::new(
                index as u8 + 1,
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
    }
}

fn render_page(name: &str, page: Page) {
    render_page_at(name, page, WINDOW_SIZE_PX);
}

fn render_page_at(name: &str, mut page: Page, (width, height): (u32, u32)) {
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
                ui.scope_builder(UiBuilder::new().max_rect(window), |ui| {
                    draw(ui, &mut page.tab, &mut page.launcher, &mut page.forms);
                });
            },
            page,
        );
    harness.run();
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

#[test]
fn lobby_idle() {
    let root = scratch_dir("lobby-idle");
    let launcher = fixture_launcher(
        &REPLICANT,
        both_games_found(&root),
        LinkStatus::default(),
        Screen::Game,
    );
    render_page("lobby-idle", game_page(launcher, NetState::Idle));
}

#[test]
fn hosted_lobby_of_two() {
    let root = scratch_dir("lobby-two");
    let link = lobby_link(1, false, 2);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(&REPLICANT, both_games_found(&root), link, Screen::Game);
    render_page("lobby-hosted-two", game_page(launcher, state));
}

#[test]
fn hosted_lobby_of_six_with_a_limit_of_eight() {
    let root = scratch_dir("lobby-six");
    let link = lobby_link(5, false, 8);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(&REPLICANT, both_games_found(&root), link, Screen::Game);
    render_page("lobby-hosted-six-of-eight", game_page(launcher, state));
}

#[test]
fn lobby_of_sixteen_seen_by_a_guest() {
    let root = scratch_dir("lobby-sixteen");
    let link = lobby_link(15, true, 16);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(&REPLICANT, both_games_found(&root), link, Screen::Game);
    render_page("lobby-guest-sixteen", game_page(launcher, state));
}

#[test]
fn automata_hosted_lobby_of_two() {
    let root = scratch_dir("automata-lobby-two");
    let link = lobby_link(1, false, 2);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(&AUTOMATA, both_games_found(&root), link, Screen::Game);
    render_page("automata-lobby-hosted-two", game_page(launcher, state));
}

#[test]
fn automata_lobby_at_the_minimum_window_size() {
    let root = scratch_dir("automata-lobby-minimum");
    let link = lobby_link(1, false, 2);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(&AUTOMATA, both_games_found(&root), link, Screen::Game);
    render_page_at(
        "automata-lobby-minimum-size",
        game_page(launcher, state),
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
