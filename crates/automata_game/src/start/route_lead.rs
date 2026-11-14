use crate::character::Character;
use crate::enemies::EnemyShare;
use crate::save_redirect::SaveRole;

/// Whose character picks the route flag of a chapter start. The route decides which of the
/// phase's generators exist, so a guest that shares the host's enemies takes the host's route
/// (its `partner` is the host's character); the hero and buddy flags stay its own.
pub fn route_lead(
    role: SaveRole,
    hero: Character,
    partner: Character,
    share: EnemyShare,
) -> Character {
    if role == SaveRole::Guest && share.lists_enemies() {
        partner
    } else {
        hero
    }
}
