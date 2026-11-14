use nt_launcher::steam::{VdfError, VdfNode, library_paths, manifest_install_dir, parse_vdf};

const LIBRARY_FOLDERS: &str = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"/home/someone/.local/share/Steam"
		"label"		""
		"apps"
		{
			"228980"		"123"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
		"apps"
		{
			"1113560"		"456"
		}
	}
}
"#;

const OLD_LIBRARY_FOLDERS: &str = r#"
"LibraryFolders"
{
	"TimeNextStatsReport"		"1"
	"ContentStatsID"		"-1"
	"1"		"/mnt/old/SteamLibrary"
}
"#;

const APP_MANIFEST: &str = r#"
"AppState"
{
	"appid"		"1113560"
	"installdir"		"NieR Replicant ver.1.22474487139"
	// comment line
	"StateFlags"		"4"
}
"#;

#[test]
fn nested_blocks_and_escaped_backslashes_parse() {
    let root = parse_vdf(LIBRARY_FOLDERS).unwrap();
    let second = root
        .get("libraryfolders")
        .and_then(|folders| folders.get("1"))
        .and_then(|library| library.get("path"))
        .and_then(VdfNode::text);
    assert_eq!(second, Some("D:\\SteamLibrary"));
}

#[test]
fn library_paths_lists_every_numbered_library() {
    let paths = library_paths(LIBRARY_FOLDERS);
    assert_eq!(
        paths,
        vec![
            std::path::PathBuf::from("/home/someone/.local/share/Steam"),
            std::path::PathBuf::from("D:\\SteamLibrary"),
        ]
    );
}

#[test]
fn old_style_library_list_with_bare_paths_is_read() {
    let paths = library_paths(OLD_LIBRARY_FOLDERS);
    assert_eq!(
        paths,
        vec![std::path::PathBuf::from("/mnt/old/SteamLibrary")]
    );
}

#[test]
fn manifest_install_dir_is_read_and_comments_are_skipped() {
    assert_eq!(
        manifest_install_dir(APP_MANIFEST).as_deref(),
        Some("NieR Replicant ver.1.22474487139")
    );
}

#[test]
fn broken_files_give_nothing_instead_of_panicking() {
    assert!(library_paths("\"libraryfolders\" {").is_empty());
    assert_eq!(manifest_install_dir("garbage"), None);
    assert_eq!(parse_vdf("\"a\" \"b"), Err(VdfError::UnterminatedString));
    assert_eq!(parse_vdf("}"), Err(VdfError::UnexpectedClose));
}
