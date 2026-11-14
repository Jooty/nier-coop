use crate::defeat::{DEFEAT_FLOW_GAME_OVER_MENU_STATE, DEFEAT_FLOW_MENU_OPENS_STEP};
use crate::load_gate::PlaceRelation;
use crate::minute_line_budget::MinuteLineBudget;
use crate::shared_defeat::{
    DefeatNote, DefeatWay, GameOverChoice, GameOverMenu, GameOverMenuWrite,
    HEARD_DEFEAT_NOTE_KEPT_FOR_MS, RestorePlace, SHARED_DEFEAT_LINES_LOGGED_PER_MINUTE,
    SHARED_DEFEAT_WAIT_LINE_INTERVAL_MS, SharedDefeatReadings, SharedDefeatStep, follow_line,
    heard_choice_after_own_line, heard_choice_dropped_for_the_way_line, heard_choice_gave_up_line,
    heard_choice_line, heard_choice_waits_line, local_flow_over_line, local_way_line,
    no_partner_any_more_line, own_choice_first_line, own_choice_line, partner_way_line,
    partner_way_text, restore_text, retry_not_followed_line, ways_differ_line,
};

const SHARED_DEFEAT_LINES_TOPIC: &str = "shared defeat";

/// One defeat for the pair: each copy tells its way on and its first menu choice, and makes the
/// other copy's first menu choice in its own menu when that menu waits for one.
#[derive(Debug, Clone)]
pub struct SharedDefeatMachine {
    local_flow_runs: bool,
    told_way: Option<DefeatWay>,
    taken: Option<GameOverChoice>,
    menu_before: Option<GameOverMenu>,
    heard_choice: Option<HeardChoice>,
    partner_way: Option<HeardWay>,
    budget: MinuteLineBudget,
}

impl Default for SharedDefeatMachine {
    fn default() -> Self {
        Self {
            local_flow_runs: false,
            told_way: None,
            taken: None,
            menu_before: None,
            heard_choice: None,
            partner_way: None,
            budget: MinuteLineBudget::new(SHARED_DEFEAT_LINES_LOGGED_PER_MINUTE),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct HeardChoice {
    choice: GameOverChoice,
    at_ms: u64,
    wait_line_at_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct HeardWay {
    way: DefeatWay,
    restore: Option<RestorePlace>,
    at_ms: u64,
}

impl SharedDefeatMachine {
    pub fn new() -> Self {
        Self::default()
    }

    /// The other copy's choice, heard and not yet made here.
    pub fn heard_choice(&self) -> Option<GameOverChoice> {
        self.heard_choice.map(|heard| heard.choice)
    }

    pub fn step(&mut self, readings: &SharedDefeatReadings) -> SharedDefeatStep {
        let mut step = SharedDefeatStep::default();
        if !readings.campaign_pair() {
            if self.local_flow_runs || self.heard_choice.is_some() {
                step.logs.push(no_partner_any_more_line());
            }
            let budget = self.budget.clone();
            *self = Self {
                budget,
                ..Self::default()
            };
            return self.counted(readings.now_ms, step);
        }
        for note in &readings.heard {
            self.hear(*note, readings, &mut step);
        }
        self.watch_the_way(readings, &mut step);
        self.watch_the_menu(readings, &mut step);
        self.age_the_heard_choice(readings, &mut step);
        self.counted(readings.now_ms, step)
    }

    fn counted(&mut self, now_ms: u64, mut step: SharedDefeatStep) -> SharedDefeatStep {
        step.logs = std::mem::take(&mut step.logs)
            .into_iter()
            .flat_map(|line| {
                self.budget
                    .lines_for(now_ms, SHARED_DEFEAT_LINES_TOPIC, line)
            })
            .collect();
        step
    }

    fn hear(
        &mut self,
        note: DefeatNote,
        readings: &SharedDefeatReadings,
        step: &mut SharedDefeatStep,
    ) {
        match note {
            DefeatNote::Way { way, restore } => {
                self.partner_way = Some(HeardWay {
                    way,
                    restore,
                    at_ms: readings.now_ms,
                });
                let relation = match (restore, readings.restore_place) {
                    (Some(there), Some(here)) => here.place.relation_to(&there.place),
                    _ => PlaceRelation::Unknown,
                };
                step.logs.push(partner_way_line(
                    readings.role,
                    &partner_way_text(way, restore),
                    self.told_way,
                    &restore_text(readings.restore_point, readings.restore_place),
                    relation,
                ));
                if let Some(local) = self.told_way
                    && local != way
                {
                    step.logs.push(ways_differ_line(local, way));
                }
            }
            DefeatNote::MenuChoice(choice) => {
                if let Some(own) = self.taken {
                    step.logs.push(heard_choice_after_own_line(choice, own));
                    return;
                }
                self.heard_choice = Some(HeardChoice {
                    choice,
                    at_ms: readings.now_ms,
                    wait_line_at_ms: readings.now_ms,
                });
                step.logs.push(heard_choice_line(choice, readings.flow));
            }
        }
    }

    fn watch_the_way(&mut self, readings: &SharedDefeatReadings, step: &mut SharedDefeatStep) {
        let Some(flow) = readings.flow else {
            return;
        };
        if !flow.is_active() {
            if self.local_flow_runs {
                step.logs.push(local_flow_over_line());
                self.local_flow_runs = false;
                self.told_way = None;
                self.taken = None;
                self.menu_before = None;
            }
            return;
        }
        self.local_flow_runs = true;
        let Some(way) = DefeatWay::of_flow(flow) else {
            return;
        };
        if self.told_way == Some(way) {
            return;
        }
        self.told_way = Some(way);
        step.tell.push(DefeatNote::Way {
            way,
            restore: readings.restore_place,
        });
        let partner = self
            .partner_way
            .filter(|heard| {
                readings.now_ms.saturating_sub(heard.at_ms) <= HEARD_DEFEAT_NOTE_KEPT_FOR_MS
            })
            .map(|heard| partner_way_text(heard.way, heard.restore));
        step.logs.push(local_way_line(
            way,
            &restore_text(readings.restore_point, readings.restore_place),
            partner.as_deref(),
        ));
        if way != DefeatWay::GameOverMenu
            && let Some(heard) = self.heard_choice.take()
        {
            step.logs
                .push(heard_choice_dropped_for_the_way_line(heard.choice, way));
        }
    }

    fn watch_the_menu(&mut self, readings: &SharedDefeatReadings, step: &mut SharedDefeatStep) {
        let (Some(flow), Some(menu)) = (readings.flow, readings.menu) else {
            self.menu_before = None;
            return;
        };
        // Before the flow opens the menu its words still hold the last menu's.
        if flow.state != DEFEAT_FLOW_GAME_OVER_MENU_STATE || flow.step < DEFEAT_FLOW_MENU_OPENS_STEP
        {
            self.menu_before = None;
            return;
        }
        if let Some(before) = self.menu_before.replace(menu)
            && self.taken.is_none()
            && let Some(choice) = menu.choice_since(before)
        {
            self.taken = Some(choice);
            step.tell.push(DefeatNote::MenuChoice(choice));
            step.logs
                .push(own_choice_line(choice, readings.local_unattended));
            if let Some(heard) = self.heard_choice.take() {
                step.logs.push(own_choice_first_line(heard.choice, choice));
            }
            return;
        }
        if self.taken.is_some() || !flow.waits_in_the_game_over_menu() || !menu.is_choosing() {
            return;
        }
        let Some(heard) = self.heard_choice.take() else {
            return;
        };
        let write = match heard.choice {
            GameOverChoice::Title => GameOverMenuWrite::Title,
            GameOverChoice::Retry if readings.in_place_retry => GameOverMenuWrite::RetryInPlace,
            GameOverChoice::Retry => {
                step.logs.push(retry_not_followed_line());
                return;
            }
        };
        self.taken = Some(heard.choice);
        step.menu_write = Some(write);
        step.logs.push(follow_line(
            heard.choice,
            write,
            readings.now_ms.saturating_sub(heard.at_ms),
            readings.local_unattended,
        ));
    }

    fn age_the_heard_choice(
        &mut self,
        readings: &SharedDefeatReadings,
        step: &mut SharedDefeatStep,
    ) {
        let Some(heard) = self.heard_choice.as_mut() else {
            return;
        };
        let waited_ms = readings.now_ms.saturating_sub(heard.at_ms);
        if waited_ms > HEARD_DEFEAT_NOTE_KEPT_FOR_MS {
            step.logs
                .push(heard_choice_gave_up_line(heard.choice, readings.flow));
            self.heard_choice = None;
            return;
        }
        if readings.now_ms.saturating_sub(heard.wait_line_at_ms)
            >= SHARED_DEFEAT_WAIT_LINE_INTERVAL_MS
        {
            heard.wait_line_at_ms = readings.now_ms;
            step.logs.push(heard_choice_waits_line(
                heard.choice,
                waited_ms,
                readings.flow,
            ));
        }
    }
}
