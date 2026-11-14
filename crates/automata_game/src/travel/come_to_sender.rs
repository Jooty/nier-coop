use crate::travel::TeleportRequest;

/// The slot of the player whose newest `ComeToSender` is among the requests, oldest first.
pub fn newest_come_sender(requests: &[(u8, TeleportRequest)]) -> Option<u8> {
    requests
        .iter()
        .rev()
        .find(|(_, request)| *request == TeleportRequest::ComeToSender)
        .map(|(sender, _)| *sender)
}

/// The body the asking player drives here, out of the bodies known by slot.
pub fn body_of_sender<T: Copy>(sender: u8, bodies: &[(u8, T)]) -> Option<T> {
    bodies
        .iter()
        .find(|(slot, _)| *slot == sender)
        .map(|(_, body)| *body)
}
