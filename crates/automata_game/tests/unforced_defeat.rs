use automata_game::enemies::{
    EnemyHereForADefeat, EnemyId, EnemySighting, UnforcedDefeatReason, unforced_defeat_line,
    unforced_defeat_reason,
};

const A_LAYOUT: u32 = 0x5A01_77C2;
const AN_OBJECT: u32 = 0x2_0000;
const ANOTHER_OBJECT: u32 = 0x2_0010;

fn id() -> EnemyId {
    EnemyId::Placed {
        layout: A_LAYOUT,
        entry: 7,
    }
}

fn here(health: i32, uses_the_base_damage_code: bool) -> Option<EnemyHereForADefeat> {
    Some(EnemyHereForADefeat {
        sighting: EnemySighting {
            id: id(),
            object_id: AN_OBJECT,
            health,
            max_health: 200,
        },
        uses_the_base_damage_code,
    })
}

#[test]
fn an_enemy_that_will_be_forced_has_no_reason() {
    assert_eq!(
        unforced_defeat_reason(here(150, true), false, Some(AN_OBJECT)),
        None
    );
    assert_eq!(unforced_defeat_reason(here(150, true), false, None), None);
}

#[test]
fn each_check_of_the_forcing_gives_its_own_reason() {
    assert_eq!(
        unforced_defeat_reason(None, false, Some(AN_OBJECT)),
        Some(UnforcedDefeatReason::NotOnThisCopy)
    );
    assert_eq!(
        unforced_defeat_reason(here(150, true), true, Some(AN_OBJECT)),
        Some(UnforcedDefeatReason::IdSharedByTwoEnemiesHere)
    );
    assert_eq!(
        unforced_defeat_reason(here(150, false), false, Some(AN_OBJECT)),
        Some(UnforcedDefeatReason::ClassHasItsOwnDamageCode)
    );
    assert_eq!(
        unforced_defeat_reason(here(0, true), false, Some(AN_OBJECT)),
        Some(UnforcedDefeatReason::AlreadyDefeatedHere)
    );
    assert_eq!(
        unforced_defeat_reason(here(150, true), false, Some(ANOTHER_OBJECT)),
        Some(UnforcedDefeatReason::HostHoldsAnotherObjectUnderTheId)
    );
}

#[test]
fn the_line_names_the_enemy_the_wait_and_the_reason() {
    assert_eq!(
        unforced_defeat_line(id(), UnforcedDefeatReason::AlreadyDefeatedHere, 1_600),
        "enemies: the host's defeat of layout 0x5A0177C2 entry 0x7 heard 1600 ms ago is not forced on this copy: it is already defeated on this copy"
    );
    for reason in [
        UnforcedDefeatReason::NotOnThisCopy,
        UnforcedDefeatReason::IdSharedByTwoEnemiesHere,
        UnforcedDefeatReason::ClassHasItsOwnDamageCode,
        UnforcedDefeatReason::HostHoldsAnotherObjectUnderTheId,
    ] {
        assert!(unforced_defeat_line(id(), reason, 0).contains("is not forced on this copy: "));
    }
}
