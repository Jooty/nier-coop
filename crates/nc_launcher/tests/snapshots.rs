use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Theme, ThemePreference, vec2};
use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use nc_launcher::app::{
    Launcher, NetForms, Screen, Tab, apply_shared_spacing, draw, fixture_launcher,
    imgui_dark_visuals, install_heading_font,
};
use nc_launcher::home::GameCard;
use nc_launcher::install::ModStatus;
use nc_launcher::net::{
    ConnectionReport, ConnectionState, LauncherNet, LinkStatus, MemberStatus, NetError, NetState,
    NetStatus, SessionOptions,
};
use nc_launcher::steam::GameInstall;
use nc_launcher_core::{ALL_PROFILES, GameProfile};

const SNAPSHOT_WIDTH: f32 = 1180.0;
const SNAPSHOT_HEIGHT: f32 = 760.0;
const SNAPSHOT_FOLDER: &str = "launcher-snapshots";
const SCRATCH_FOLDER: &str = "test-scratch/launcher-snapshots";
const FIXTURE_TICKET: &str = "nc1ticketfixtureaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const GUEST_NAMES: [&str; 15] = [
    "Emil", "Kaine", "Devola", "Popola", "Fyra", "Yonah", "Weiss", "Grimoire", "Hook", "Gideon",
    "Halua", "Facade", "Cal", "Dito", "Nier2",
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
        installed_build: Some("e8e6df92".to_owned()),
    }
}

fn both_games_found(root: &Path) -> Vec<GameCard> {
    ALL_PROFILES
        .iter()
        .map(|profile| found_card(profile, root))
        .collect()
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

fn render_page(name: &str, mut page: Page) {
    page.launcher.new_game_name = "Nier".to_owned();
    let mut harness = Harness::builder()
        .with_size(vec2(SNAPSHOT_WIDTH, SNAPSHOT_HEIGHT))
        .with_accessibility_check(false)
        .renderer(WgpuTestRenderer::new())
        .with_context_setup(|ctx| {
            install_heading_font(ctx);
            ctx.set_theme(ThemePreference::Dark);
            ctx.set_visuals_of(Theme::Dark, imgui_dark_visuals());
            ctx.all_styles_mut(apply_shared_spacing);
        })
        .build_ui_state(
            |ui: &mut egui::Ui, page: &mut Page| {
                draw(ui, &mut page.tab, &mut page.launcher, &mut page.forms);
            },
            page,
        );
    harness.run();
    let image = harness.render().expect("off-screen render");
    assert_eq!((image.width(), image.height()), (1180, 760));
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(SNAPSHOT_FOLDER);
    std::fs::create_dir_all(&folder).expect("snapshot folder");
    image
        .save(folder.join(format!("{name}.png")))
        .expect("png written");
}

#[test]
fn home_with_both_games_found() {
    let root = scratch_dir("home");
    let launcher = fixture_launcher(both_games_found(&root), LinkStatus::default(), Screen::Home);
    render_page("home-both-games", game_page(launcher, NetState::Idle));
}

#[test]
fn lobby_idle() {
    let root = scratch_dir("lobby-idle");
    let launcher = fixture_launcher(both_games_found(&root), LinkStatus::default(), Screen::Game);
    render_page("lobby-idle", game_page(launcher, NetState::Idle));
}

#[test]
fn hosted_lobby_of_two() {
    let root = scratch_dir("lobby-two");
    let link = lobby_link(1, false, 2);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(both_games_found(&root), link, Screen::Game);
    render_page("lobby-hosted-two", game_page(launcher, state));
}

#[test]
fn hosted_lobby_of_six_with_a_limit_of_eight() {
    let root = scratch_dir("lobby-six");
    let link = lobby_link(5, false, 8);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(both_games_found(&root), link, Screen::Game);
    render_page("lobby-hosted-six-of-eight", game_page(launcher, state));
}

#[test]
fn lobby_of_sixteen_seen_by_a_guest() {
    let root = scratch_dir("lobby-sixteen");
    let link = lobby_link(15, true, 16);
    let state = net_state_for(&link, false);
    let launcher = fixture_launcher(both_games_found(&root), link, Screen::Game);
    render_page("lobby-guest-sixteen", game_page(launcher, state));
}
