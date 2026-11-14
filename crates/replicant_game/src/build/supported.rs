use nt_hook::build_gate::PeIdentity;

pub const SUPPORTED_TIME_DATE_STAMP: u32 = 0x60B7_3C00;
pub const SUPPORTED_SIZE_OF_IMAGE: u32 = 0x0533_9000;
pub const SUPPORTED_PE_IDENTITY: PeIdentity = PeIdentity {
    time_date_stamp: SUPPORTED_TIME_DATE_STAMP,
    size_of_image: SUPPORTED_SIZE_OF_IMAGE,
};
pub const SUPPORTED_PE_IDENTITIES: [PeIdentity; 1] = [SUPPORTED_PE_IDENTITY];
