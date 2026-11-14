use std::fmt::Debug;

use crate::launcher_game::SeatError;

pub trait GameSaves: Debug + Sync {
    fn vanilla_file_name(&self) -> &'static str;
    fn coop_file_name(&self) -> &'static str;
    fn is_coop_session_file_name(&self, name: &str) -> bool;
    fn seat_slot_bytes(&self) -> usize;
    fn check_seat(&self, seat: &[u8]) -> Result<(), SeatError>;
    fn changed_slot(&self, before: &[u8], after: &[u8]) -> Option<u8>;
    fn seat_bytes_of(&self, file: &[u8], slot: u8) -> Result<Vec<u8>, SeatError>;
    fn session_file_with_seat(&self, blank: &[u8], seat: &[u8]) -> Result<Vec<u8>, SeatError>;
    fn fix_header_checksum(&self, file: &mut [u8]) -> bool;
}
