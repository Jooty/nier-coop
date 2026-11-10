use std::fmt;

use crate::build::SUPPORTED_PE_IDENTITY;

pub const PE_HEADER_READ_LEN: usize = 0x400;
const DOS_MAGIC: [u8; 2] = *b"MZ";
const PE_SIGNATURE: [u8; 4] = *b"PE\0\0";
const NEW_HEADER_OFFSET_FIELD: usize = 0x3C;
const TIME_DATE_STAMP_FROM_PE_SIGNATURE: usize = 8;
const SIZE_OF_IMAGE_FROM_PE_SIGNATURE: usize = 24 + 56;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeIdentity {
    pub time_date_stamp: u32,
    pub size_of_image: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeError {
    Truncated,
    NotDosImage,
    NotPeImage,
    UnsupportedBuild { found: PeIdentity },
}

impl PeIdentity {
    pub fn parse(header: &[u8]) -> Result<Self, PeError> {
        if header.get(..2) != Some(&DOS_MAGIC[..]) {
            return Err(PeError::NotDosImage);
        }
        let pe_offset = read_u32_at(header, NEW_HEADER_OFFSET_FIELD).ok_or(PeError::Truncated)?;
        let pe_offset = usize::try_from(pe_offset).map_err(|_| PeError::Truncated)?;
        let signature_end = pe_offset
            .checked_add(PE_SIGNATURE.len())
            .ok_or(PeError::Truncated)?;
        let signature = header
            .get(pe_offset..signature_end)
            .ok_or(PeError::Truncated)?;
        if signature != PE_SIGNATURE {
            return Err(PeError::NotPeImage);
        }
        let time_date_stamp = read_u32_at(header, pe_offset + TIME_DATE_STAMP_FROM_PE_SIGNATURE)
            .ok_or(PeError::Truncated)?;
        let size_of_image = read_u32_at(header, pe_offset + SIZE_OF_IMAGE_FROM_PE_SIGNATURE)
            .ok_or(PeError::Truncated)?;
        Ok(Self {
            time_date_stamp,
            size_of_image,
        })
    }

    pub fn check_supported(header: &[u8]) -> Result<Self, PeError> {
        let found = Self::parse(header)?;
        if found == SUPPORTED_PE_IDENTITY {
            Ok(found)
        } else {
            Err(PeError::UnsupportedBuild { found })
        }
    }
}

impl fmt::Display for PeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(formatter, "executable header is truncated"),
            Self::NotDosImage => write!(formatter, "image does not start with MZ"),
            Self::NotPeImage => write!(formatter, "image has no PE signature"),
            Self::UnsupportedBuild { found } => write!(
                formatter,
                "unsupported game build: TimeDateStamp {:#x}, SizeOfImage {:#x}",
                found.time_date_stamp, found.size_of_image
            ),
        }
    }
}

impl std::error::Error for PeError {}

fn read_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let field: [u8; 4] = bytes.get(offset..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(field))
}
