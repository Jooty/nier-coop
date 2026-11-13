use nc_wire::SessionRules;

/// Why START stays unavailable when the party is larger than the picked mode takes, or `None` when it fits.
pub fn party_misfit_note(party: usize, rules: SessionRules) -> Option<String> {
    let most = usize::from(rules.max_players);
    (party > most).then(|| format!("This mode takes {most} players; {party} are in"))
}
