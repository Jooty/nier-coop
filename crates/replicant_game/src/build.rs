mod supported;

pub use nc_hook::build_gate::{PE_HEADER_READ_LEN, PeError, PeIdentity};
pub use supported::{
    SUPPORTED_PE_IDENTITIES, SUPPORTED_PE_IDENTITY, SUPPORTED_SIZE_OF_IMAGE,
    SUPPORTED_TIME_DATE_STAMP,
};
