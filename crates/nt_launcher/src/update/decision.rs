use crate::update::{BUILD_COMMIT, BUILD_COMMIT_TIME_TEXT, Manifest, SHORT_COMMIT_LENGTH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnBuild {
    pub commit: String,
    pub commit_time: u64,
}

pub fn own_build() -> OwnBuild {
    OwnBuild {
        commit: BUILD_COMMIT.to_owned(),
        commit_time: BUILD_COMMIT_TIME_TEXT.parse().unwrap_or(0),
    }
}

pub fn is_newer(own: &OwnBuild, manifest: &Manifest) -> bool {
    if manifest.commit == own.commit {
        return false;
    }
    own.commit_time == 0 || manifest.commit_time > own.commit_time
}

pub fn short_commit(commit: &str) -> &str {
    commit.get(..SHORT_COMMIT_LENGTH).unwrap_or(commit)
}
