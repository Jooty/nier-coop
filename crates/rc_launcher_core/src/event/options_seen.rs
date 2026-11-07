use rc_proto::SessionOptions;

pub fn options_differ_from_last(
    last: &mut Option<SessionOptions>,
    options: SessionOptions,
) -> bool {
    if *last == Some(options) {
        return false;
    }
    *last = Some(options);
    true
}
