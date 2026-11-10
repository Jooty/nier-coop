use nc_launcher_core::{
    BUNDLE_TITLE_LINE, BundleFile, BundleHeader, LogFileEntry, build_bundle, gunzip_text,
    gzip_text, is_allowed_upload_name, is_valid_install_id, log_upload_offered, new_install_id,
    origin_of, parse_sent_marker, pick_since, pick_unsent, render_sent_marker, role_of_files,
    upload_file_name, upload_url,
};

fn header() -> BundleHeader {
    BundleHeader {
        install_id: "0123456789abcdef0123456789abcdef".to_owned(),
        player_name: "Ayla\nInjected: line".to_owned(),
        game: "replicant".to_owned(),
        role: "host".to_owned(),
        build_commit: "abc1234".to_owned(),
        os: "linux".to_owned(),
        launcher_started: "2026-11-07 10:00:00 UTC".to_owned(),
    }
}

fn entry(name: &str, modified_secs: u64) -> LogFileEntry {
    LogFileEntry {
        name: name.to_owned(),
        modified_secs,
    }
}

#[test]
fn bundle_starts_with_a_header_then_each_file_under_its_banner() {
    let files = [
        BundleFile {
            name: "game-host.log".to_owned(),
            bytes: b"one\ntwo\n".to_vec(),
        },
        BundleFile {
            name: "launcher-host.log".to_owned(),
            bytes: b"no newline at the end".to_vec(),
        },
    ];
    let text = build_bundle(&header(), &files, 1024);
    let expected = format!(
        "{BUNDLE_TITLE_LINE}\ninstall id: 0123456789abcdef0123456789abcdef\nplayer name: Ayla Injected: line\ngame: replicant\nrole: host\nbuild commit: abc1234\nos: linux\nlauncher started: 2026-11-07 10:00:00 UTC\n\
         \n===== game-host.log (8 bytes) =====\none\ntwo\n\
         \n===== launcher-host.log (21 bytes) =====\nno newline at the end\n"
    );
    assert_eq!(text, expected);
}

#[test]
fn a_file_over_the_cap_keeps_only_its_last_bytes_and_its_real_size_in_the_banner() {
    let files = [BundleFile {
        name: "game-host.log".to_owned(),
        bytes: b"ABCDEFGHIJ\n".to_vec(),
    }];
    let text = build_bundle(&header(), &files, 4);
    assert!(text.contains("===== game-host.log (11 bytes) =====\nHIJ\n"));
    assert!(!text.contains("ABCD"));
}

#[test]
fn gzip_round_trips_the_bundle() {
    let text = build_bundle(&header(), &[], 16);
    assert_eq!(gunzip_text(&gzip_text(&text).unwrap()).unwrap(), text);
}

#[test]
fn upload_names_use_only_safe_characters() {
    assert_eq!(
        upload_file_name("20261107T100000Z", "host").as_deref(),
        Some("20261107T100000Z-host.log.gz")
    );
    assert_eq!(upload_file_name("2026/11", "host"), None);
    assert_eq!(upload_file_name("2026", "../etc"), None);
    assert_eq!(upload_file_name("2026", "ho st"), None);
    assert!(!is_allowed_upload_name("x.log"));
    assert!(!is_allowed_upload_name(".log.gz"));
    assert!(!is_allowed_upload_name(&format!(
        "{}.log.gz",
        "a".repeat(49)
    )));
}

#[test]
fn install_ids_are_32_hex_characters_and_differ() {
    let first = new_install_id();
    assert!(is_valid_install_id(&first));
    assert_ne!(first, new_install_id());
    assert!(!is_valid_install_id("short"));
    assert!(!is_valid_install_id(&"g".repeat(32)));
}

#[test]
fn the_upload_url_is_the_origin_of_the_update_url_plus_the_logs_path() {
    let id = "0123456789abcdef0123456789abcdef";
    assert_eq!(
        origin_of("https://host.example:8443/test/manifest.json"),
        Some("https://host.example:8443")
    );
    assert_eq!(
        origin_of("http://127.0.0.1:9000"),
        Some("http://127.0.0.1:9000")
    );
    assert_eq!(origin_of("ftp://host/x"), None);
    assert_eq!(origin_of("https:///x"), None);
    assert_eq!(
        upload_url(
            "https://host.example/test/manifest.json",
            id,
            "a-host.log.gz"
        )
        .as_deref(),
        Some("https://host.example/logs/0123456789abcdef0123456789abcdef/a-host.log.gz")
    );
    assert_eq!(
        upload_url("https://host.example/m.json", "bad id", "a-host.log.gz"),
        None
    );
    assert_eq!(upload_url("https://host.example/m.json", id, "../x"), None);
}

#[test]
fn files_are_picked_by_modified_time_and_name() {
    let entries = [
        entry("game-host.log", 100),
        entry("game-host.log.1", 50),
        entry("launcher-host.log", 120),
        entry("second-copy.log", 110),
        entry("2026-11-07_10-00-00.txt", 130),
        entry("logs-sent.txt", 140),
    ];
    assert_eq!(
        pick_since(&entries, 100),
        ["game-host.log", "launcher-host.log", "second-copy.log"]
    );
    assert!(pick_since(&entries, 200).is_empty());
}

#[test]
fn an_unsent_run_is_found_only_between_the_marker_and_the_launcher_start() {
    let entries = [
        entry("game-guest.log", 100),
        entry("launcher-guest.log", 105),
        entry("launcher.log", 300),
    ];
    assert_eq!(
        pick_unsent(&entries, None, 200),
        ["game-guest.log", "launcher-guest.log"]
    );
    assert_eq!(pick_unsent(&entries, Some(100), 200), Vec::<String>::new());
    assert_eq!(pick_unsent(&entries, Some(99), 200).len(), 2);
}

#[test]
fn launcher_logs_alone_are_not_an_unsent_run() {
    let entries = [entry("launcher-host.log", 100)];
    assert!(pick_unsent(&entries, None, 200).is_empty());
}

#[test]
fn the_role_comes_from_the_game_log_name() {
    let names = |list: &[&str]| {
        list.iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        role_of_files(&names(&["launcher-host.log", "game-guest.log.1"])),
        "guest"
    );
    assert_eq!(role_of_files(&names(&["game-host.log"])), "host");
    assert_eq!(role_of_files(&names(&["launcher.log"])), "launcher");
}

#[test]
fn the_sent_marker_round_trips_and_ignores_junk() {
    assert_eq!(
        parse_sent_marker(&render_sent_marker(1_762_509_600)),
        Some(1_762_509_600)
    );
    assert_eq!(parse_sent_marker("garbage"), None);
    assert_eq!(parse_sent_marker(""), None);
}

#[test]
fn the_switch_is_offered_only_with_both_the_server_and_the_token() {
    assert!(log_upload_offered(Some("https://h/m.json"), Some("t")));
    assert!(!log_upload_offered(None, Some("t")));
    assert!(!log_upload_offered(Some("https://h/m.json"), None));
    assert!(!log_upload_offered(Some("  "), Some("t")));
    assert!(!log_upload_offered(Some("https://h/m.json"), Some(" ")));
}
