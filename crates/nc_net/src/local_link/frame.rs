use std::io::{self, Read, Write};
use std::time::Duration;

use replicant_proto::BuildGate;

use crate::local_link::LINK_VERSION;
use crate::transport::{Channel, MAX_PAYLOAD_BYTES};

pub(super) const MAGIC: [u8; 4] = *b"RCLK";
pub(super) const HELLO_BYTES: usize = 14;
pub(super) const ACK_OK: u8 = 0;
pub(super) const ACK_BUSY: u8 = 1;
pub(super) const ACK_BUILD_MISMATCH: u8 = 2;
pub(super) const ACK_BAD_HELLO: u8 = 3;
pub(super) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) fn write_hello(out: &mut impl Write, build: BuildGate) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(HELLO_BYTES);
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&LINK_VERSION.to_be_bytes());
    bytes.extend_from_slice(&build.time_date_stamp.to_be_bytes());
    bytes.extend_from_slice(&build.size_of_image.to_be_bytes());
    out.write_all(&bytes)
}

pub(super) fn read_hello(input: &mut impl Read) -> io::Result<Option<BuildGate>> {
    let mut bytes = [0u8; HELLO_BYTES];
    input.read_exact(&mut bytes)?;
    let version = u16::from_be_bytes([bytes[4], bytes[5]]);
    if bytes[..4] != MAGIC || version != LINK_VERSION {
        return Ok(None);
    }
    Ok(Some(BuildGate {
        time_date_stamp: u32::from_be_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
        size_of_image: u32::from_be_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]),
    }))
}

pub(super) fn write_frame(
    out: &mut impl Write,
    channel: Channel,
    payload: &[u8],
) -> io::Result<()> {
    let len = u16::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "payload too large"))?;
    let mut frame = Vec::with_capacity(3 + payload.len());
    frame.push(match channel {
        Channel::UnreliableLatest => 0,
        Channel::ReliableOrdered => 1,
    });
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    out.write_all(&frame)
}

pub(super) fn read_frame(input: &mut impl Read) -> io::Result<(Channel, Vec<u8>)> {
    let mut header = [0u8; 3];
    input.read_exact(&mut header)?;
    let channel = if header[0] == 0 {
        Channel::UnreliableLatest
    } else {
        Channel::ReliableOrdered
    };
    let len = usize::from(u16::from_be_bytes([header[1], header[2]]));
    if len > MAX_PAYLOAD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "oversized frame",
        ));
    }
    let mut payload = vec![0u8; len];
    input.read_exact(&mut payload)?;
    Ok((channel, payload))
}
