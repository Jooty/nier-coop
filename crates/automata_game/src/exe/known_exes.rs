use crate::exe::{ExeFacts, ExeIdentity, ExeVariant};

pub const NEW_MAIN_EXE: ExeIdentity = ExeIdentity {
    name: "new main",
    variant: ExeVariant::Main,
    file_size: 17_613_568,
    time_date_stamp: 0x6AC8_9519,
    sha256_hex: "09854c2d45d9a72e66357e72f50cee379bb2eafe35b3d64e5e88f4eddc8613fa",
};

pub const OLD_MAIN_EXE: ExeIdentity = ExeIdentity {
    name: "old main",
    variant: ExeVariant::Main,
    file_size: 17_601_536,
    time_date_stamp: 0x6AC8_9519,
    sha256_hex: "6e038e27c2c707afbadb53b3bf15cc46b4d7c018584206280cbaf468bf24e468",
};

pub const COMPAT_EXE: ExeIdentity = ExeIdentity {
    name: "compat",
    variant: ExeVariant::Compat,
    file_size: 17_550_336,
    time_date_stamp: 0x6AC8_9501,
    sha256_hex: "8a510d8d6f5691b69dceddc335508b2dc6cdff9274c236bd6cc80b14e548500c",
};

pub const KNOWN_EXES: [ExeIdentity; 3] = [NEW_MAIN_EXE, OLD_MAIN_EXE, COMPAT_EXE];

pub fn identify(facts: &ExeFacts) -> Option<&'static ExeIdentity> {
    KNOWN_EXES.iter().find(|known| known.matches(facts))
}
