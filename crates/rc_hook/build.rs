use std::process::Command;

const UNKNOWN_COMMIT: &str = "unknown";

fn main() {
    println!("cargo:rerun-if-env-changed=RC_BUILD_COMMIT");
    watch_git_head();
    let commit = from_env_or_git("RC_BUILD_COMMIT", &["rev-parse", "HEAD"])
        .unwrap_or_else(|| UNKNOWN_COMMIT.to_owned());
    println!("cargo:rustc-env=RC_BUILD_COMMIT={commit}");
}

fn from_env_or_git(variable: &str, git_args: &[&str]) -> Option<String> {
    if let Ok(value) = std::env::var(variable)
        && !value.trim().is_empty()
    {
        return Some(value.trim().to_owned());
    }
    git_output(git_args)
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn watch_git_head() {
    if let Some(head) = git_output(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(branch) = git_output(&["symbolic-ref", "-q", "HEAD"])
        && let Some(reference) = git_output(&["rev-parse", "--git-path", &branch])
    {
        println!("cargo:rerun-if-changed={reference}");
    }
}
