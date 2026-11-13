use std::io;
use std::path::Path;

use crate::saves::SlotSummary;
use crate::saves::campaigns::{
    CampaignRecord, campaigns_path_beside, read_campaigns, write_campaigns,
};
use crate::saves::seats::{remove_seat, seats_dir_beside};

const PLAY_SECONDS_TOLERANCE: f64 = 0.5;

pub fn drop_campaign(coop_file: &Path, host_slot: u8) -> io::Result<bool> {
    let record_path = campaigns_path_beside(coop_file);
    let mut record = read_campaigns(&record_path);
    let had_entry = record.remove(&host_slot).is_some();
    if had_entry {
        write_campaigns(&record_path, &record)?;
    }
    remove_seat(&seats_dir_beside(coop_file), host_slot)?;
    Ok(had_entry)
}

pub fn host_slots_that_lost_their_campaign(
    record: &CampaignRecord,
    slots: &[SlotSummary],
) -> Vec<u8> {
    record
        .iter()
        .filter(|(host_slot, campaign)| {
            let slot = slots
                .iter()
                .find(|slot| slot.index == **host_slot && slot.used);
            match slot {
                None => true,
                Some(slot) => {
                    slot.name.trim() != campaign.host_name.trim()
                        || slot.play_seconds + PLAY_SECONDS_TOLERANCE < campaign.host_play_seconds
                }
            }
        })
        .map(|(host_slot, _)| *host_slot)
        .collect()
}

pub fn drop_lost_campaigns(coop_file: &Path, slots: &[SlotSummary]) -> io::Result<Vec<u8>> {
    let record = read_campaigns(&campaigns_path_beside(coop_file));
    let lost = host_slots_that_lost_their_campaign(&record, slots);
    for host_slot in &lost {
        drop_campaign(coop_file, *host_slot)?;
    }
    Ok(lost)
}
