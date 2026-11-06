use data_encoding::BASE32_NOPAD;
use iroh::EndpointAddr;

use crate::iroh_adapter::INVITE_PREFIX;
use crate::transport::{Invite, MAX_INVITE_BYTES, NetError};

pub(super) fn encode_invite(addr: &EndpointAddr) -> Result<Invite, NetError> {
    let mut trimmed = EndpointAddr::new(addr.id);
    if let Some(relay) = addr.relay_urls().next() {
        trimmed = trimmed.with_relay_url(relay.clone());
    }
    let mut text = render(&trimmed)?;
    for ip in addr.ip_addrs() {
        let candidate = trimmed.clone().with_ip_addr(*ip);
        match render(&candidate) {
            Ok(longer) => {
                trimmed = candidate;
                text = longer;
            }
            Err(_) => break,
        }
    }
    Invite::new(text)
}

pub(super) fn decode_invite(invite: &Invite) -> Result<EndpointAddr, NetError> {
    let body = invite
        .as_str()
        .strip_prefix(INVITE_PREFIX)
        .ok_or(NetError::InvalidInvite)?;
    let raw = BASE32_NOPAD
        .decode(body.to_ascii_uppercase().as_bytes())
        .map_err(|_| NetError::InvalidInvite)?;
    postcard::from_bytes::<EndpointAddr>(&raw).map_err(|_| NetError::InvalidInvite)
}

fn render(addr: &EndpointAddr) -> Result<String, NetError> {
    let raw = postcard::to_stdvec(addr).map_err(|_| NetError::InvalidInvite)?;
    let text = format!(
        "{INVITE_PREFIX}{}",
        BASE32_NOPAD.encode(&raw).to_ascii_lowercase()
    );
    if text.len() > MAX_INVITE_BYTES {
        return Err(NetError::InvalidInvite);
    }
    Ok(text)
}
