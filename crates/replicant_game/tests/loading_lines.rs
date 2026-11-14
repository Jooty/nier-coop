use nc_hook::loading_text::{LoadingText, SessionPartner, fill_partner};
use replicant_game::loading_lines::DIARY_ENTRY_TEMPLATES;

#[test]
fn the_diary_entries_are_the_four_agreed_ones() {
    assert_eq!(
        DIARY_ENTRY_TEMPLATES,
        [
            "{partner} came with me again today. Neither of us said much on the road. We didn't need to.",
            "Two sets of footprints on the way home. I counted them twice, just to be sure.",
            "I told {partner} to stay close when the Shades came. They didn't listen. Neither did I.",
            "Some days the road feels shorter. Today was one of those days. {partner} was there.",
        ]
    );
}

#[test]
fn every_diary_entry_fits_the_loading_screen_with_the_longest_name() {
    let longest_name = "n".repeat(31);
    for template in DIARY_ENTRY_TEMPLATES {
        let filled = fill_partner(template, Some(&longest_name)).expect("fits");
        assert!(!filled.contains("{partner}"));
    }
}

#[test]
fn a_partner_name_never_reaches_a_screen_without_one() {
    let mut text = LoadingText::new(&DIARY_ENTRY_TEMPLATES);
    for _ in 0..300 {
        if let Some(choice) = text.choose(SessionPartner::NoPartnerYet) {
            assert_eq!(choice.line_index, 1);
        }
    }
}
