use rc_game::side_view::SideViewWatch;

#[test]
fn the_first_look_only_sets_the_baseline() {
    let mut watch = SideViewWatch::default();
    assert_eq!(watch.observe(true), None);
}

#[test]
fn a_flip_is_reported_once() {
    let mut watch = SideViewWatch::default();
    watch.observe(false);
    assert_eq!(watch.observe(true), Some(true));
    assert_eq!(watch.observe(true), None);
    assert_eq!(watch.observe(false), Some(false));
}
