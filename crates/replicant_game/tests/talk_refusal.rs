use replicant_game::npcs::{NPC_TALK_COMMAND_IDS, refuses_talk_start, talk_busy_notice};

#[test]
fn a_talk_start_on_a_villager_the_partner_talks_to_is_refused() {
    for command in NPC_TALK_COMMAND_IDS {
        assert!(refuses_talk_start(command, true));
    }
}

#[test]
fn a_free_villager_or_another_command_is_never_refused() {
    for command in NPC_TALK_COMMAND_IDS {
        assert!(!refuses_talk_start(command, false));
    }
    assert!(!refuses_talk_start(0x14, true));
}

#[test]
fn the_notice_names_the_partner() {
    assert_eq!(talk_busy_notice("Ana"), "Ana is talking to them");
}
