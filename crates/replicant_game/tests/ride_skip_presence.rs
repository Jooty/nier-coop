use replicant_game::cutscene_skip::{
    PopVerdict, PromptSequences, SequenceChange, SkipPromptVote, VoteEvent, VoteView, skip_prompt,
};

const RIDE: u16 = 5410;
const RIDE_STARTS: &str = "Put_Boat_02";
const RIDE_ARRIVES: &str = "Put_Boat_03";
const RIDE_SKIP: &str = "Put_Boat_04";
const TOUR_PROMPT: u16 = 2437;
const AREA: u16 = 7;
const OTHER_AREA: u16 = 8;

fn view(host: bool, partner_present: bool, now_ms: u64) -> VoteView {
    VoteView {
        host,
        partner_present,
        page_shown_here: false,
        dialog_free: true,
        now_ms,
    }
}

fn riding(phase: u16) -> PromptSequences {
    let mut sequences = PromptSequences::new();
    sequences.enter_phase(Some(phase));
    sequences.ran_here(RIDE_STARTS);
    sequences
}

#[test]
fn the_rides_start_and_its_skip_put_a_game_in_and_out_of_its_sequence() {
    let mut sequences = PromptSequences::new();
    sequences.enter_phase(Some(AREA));
    assert_eq!(
        sequences.ran_here(RIDE_STARTS),
        Some(SequenceChange {
            prompt: RIDE,
            inside: true,
            by: RIDE_STARTS
        })
    );
    assert_eq!(sequences.ran_here(RIDE_STARTS), None);
    assert_eq!(sequences.ran_here("EID_1410_a0100g_06"), None);
    assert_eq!(
        sequences.ran_here(RIDE_SKIP),
        Some(SequenceChange {
            prompt: RIDE,
            inside: false,
            by: RIDE_SKIP
        })
    );
    assert_eq!(sequences.ran_here(RIDE_ARRIVES), None);
}

#[test]
fn the_rides_arrival_ends_it_too() {
    let mut sequences = riding(AREA);
    assert_eq!(
        sequences.ran_here(RIDE_ARRIVES).map(|change| change.inside),
        Some(false)
    );
    assert!(!sequences.here_inside(skip_prompt(RIDE).unwrap()));
}

#[test]
fn a_partner_has_a_say_only_while_it_reports_being_in_the_sequence() {
    let ride = skip_prompt(RIDE).unwrap();
    let mut sequences = riding(AREA);
    assert!(sequences.here_inside(ride));
    assert!(!sequences.partner_inside(ride));
    sequences.partner_reported(RIDE, true);
    assert!(sequences.partner_inside(ride));
    sequences.partner_reported(RIDE, false);
    assert!(!sequences.partner_inside(ride));
}

#[test]
fn a_prompt_with_no_known_start_keeps_every_partner_in_the_vote() {
    let tour = skip_prompt(TOUR_PROMPT).unwrap();
    let sequences = PromptSequences::new();
    assert!(sequences.partner_inside(tour));
    assert!(sequences.here_inside(tour));
}

#[test]
fn a_new_place_ends_both_players_sequences() {
    let ride = skip_prompt(RIDE).unwrap();
    let mut sequences = riding(AREA);
    sequences.partner_reported(RIDE, true);
    sequences.enter_phase(Some(AREA));
    assert!(sequences.partner_inside(ride));
    sequences.enter_phase(Some(OTHER_AREA));
    assert!(!sequences.here_inside(ride));
    assert!(!sequences.partner_inside(ride));
}

#[test]
fn the_hosts_ride_skip_goes_through_at_once_when_the_guest_is_not_on_the_ride() {
    // Run 1113l, host 00:56:19.513: the skip was held "until jay answers their own", but the
    // guest never boarded and could never answer.
    let sequences = riding(AREA);
    let present = sequences.partner_inside(skip_prompt(RIDE).unwrap());
    let mut vote = SkipPromptVote::new();
    let (verdict, step) = vote.local_pop(RIDE_SKIP, view(true, present, 19_513));
    assert_eq!(verdict, PopVerdict::Run);
    assert_eq!(step.event, Some(VoteEvent::SkipAlone));
    assert_eq!(step.send, None);
    assert_eq!(step.verdict, None);
    assert_eq!(vote.open_prompt(), None);
}

#[test]
fn a_held_skip_runs_alone_with_no_verdict_when_the_partner_leaves_the_sequence() {
    let mut vote = SkipPromptVote::new();
    let (verdict, _) = vote.local_pop(RIDE_SKIP, view(true, true, 19_513));
    assert_eq!(verdict, PopVerdict::Drop);
    let step = vote.tick(view(true, false, 21_000));
    assert_eq!(step.fire, Some(RIDE_SKIP));
    assert_eq!(step.event, Some(VoteEvent::SkipAlone));
    assert_eq!(step.verdict, None);
}
