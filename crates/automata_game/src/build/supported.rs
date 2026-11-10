use nc_hook::build_gate::PeIdentity;

use crate::exe::ExeVariant;

pub const MAIN_PE_IDENTITY: PeIdentity = PeIdentity {
    time_date_stamp: 0x6AC8_9519,
    size_of_image: 0x018B_E000,
};

pub const COMPAT_PE_IDENTITY: PeIdentity = PeIdentity {
    time_date_stamp: 0x6AC8_9501,
    size_of_image: 0x018B_0000,
};

pub const SUPPORTED_PE_IDENTITIES: [PeIdentity; 2] = [MAIN_PE_IDENTITY, COMPAT_PE_IDENTITY];

pub fn variant_of_image_size(size_of_image: u32) -> Option<ExeVariant> {
    if size_of_image == MAIN_PE_IDENTITY.size_of_image {
        Some(ExeVariant::Main)
    } else if size_of_image == COMPAT_PE_IDENTITY.size_of_image {
        Some(ExeVariant::Compat)
    } else {
        None
    }
}
