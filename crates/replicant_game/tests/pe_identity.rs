use replicant_game::build::{
    PE_HEADER_READ_LEN, PeIdentity, SUPPORTED_PE_IDENTITIES, SUPPORTED_PE_IDENTITY,
    SUPPORTED_SIZE_OF_IMAGE, SUPPORTED_TIME_DATE_STAMP,
};

const PE_OFFSET: usize = 0x150;

fn header_with(time_date_stamp: u32, size_of_image: u32) -> Vec<u8> {
    let mut header = vec![0_u8; PE_HEADER_READ_LEN];
    header[..2].copy_from_slice(b"MZ");
    header[0x3C..0x40].copy_from_slice(&(PE_OFFSET as u32).to_le_bytes());
    header[PE_OFFSET..PE_OFFSET + 4].copy_from_slice(b"PE\0\0");
    header[PE_OFFSET + 8..PE_OFFSET + 12].copy_from_slice(&time_date_stamp.to_le_bytes());
    header[PE_OFFSET + 80..PE_OFFSET + 84].copy_from_slice(&size_of_image.to_le_bytes());
    header
}

#[test]
fn the_supported_build_passes_the_gate() {
    let header = header_with(SUPPORTED_TIME_DATE_STAMP, SUPPORTED_SIZE_OF_IMAGE);
    assert_eq!(
        PeIdentity::check_supported(&header, &SUPPORTED_PE_IDENTITIES),
        Ok(SUPPORTED_PE_IDENTITY)
    );
}

#[test]
fn the_supported_values_match_the_game_exe() {
    assert_eq!(SUPPORTED_TIME_DATE_STAMP, 0x60B7_3C00);
    assert_eq!(SUPPORTED_SIZE_OF_IMAGE, 0x0533_9000);
}
