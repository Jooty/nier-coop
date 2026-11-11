use replicant_game::enemies::{
    OwnHitInputs, OwnHitLimiter, OwnHitRecord, own_hit_text, spec_class_of,
};
use replicant_proto::PartnerBody;

#[test]
fn the_first_twenty_hits_of_a_phase_log_then_every_tenth() {
    let mut limiter = OwnHitLimiter::default();
    let logged = (1..=50).filter(|_| limiter.allow(7)).count();
    assert_eq!(logged, 20 + 3);
}

#[test]
fn a_new_phase_starts_the_count_again() {
    let mut limiter = OwnHitLimiter::default();
    for _ in 0..30 {
        limiter.allow(1);
    }
    assert!(limiter.allow(2));
}

#[test]
fn the_class_follows_the_body() {
    assert_eq!(spec_class_of(PartnerBody::Nier), "PlayerBoy");
    assert_eq!(
        spec_class_of(PartnerBody::PrologueCharacter),
        "PlayerDefault"
    );
    assert_eq!(spec_class_of(PartnerBody::SeedKaineE), "PlayerKaine");
}

#[test]
fn the_line_names_every_input() {
    let inputs = OwnHitInputs {
        weapon: Some(1000),
        weapon_level: Some(2),
        level: Some(5),
        kaine_flag: Some(0),
        cached_attack: Some((0x4120_0000, 10.0)),
        mode: Some(1),
        mode_percent: Some(100),
    };
    let record = OwnHitRecord {
        hp_damage: 141,
        second_gauge: 3,
        third_gauge: 4,
    };
    assert_eq!(
        own_hit_text(
            &inputs,
            record,
            Some("NIER013_atk"),
            Some(("nierT", "PlayerDefault"))
        ),
        "own hit: weapon 1000 lv 2, level 5, kaine flag 0, base 0x41200000 (10.00), mode 1/100, words n/a, record 141/3/4, motion NIER013_atk, hp_damage 141 (seed nierT, class PlayerDefault)"
    );
}

#[test]
fn unreadable_inputs_show_a_question_mark() {
    let record = OwnHitRecord {
        hp_damage: 1,
        second_gauge: 0,
        third_gauge: 0,
    };
    let text = own_hit_text(&OwnHitInputs::default(), record, None, None);
    assert!(text.contains("weapon ? lv ?, level ?"));
    assert!(text.ends_with("(seed ?, class ?)"));
}
