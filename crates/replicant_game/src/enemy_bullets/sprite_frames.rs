use crate::enemy_bullets::BULLET_SPRITE_FRAME_COUNT;

/// The game starts each sprite bullet on a random frame of its loop; the guest's copy takes one
/// from the bullet's instance so a volley does not pulse as one.
pub fn first_sprite_frame(instance: u16) -> u16 {
    instance % BULLET_SPRITE_FRAME_COUNT
}

pub fn next_sprite_frame(frame: u16) -> u16 {
    (frame + 1) % BULLET_SPRITE_FRAME_COUNT
}
