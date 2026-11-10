use automata_game::build::{
    COMPAT_PE_IDENTITY, MAIN_PE_IDENTITY, SUPPORTED_PE_IDENTITIES, variant_of_image_size,
};
use automata_game::exe::ExeVariant;

#[test]
fn the_pinned_images_are_the_supported_builds() {
    assert_eq!(MAIN_PE_IDENTITY.time_date_stamp, 0x6AC8_9519);
    assert_eq!(MAIN_PE_IDENTITY.size_of_image, 0x018B_E000);
    assert_eq!(COMPAT_PE_IDENTITY.time_date_stamp, 0x6AC8_9501);
    assert_eq!(COMPAT_PE_IDENTITY.size_of_image, 0x018B_0000);
    assert_eq!(SUPPORTED_PE_IDENTITIES.len(), 2);
}

#[test]
fn the_image_size_gives_the_variant() {
    assert_eq!(
        variant_of_image_size(MAIN_PE_IDENTITY.size_of_image),
        Some(ExeVariant::Main)
    );
    assert_eq!(
        variant_of_image_size(COMPAT_PE_IDENTITY.size_of_image),
        Some(ExeVariant::Compat)
    );
    assert_eq!(variant_of_image_size(0x1000), None);
}

#[test]
fn the_two_images_differ_in_both_timestamp_and_size() {
    assert_ne!(MAIN_PE_IDENTITY, COMPAT_PE_IDENTITY);
    assert_ne!(
        MAIN_PE_IDENTITY.time_date_stamp,
        COMPAT_PE_IDENTITY.time_date_stamp
    );
    assert_ne!(
        MAIN_PE_IDENTITY.size_of_image,
        COMPAT_PE_IDENTITY.size_of_image
    );
}
