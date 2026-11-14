use crate::address::GameAddress;
use crate::story_steps::StepSort;

/// Whether the research read a row's sorting in this build's code or reasoned it from the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Proven,
    Guess,
}

/// One row of `step_classes.tsv`: a story action class by its RTTI name and vftable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepClassRow {
    pub name: &'static str,
    pub vftable: GameAddress,
    pub sort: StepSort,
    pub basis: Basis,
    pub reason: &'static str,
}

impl Basis {
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "proven" => Some(Self::Proven),
            "guess" => Some(Self::Guess),
            _ => None,
        }
    }
}

impl StepClassRow {
    pub fn of_tsv_line(line: &'static str) -> Option<Self> {
        let mut fields = line.split('\t');
        let name = fields.next()?;
        let main = hex_field(fields.next()?)?;
        let compat = hex_field(fields.next()?)?;
        let sort = StepSort::from_text(fields.next()?)?;
        let basis = Basis::from_text(fields.next()?)?;
        let reason = fields.next()?;
        if fields.next().is_some() || name.is_empty() || reason.is_empty() {
            return None;
        }
        Some(Self {
            name,
            vftable: GameAddress::new(main, compat),
            sort,
            basis,
            reason,
        })
    }
}

fn hex_field(text: &str) -> Option<u32> {
    u32::from_str_radix(text.strip_prefix("0x")?, 16).ok()
}
