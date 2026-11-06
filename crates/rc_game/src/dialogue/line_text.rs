use rc_proto::DialogueLine;

use crate::dialogue::talk_cursor::talk_window;
use crate::dialogue::{
    MAX_TALK_LINES, MAX_TALK_TEXT_BYTES, MAX_TALKER_NAME_BYTES, PROTAGONIST_NAME_TAG,
    TALK_LINE_COUNT_OFFSET, TALK_LINE_SPEAKER_ID_OFFSET, TALK_LINE_TEXT_OFFSET, TALK_LINES_OFFSET,
    TALK_TABLE_SLOTS, TALK_WINDOW_TALK_TABLE_OFFSET, TALKER_NAME_COUNT_OFFSET,
    TALKER_NAME_ENTRIES_OFFSET, TALKER_NAME_ENTRY_BYTES, TALKER_NAME_MAX_ENTRIES,
    TALKER_NAME_TABLE_RVA,
};
use crate::event_log::EVENT_LOG_MAX_TEXT_CHARS;
use crate::memory::{GameMemory, MemoryError};

/// One talk line as this game's own talk data holds it, tags not yet expanded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineText {
    pub speaker: Option<String>,
    pub text: String,
}

/// The text of `line` in this game's loaded talk data, or `None` when that talk or line is not
/// loaded here (the partner may be in another zone).
pub fn read_line_text<M: GameMemory>(
    memory: &M,
    line: DialogueLine,
) -> Result<Option<LineText>, MemoryError> {
    if line.talk_id >= TALK_TABLE_SLOTS {
        return Ok(None);
    }
    let slot = talk_window(memory) + TALK_WINDOW_TALK_TABLE_OFFSET + u64::from(line.talk_id) * 8;
    let talk = memory.read_u64(slot)?;
    if talk == 0 {
        return Ok(None);
    }
    let count = memory.read_i32(talk + TALK_LINE_COUNT_OFFSET)?;
    if !(0..=MAX_TALK_LINES).contains(&count) || i32::from(line.line) >= count {
        return Ok(None);
    }
    let line_address = memory.read_u64(talk + TALK_LINES_OFFSET + u64::from(line.line) * 8)?;
    if line_address == 0 {
        return Ok(None);
    }
    let text_address = memory.read_u64(line_address + TALK_LINE_TEXT_OFFSET)?;
    if text_address == 0 {
        return Ok(None);
    }
    let text = read_c_string(memory, text_address, MAX_TALK_TEXT_BYTES)?;
    let speaker_id = memory.read_i32(line_address + TALK_LINE_SPEAKER_ID_OFFSET)?;
    let speaker = read_talker_name(memory, speaker_id).ok().flatten();
    Ok(Some(LineText { speaker, text }))
}

/// Event log entries that show the partner's line: tags expanded (the protagonist tag becomes
/// the partner's name), whitespace folded, and split at word breaks to fit the log.
pub fn partner_line_events(partner_name: &str, line: &LineText) -> Vec<String> {
    let text = expand_tags(&line.text, partner_name);
    if text.is_empty() {
        return Vec::new();
    }
    let speaker = line
        .speaker
        .as_deref()
        .map(|speaker| expand_tags(speaker, partner_name))
        .filter(|speaker| !speaker.is_empty());
    let full = match speaker {
        Some(speaker) => format!("{speaker}: {text}"),
        None => text,
    };
    split_into_entries(&full, EVENT_LOG_MAX_TEXT_CHARS)
}

fn read_talker_name<M: GameMemory>(
    memory: &M,
    speaker_id: i32,
) -> Result<Option<String>, MemoryError> {
    if speaker_id < 0 {
        return Ok(None);
    }
    let table = memory.module_base() + u64::from(TALKER_NAME_TABLE_RVA);
    let count = memory.read_i32(table + TALKER_NAME_COUNT_OFFSET)?;
    for index in 0..u64::try_from(count.clamp(0, TALKER_NAME_MAX_ENTRIES)).unwrap_or(0) {
        let entry = table + TALKER_NAME_ENTRIES_OFFSET + index * TALKER_NAME_ENTRY_BYTES;
        if memory.read_i32(entry)? != speaker_id {
            continue;
        }
        let name_address = memory.read_u64(entry + 8)?;
        if name_address == 0 {
            return Ok(None);
        }
        return read_c_string(memory, name_address, MAX_TALKER_NAME_BYTES).map(Some);
    }
    Ok(None)
}

fn read_c_string<M: GameMemory>(
    memory: &M,
    address: u64,
    max_bytes: usize,
) -> Result<String, MemoryError> {
    let mut bytes = Vec::new();
    // Byte by byte: a string may end just before an unmapped page.
    for offset in 0..max_bytes as u64 {
        match memory.read_u8(address + offset)? {
            0 => break,
            byte => bytes.push(byte),
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn expand_tags(raw: &str, partner_name: &str) -> String {
    let named = raw.replace(PROTAGONIST_NAME_TAG, partner_name);
    let mut plain = String::with_capacity(named.len());
    let mut rest = named.as_str();
    while let Some(open) = rest.find('<') {
        plain.push_str(&rest[..open]);
        match rest[open..].find('>') {
            Some(close) => rest = &rest[open + close + 1..],
            None => {
                rest = "";
            }
        }
    }
    plain.push_str(rest);
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_into_entries(text: &str, max_chars: usize) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > max_chars {
            if !current.is_empty() {
                entries.push(std::mem::take(&mut current));
            }
            entries.push(word.drain(..max_chars).collect());
        }
        let word: String = word.into_iter().collect();
        let joined_len = current.chars().count() + 1 + word.chars().count();
        if !current.is_empty() && joined_len > max_chars {
            entries.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&word);
    }
    if !current.is_empty() {
        entries.push(current);
    }
    entries
}
