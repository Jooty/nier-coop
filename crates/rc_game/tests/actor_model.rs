use rc_game::offsets::ACTOR_MODEL_RVA;
use rc_game::{ActorModel, Replicant};

use crate::fake_memory::FakeMemory;

#[test]
fn set_actor_model_writes_seven_padded_bytes() {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(ACTOR_MODEL_RVA, b"kaineE\0");
    let mut game = Replicant::new(memory);
    game.set_actor_model(ActorModel::NierT)
        .expect("model text is writable");
    assert_eq!(game.memory().get_at_rva(ACTOR_MODEL_RVA, 7), b"nierT\0\0");
    assert_eq!(game.actor_model_name().as_deref(), Ok("nierT"));
}

#[test]
fn model_names_round_trip_and_unknown_names_are_none() {
    for model in [
        ActorModel::NierB,
        ActorModel::NierT,
        ActorModel::NierF,
        ActorModel::NierY,
        ActorModel::KaineE,
    ] {
        assert_eq!(ActorModel::from_name(model.name()), Some(model));
    }
    assert_eq!(ActorModel::from_name("nier010"), None);
}
