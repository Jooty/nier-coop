pub fn partner_shares_phase(local_phase: Option<u16>, partner_phase: u16) -> bool {
    local_phase == Some(partner_phase)
}
