use nc_hook::build_gate::{PE_HEADER_READ_LEN, PeError, PeIdentity};

const SUPPORTED_TIME_DATE_STAMP: u32 = 0x1111_0000;
const SUPPORTED_SIZE_OF_IMAGE: u32 = 0x0100_0000;
const SUPPORTED_PE_IDENTITY: PeIdentity = PeIdentity {
    time_date_stamp: SUPPORTED_TIME_DATE_STAMP,
    size_of_image: SUPPORTED_SIZE_OF_IMAGE,
};
const SECOND_PE_IDENTITY: PeIdentity = PeIdentity {
    time_date_stamp: 0x2222_0000,
    size_of_image: 0x0200_0000,
};
const SUPPORTED_TABLE: [PeIdentity; 2] = [SUPPORTED_PE_IDENTITY, SECOND_PE_IDENTITY];

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
        PeIdentity::check_supported(&header, &SUPPORTED_TABLE),
        Ok(SUPPORTED_PE_IDENTITY)
    );
}

#[test]
fn a_different_time_stamp_is_refused() {
    let header = header_with(SUPPORTED_TIME_DATE_STAMP + 1, SUPPORTED_SIZE_OF_IMAGE);
    let found = PeIdentity {
        time_date_stamp: SUPPORTED_TIME_DATE_STAMP + 1,
        size_of_image: SUPPORTED_SIZE_OF_IMAGE,
    };
    assert_eq!(
        PeIdentity::check_supported(&header, &SUPPORTED_TABLE),
        Err(PeError::UnsupportedBuild { found })
    );
}

#[test]
fn a_different_image_size_is_refused() {
    let header = header_with(SUPPORTED_TIME_DATE_STAMP, SUPPORTED_SIZE_OF_IMAGE + 0x1000);
    assert!(matches!(
        PeIdentity::check_supported(&header, &SUPPORTED_TABLE),
        Err(PeError::UnsupportedBuild { .. })
    ));
}

#[test]
fn garbage_headers_are_refused() {
    assert_eq!(PeIdentity::parse(b"ELF"), Err(PeError::NotDosImage));
    assert_eq!(PeIdentity::parse(b"MZ"), Err(PeError::Truncated));
    let mut header = header_with(0, 0);
    header[PE_OFFSET] = b'X';
    assert_eq!(PeIdentity::parse(&header), Err(PeError::NotPeImage));
}

#[test]
fn any_row_of_the_table_passes_the_gate() {
    let header = header_with(
        SECOND_PE_IDENTITY.time_date_stamp,
        SECOND_PE_IDENTITY.size_of_image,
    );
    assert_eq!(
        PeIdentity::check_supported(&header, &SUPPORTED_TABLE),
        Ok(SECOND_PE_IDENTITY)
    );
}

#[test]
fn an_empty_table_refuses_every_build() {
    let header = header_with(SUPPORTED_TIME_DATE_STAMP, SUPPORTED_SIZE_OF_IMAGE);
    assert!(matches!(
        PeIdentity::check_supported(&header, &[]),
        Err(PeError::UnsupportedBuild { .. })
    ));
}
