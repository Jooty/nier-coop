use automata_game::address::{
    CURSOR_RECENTRE_FUNCTION, KEYBOARD_INPUT_SOURCE, MOUSE_INPUT_SOURCE, PAD_INPUT_SOURCE,
    POLL_DEVICES_FUNCTION, POLL_MOUSE_FUNCTION,
};
use automata_game::exe::ExeVariant;
use automata_game::input_block::{InputHook, SourceSpan, input_block_plan, prologue_matches};

#[test]
fn a_closed_menu_changes_nothing() {
    for variant in ExeVariant::ALL {
        let plan = input_block_plan(false, variant);
        assert!(plan.zero_after_poll_devices.is_empty());
        assert_eq!(plan.mouse_active_override, None);
        assert!(!plan.skip_cursor_recentre);
    }
}

#[test]
fn an_open_menu_zeroes_pads_then_keyboard_then_mouse_in_that_order() {
    for variant in ExeVariant::ALL {
        let plan = input_block_plan(true, variant);
        assert_eq!(
            plan.zero_after_poll_devices,
            vec![
                SourceSpan {
                    rva: PAD_INPUT_SOURCE.rva(variant),
                    len: 0x200
                },
                SourceSpan {
                    rva: KEYBOARD_INPUT_SOURCE.rva(variant),
                    len: 0x68
                },
                SourceSpan {
                    rva: MOUSE_INPUT_SOURCE.rva(variant),
                    len: 0x34
                },
            ]
        );
    }
}

#[test]
fn an_open_menu_makes_the_mouse_poll_inactive_and_skips_the_recentre() {
    for variant in ExeVariant::ALL {
        let plan = input_block_plan(true, variant);
        assert_eq!(plan.mouse_active_override, Some(0));
        assert!(plan.skip_cursor_recentre);
    }
}

#[test]
fn the_keyboard_span_stops_before_the_direct_input_pointer() {
    let plan = input_block_plan(true, ExeVariant::Main);
    let keyboard = plan.zero_after_poll_devices[1];
    assert_eq!(keyboard.rva + keyboard.len, 0x178_0018);
}

#[test]
fn the_sources_sit_at_the_noted_addresses_of_both_exes() {
    assert_eq!(PAD_INPUT_SOURCE.main, 0x177_FD80);
    assert_eq!(PAD_INPUT_SOURCE.compat, 0x177_3BA0);
    assert_eq!(KEYBOARD_INPUT_SOURCE.compat, 0x177_3DD0);
    assert_eq!(MOUSE_INPUT_SOURCE.compat, 0x179_16E8);
}

#[test]
fn the_hook_points_sit_at_the_noted_addresses_of_both_exes() {
    assert_eq!(POLL_DEVICES_FUNCTION.main, 0x27_7CD0);
    assert_eq!(POLL_DEVICES_FUNCTION.compat, 0x27_24B0);
    assert_eq!(POLL_MOUSE_FUNCTION.main, 0x27_8FB0);
    assert_eq!(POLL_MOUSE_FUNCTION.compat, 0x27_3790);
    assert_eq!(CURSOR_RECENTRE_FUNCTION.main, 0x27_84F0);
    assert_eq!(CURSOR_RECENTRE_FUNCTION.compat, 0x27_2CD0);
    for hook in InputHook::ALL {
        assert_ne!(hook.address().main, hook.address().compat);
    }
}

#[test]
fn a_hook_is_only_placed_where_the_noted_first_bytes_are() {
    let poll_devices = [
        0x40, 0x53, 0x41, 0x56, 0x48, 0x81, 0xEC, 0x68, 0x01, 0x00, 0x00, 0x90,
    ];
    assert!(prologue_matches(InputHook::PollDevices, &poll_devices));
    assert!(!prologue_matches(InputHook::PollMouse, &poll_devices));
    let mut changed = poll_devices;
    changed[3] = 0x00;
    assert!(!prologue_matches(InputHook::PollDevices, &changed));
    assert!(!prologue_matches(InputHook::CursorRecentre, &[0x48, 0x83]));
}

#[test]
fn every_hook_has_a_distinct_label() {
    let labels: std::collections::BTreeSet<&str> =
        InputHook::ALL.iter().map(|hook| hook.label()).collect();
    assert_eq!(labels.len(), 3);
}
