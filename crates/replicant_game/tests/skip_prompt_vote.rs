use replicant_game::cutscene_skip::{
    PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE, PopVerdict, PromptAnswer,
    SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS, SentAnswer, SkipPromptVote, VoteEvent, VoteView,
    skip_prompt, skip_prompt_answer, skip_prompt_ended_by,
};

const SKIP: &str = "EID_1410_a0100g_16_GuideSkip_01";
const OTHER: &str = "EID_1410_a0100g_16_GuideSkip_02";
const PROMPT: u16 = 2437;
const RIDE_SKIP: &str = "Put_Boat_04";
const RIDE_ARRIVES: &str = "Put_Boat_03";
const HOST_PROMPT_SHOWN_MS: u64 = 546;
const HOST_SKIP_POPPED_MS: u64 = 2_195;

fn view(host: bool, now_ms: u64) -> VoteView {
    VoteView {
        host,
        partner_present: true,
        page_shown_here: false,
        dialog_free: true,
        now_ms,
    }
}

fn answer(prompt: u16, skip: bool) -> Option<SentAnswer> {
    Some(SentAnswer { prompt, skip })
}

fn host_holding_its_skip() -> SkipPromptVote {
    let mut vote = SkipPromptVote::new();
    let (verdict, step) = vote.local_pop(SKIP, view(true, HOST_SKIP_POPPED_MS));
    assert_eq!(verdict, PopVerdict::Drop);
    assert_eq!(step.send, answer(PROMPT, true));
    assert_eq!(step.event, Some(VoteEvent::AskedAndHeld));
    vote
}

#[test]
fn the_tour_prompt_names_which_answer_skips() {
    assert_eq!(PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE.len(), 1);
    let (prompt, answer) = skip_prompt_answer(SKIP).unwrap();
    assert_eq!((prompt.prompt, answer), (PROMPT, PromptAnswer::Skip));
    let (prompt, answer) = skip_prompt_answer(OTHER).unwrap();
    assert_eq!((prompt.prompt, answer), (PROMPT, PromptAnswer::Other));
    assert_eq!(skip_prompt_answer("EID_1410_a0100g_15_01"), None);
    assert_eq!(skip_prompt(PROMPT).map(|row| row.skip_answer), Some(SKIP));
    assert_eq!(skip_prompt(2436), None);
}

#[test]
fn the_hosts_skip_is_held_until_the_guest_agrees_and_then_fires_once_with_its_verdict() {
    let mut vote = host_holding_its_skip();
    assert_eq!(vote.tick(view(true, HOST_SKIP_POPPED_MS + 16)).fire, None);
    let asks = vote.partner_answer(PROMPT, true, view(true, 3_000));
    assert_eq!(asks.event, Some(VoteEvent::PartnerAsks));
    let step = vote.tick(view(true, 3_016));
    assert_eq!(step.fire, Some(SKIP));
    assert_eq!(step.verdict, answer(PROMPT, true));
    assert_eq!(step.event, Some(VoteEvent::SkipAgreed));
    assert_eq!(vote.open_prompt(), None);
    let (released, _) = vote.local_pop(SKIP, view(true, 3_050));
    assert_eq!(released, PopVerdict::Run);
    for now_ms in [3_066, 3_600, 10_000] {
        assert_eq!(vote.tick(view(true, now_ms)).fire, None);
    }
}

#[test]
fn the_host_resends_its_ask_while_it_waits() {
    let mut vote = host_holding_its_skip();
    assert_eq!(vote.tick(view(true, HOST_SKIP_POPPED_MS + 100)).send, None);
    assert_eq!(
        vote.tick(view(true, HOST_SKIP_POPPED_MS + 500)).send,
        answer(PROMPT, true)
    );
}

#[test]
fn a_guest_who_asked_first_lets_the_hosts_skip_run_at_once_and_is_told() {
    let mut vote = SkipPromptVote::new();
    vote.partner_answer(PROMPT, true, view(true, 1_800));
    let (verdict, step) = vote.local_pop(SKIP, view(true, HOST_SKIP_POPPED_MS));
    assert_eq!(verdict, PopVerdict::Run);
    assert_eq!(step.event, Some(VoteEvent::SkipAgreed));
    assert_eq!(step.verdict, answer(PROMPT, true));
}

#[test]
fn with_no_partner_in_the_same_place_the_skip_runs_as_the_game_chose() {
    let mut vote = SkipPromptVote::new();
    let alone = VoteView {
        partner_present: false,
        ..view(true, HOST_SKIP_POPPED_MS)
    };
    let (verdict, step) = vote.local_pop(SKIP, alone);
    assert_eq!(verdict, PopVerdict::Run);
    assert_eq!(step.event, Some(VoteEvent::SkipAlone));
    assert_eq!(step.send, None);
}

#[test]
fn a_held_skip_fires_alone_when_the_partner_leaves() {
    let mut vote = host_holding_its_skip();
    let gone = VoteView {
        partner_present: false,
        ..view(true, 4_000)
    };
    let step = vote.tick(gone);
    assert_eq!(step.fire, Some(SKIP));
    assert_eq!(step.event, Some(VoteEvent::SkipAlone));
}

#[test]
fn a_guest_who_declines_makes_the_host_fire_the_other_answer() {
    let mut vote = host_holding_its_skip();
    let step = vote.partner_answer(PROMPT, false, view(true, 4_000));
    assert_eq!(step.fire, Some(OTHER));
    assert_eq!(step.verdict, answer(PROMPT, false));
    assert_eq!(step.event, Some(VoteEvent::DeclinedByPartner));
    let (released, _) = vote.local_pop(OTHER, view(true, 4_020));
    assert_eq!(released, PopVerdict::Run);
}

#[test]
fn a_guest_decline_that_came_first_turns_the_hosts_skip_into_the_other_answer() {
    let mut vote = SkipPromptVote::new();
    vote.partner_answer(PROMPT, false, view(true, 1_500));
    let (verdict, step) = vote.local_pop(SKIP, view(true, HOST_SKIP_POPPED_MS));
    assert_eq!(verdict, PopVerdict::Drop);
    assert_eq!(step.fire, Some(OTHER));
}

#[test]
fn the_hosts_no_runs_at_once_and_is_its_verdict() {
    let mut vote = SkipPromptVote::new();
    let (verdict, step) = vote.local_pop(OTHER, view(true, HOST_SKIP_POPPED_MS));
    assert_eq!(verdict, PopVerdict::Run);
    assert_eq!(step.verdict, answer(PROMPT, false));
    assert_eq!(step.fire, None);
}

#[test]
fn the_host_waits_no_longer_than_the_give_up_time_and_then_skips_nothing() {
    let mut vote = host_holding_its_skip();
    let before = HOST_SKIP_POPPED_MS + SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS - 1;
    assert_eq!(vote.tick(view(true, before)).fire, None);
    let step = vote.tick(view(true, before + 1));
    assert_eq!(step.fire, Some(OTHER));
    assert_eq!(step.verdict, answer(PROMPT, false));
    assert_eq!(step.event, Some(VoteEvent::GaveUp));
}

#[test]
fn an_unfocused_guest_is_shown_the_prompt_and_never_answered_for() {
    // The run: host's ask at 17:20:40.323, the guest's unfocused window "agreed" 23 ms later.
    let mut host = host_holding_its_skip();
    let mut guest = SkipPromptVote::new();
    let shown = guest.partner_answer(PROMPT, true, view(false, 2_218));
    assert_eq!(shown.event, Some(VoteEvent::PartnerAsks));
    assert_eq!(guest.tick(view(false, 2_241)).open_page, Some(PROMPT));
    let page_up = VoteView {
        page_shown_here: true,
        dialog_free: false,
        ..view(false, 0)
    };
    for now_ms in (2_257..20_000).step_by(500) {
        guest.partner_answer(PROMPT, true, VoteView { now_ms, ..page_up });
        let step = guest.tick(VoteView { now_ms, ..page_up });
        assert_eq!(step.send, None);
        assert_eq!(step.event, None);
        assert_eq!(step.fire, None);
        assert!(!step.close_page);
        assert_eq!(host.tick(view(true, now_ms)).fire, None);
    }
}

#[test]
fn either_player_may_ask_first_and_the_other_agrees_by_answering_its_own_prompt() {
    let mut host = SkipPromptVote::new();
    let mut guest = SkipPromptVote::new();
    let (verdict, asked) = guest.local_pop(SKIP, view(false, 1_000));
    assert_eq!(verdict, PopVerdict::Drop);
    assert_eq!(asked.send, answer(PROMPT, true));
    let shown = host.partner_answer(PROMPT, true, view(true, 1_030));
    assert_eq!(shown.event, Some(VoteEvent::PartnerAsks));
    assert_eq!(host.tick(view(true, 1_046)).fire, None);
    let (verdict, step) = host.local_pop(SKIP, view(true, 1_900));
    assert_eq!(verdict, PopVerdict::Run);
    assert_eq!(step.verdict, answer(PROMPT, true));
    let here = guest.host_verdict(PROMPT, true);
    assert_eq!(here.fire, Some(SKIP));
    assert_eq!(here.event, Some(VoteEvent::HostChoseHere));
    let (released, _) = guest.local_pop(SKIP, view(false, 1_960));
    assert_eq!(released, PopVerdict::Run);
}

#[test]
fn an_agreed_skip_runs_on_the_guest_too_even_where_it_never_answered() {
    let mut guest = SkipPromptVote::new();
    guest.partner_answer(PROMPT, true, view(false, 2_218));
    let step = guest.host_verdict(PROMPT, true);
    assert_eq!(step.fire, Some(SKIP));
    let (released, _) = guest.local_pop(SKIP, view(false, 2_300));
    assert_eq!(released, PopVerdict::Run);
}

#[test]
fn the_hosts_other_answer_runs_on_the_guest_only_where_it_holds_its_skip() {
    let mut waiting = SkipPromptVote::new();
    waiting.partner_answer(PROMPT, true, view(false, 2_218));
    let step = waiting.host_verdict(PROMPT, false);
    assert_eq!(step.fire, None);
    assert_eq!(waiting.open_prompt(), None);
    let mut holding = SkipPromptVote::new();
    holding.local_pop(SKIP, view(false, 2_500));
    assert_eq!(holding.host_verdict(PROMPT, false).fire, Some(OTHER));
}

#[test]
fn a_guests_no_runs_at_once_and_is_sent_to_the_host() {
    let mut guest = SkipPromptVote::new();
    guest.partner_answer(PROMPT, true, view(false, 2_210));
    let (verdict, step) = guest.local_pop(OTHER, view(false, 3_000));
    assert_eq!(verdict, PopVerdict::Run);
    assert_eq!(step.send, answer(PROMPT, false));
    assert_eq!(step.fire, None);
}

#[test]
fn a_guest_vote_with_no_verdict_never_hangs() {
    let mut guest = SkipPromptVote::new();
    guest.local_pop(SKIP, view(false, 1_000));
    let mut last = None;
    for now_ms in (1_016..40_000).step_by(250) {
        let step = guest.tick(view(false, now_ms));
        if step.event.is_some() {
            last = Some(step);
            break;
        }
    }
    let step = last.unwrap();
    assert_eq!(step.event, Some(VoteEvent::GaveUp));
    assert_eq!(step.fire, Some(OTHER));
    assert_eq!(step.send, answer(PROMPT, false));
    assert_eq!(guest.open_prompt(), None);
}

#[test]
fn the_guests_vote_ends_and_its_page_closes_when_the_hosts_answer_runs_there() {
    let mut guest = SkipPromptVote::new();
    guest.partner_answer(PROMPT, true, view(false, 2_210));
    assert_eq!(guest.tick(view(false, 2_226)).open_page, Some(PROMPT));
    let step = guest.host_answer_ran(SKIP);
    assert_eq!(step.event, Some(VoteEvent::HostAnswerRan));
    assert!(step.close_page);
    assert_eq!(guest.open_prompt(), None);
}

#[test]
fn a_host_at_its_prompt_is_never_answered_for() {
    let mut host = SkipPromptVote::new();
    host.partner_answer(PROMPT, true, view(true, HOST_PROMPT_SHOWN_MS + 900));
    let attended = VoteView {
        page_shown_here: true,
        dialog_free: false,
        ..view(true, 0)
    };
    for now_ms in (HOST_PROMPT_SHOWN_MS + 916..40_000).step_by(500) {
        let step = host.tick(VoteView { now_ms, ..attended });
        assert_eq!(step.fire, None);
        assert!(!step.close_page);
    }
}

#[test]
fn a_callback_outside_the_table_is_never_touched() {
    let mut vote = SkipPromptVote::new();
    for host in [true, false] {
        let (verdict, step) = vote.local_pop("EID_1410_a0100g_16", view(host, 0));
        assert_eq!(verdict, PopVerdict::Run);
        assert_eq!(step.send, None);
    }
}

#[test]
fn the_rides_skip_and_arrival_are_the_riders_own_and_never_a_vote() {
    assert_eq!(skip_prompt_answer(RIDE_SKIP), None);
    assert_eq!(skip_prompt_ended_by(RIDE_ARRIVES), None);
    assert_eq!(skip_prompt(5410), None);
    let mut vote = SkipPromptVote::new();
    for host in [true, false] {
        let (verdict, step) = vote.local_pop(RIDE_SKIP, view(host, 20_086));
        assert_eq!(verdict, PopVerdict::Run);
        assert_eq!(step.send, None);
        assert_eq!(step.fire, None);
    }
    assert_eq!(vote.open_prompt(), None);
}
