use nc_wire::{
    BuildGate, Hello, Member, MemberPing, Ping, Pong, Reject, RejectReason, STORY_MODE,
    SessionRules, Welcome,
};
use serde::Serialize;

const GOLDEN_HEX: &str = include_str!("golden_wire.hex");

fn hex_of<T: Serialize>(value: &T) -> String {
    let bytes = postcard::to_stdvec(value).expect("a sample serializes");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn build_gate() -> BuildGate {
    BuildGate {
        time_date_stamp: 0x1122_3344,
        size_of_image: 0x0055_6677,
    }
}

fn session_rules() -> SessionRules {
    SessionRules {
        mode: STORY_MODE,
        max_players: 2,
    }
}

fn reject(reason: RejectReason) -> Reject {
    Reject { reason }
}

fn samples() -> Vec<(&'static str, String)> {
    vec![
        ("BuildGate", hex_of(&build_gate())),
        (
            "Hello",
            hex_of(&Hello {
                version: 300,
                build: build_gate(),
                name: "Nier".to_owned(),
            }),
        ),
        (
            "Welcome",
            hex_of(&Welcome {
                version: 300,
                slot: 1,
                host_name: "Host".to_owned(),
                rules: session_rules(),
            }),
        ),
        (
            "RejectVersionMismatch",
            hex_of(&reject(RejectReason::VersionMismatch)),
        ),
        (
            "RejectBuildMismatch",
            hex_of(&reject(RejectReason::BuildMismatch)),
        ),
        (
            "RejectSessionFull",
            hex_of(&reject(RejectReason::SessionFull)),
        ),
        ("RejectMalformed", hex_of(&reject(RejectReason::Malformed))),
        (
            "RejectRemovedByHost",
            hex_of(&reject(RejectReason::RemovedByHost)),
        ),
        ("Ping", hex_of(&Ping { nonce: 0xDEAD_BEEF })),
        ("Pong", hex_of(&Pong { nonce: 0xFEED_FACE })),
        (
            "Member",
            hex_of(&Member {
                slot: 1,
                name: "Kaine".to_owned(),
            }),
        ),
        (
            "MemberPing",
            hex_of(&MemberPing {
                slot: 1,
                rtt_ms: 42,
            }),
        ),
        ("SessionRules", hex_of(&session_rules())),
    ]
}

fn golden_lines() -> Vec<(&'static str, &'static str)> {
    GOLDEN_HEX
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_once(' ').expect("a golden line is `Name hex`"))
        .collect()
}

#[test]
fn every_link_type_keeps_its_encoded_bytes() {
    let golden = golden_lines();
    let samples = samples();
    for (index, (name, actual)) in samples.iter().enumerate() {
        let Some((golden_name, golden_hex)) = golden.get(index) else {
            panic!("{name} (sample {index}) has no line in golden_wire.hex: `{name} {actual}`");
        };
        assert_eq!(golden_name, name, "golden line {index} names another type");
        assert_eq!(golden_hex, actual, "the encoding of {name} changed");
    }
    assert_eq!(
        golden.len(),
        samples.len(),
        "golden_wire.hex has lines that have no sample"
    );
}
