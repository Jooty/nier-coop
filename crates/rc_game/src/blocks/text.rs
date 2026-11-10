use crate::blocks::{KnownBlock, StateDiff};

pub fn block_known_text(block: &KnownBlock, position: [f32; 3]) -> String {
    format!(
        "blocks: block {} is actor {:#x}, first seen in cell {} at {}",
        block.id,
        block.actor,
        cell_text(block.start_cell),
        point_text(position)
    )
}

pub fn held_block_text(id: u16, chosen_by: &str, position: [f32; 3]) -> String {
    format!(
        "blocks: holding block {id} ({chosen_by}) at {}",
        point_text(position)
    )
}

pub fn block_sent_text(id: u16, position: [f32; 3]) -> String {
    format!("blocks: sent block {id} moved to {}", point_text(position))
}

pub fn block_moved_text(id: u16, wanted: [f32; 3], now_at: Option<[f32; 3]>) -> String {
    let now_at = now_at.map_or_else(|| "unreadable".to_owned(), point_text);
    format!(
        "blocks: moved block {id} to {} (now at {now_at})",
        point_text(wanted)
    )
}

pub fn block_glide_start_text(id: u16, from: Option<[f32; 3]>, wanted: [f32; 3]) -> String {
    let from = from.map_or_else(|| "unreadable".to_owned(), point_text);
    format!(
        "blocks: sliding block {id} from {from} toward {}",
        point_text(wanted)
    )
}

pub fn block_landed_text(id: u16, wanted: [f32; 3], now_at: Option<[f32; 3]>) -> String {
    let now_at = now_at.map_or_else(|| "unreadable".to_owned(), point_text);
    format!(
        "blocks: block {id} landed at {} (now at {now_at})",
        point_text(wanted)
    )
}

pub fn block_read_back_text(id: u16, wanted: [f32; 3], now_at: Option<[f32; 3]>) -> String {
    let now_at = now_at.map_or_else(|| "unreadable".to_owned(), point_text);
    format!(
        "blocks: block {id} read back later: wanted {}, now at {now_at}",
        point_text(wanted)
    )
}

pub fn block_missing_text(id: u16, known: usize) -> String {
    format!("blocks: no block {id} here ({known} known), left alone")
}

pub fn other_map_text(id: u16) -> String {
    format!("blocks: block {id} moved in another map, left alone")
}

pub fn state_diff_text(id: u16, diff: &StateDiff) -> Vec<String> {
    let header = format!(
        "blocks: state diff block {id} {} -> {}: {} changed words",
        diff.from_stage,
        diff.to_stage,
        diff.changes.len()
    );
    let mut lines = vec![header];
    lines.extend(diff.changes.iter().map(|change| {
        format!(
            "blocks: state diff +{:#x} {:08x} -> {:08x} (block {id}, {} -> {})",
            change.offset, change.before, change.after, diff.from_stage, diff.to_stage
        )
    }));
    lines
}

fn point_text(point: [f32; 3]) -> String {
    format!("({:.3} {:.3} {:.3})", point[0], point[1], point[2])
}

fn cell_text(cell: [i32; 3]) -> String {
    format!("({} {} {})", cell[0], cell[1], cell[2])
}
