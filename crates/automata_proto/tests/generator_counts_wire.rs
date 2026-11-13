use automata_game::enemies::GeneratorRunCounts;
use automata_proto::{
    GeneratorPhase, GeneratorState, GeneratorStates, MAX_MESSAGE_BYTES, Message, WireError, decode,
    encode,
};

fn worst_case_counts() -> GeneratorRunCounts {
    GeneratorRunCounts {
        enemies_of_one_run: i16::MAX,
        most_enemies_alive: i16::MAX,
        rest_time_bits: 86_400.0_f32.to_bits(),
        dead: i16::MIN,
    }
}

fn state_with(counts: Option<GeneratorRunCounts>) -> GeneratorState {
    GeneratorState {
        generator: u32::MAX,
        phase: GeneratorPhase::NotStarted,
        spawned: u32::MAX,
        counts,
    }
}

fn states_message(counts: GeneratorRunCounts) -> Message {
    Message::GeneratorStates(GeneratorStates {
        states: vec![state_with(Some(counts))],
    })
}

#[test]
fn a_full_list_of_generator_states_with_run_counts_still_fits_the_size_limit() {
    let full = Message::GeneratorStates(GeneratorStates {
        states: vec![state_with(Some(worst_case_counts())); 16],
    });
    assert!(encode(&full).expect("encodes").len() <= MAX_MESSAGE_BYTES);
}

#[test]
fn generator_states_with_and_without_run_counts_survive_a_round_trip() {
    for counts in [None, Some(worst_case_counts())] {
        let message = Message::GeneratorStates(GeneratorStates {
            states: vec![state_with(counts)],
        });
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn run_counts_no_generator_can_hold_are_refused() {
    let refused = [
        GeneratorRunCounts {
            enemies_of_one_run: -1,
            ..worst_case_counts()
        },
        GeneratorRunCounts {
            most_enemies_alive: -1,
            ..worst_case_counts()
        },
        GeneratorRunCounts {
            rest_time_bits: f32::NAN.to_bits(),
            ..worst_case_counts()
        },
        GeneratorRunCounts {
            rest_time_bits: (-1.0_f32).to_bits(),
            ..worst_case_counts()
        },
        GeneratorRunCounts {
            rest_time_bits: 86_401.0_f32.to_bits(),
            ..worst_case_counts()
        },
    ];
    for counts in refused {
        assert!(matches!(
            encode(&states_message(counts)),
            Err(WireError::OutOfRange { field: "states" })
        ));
    }
}
