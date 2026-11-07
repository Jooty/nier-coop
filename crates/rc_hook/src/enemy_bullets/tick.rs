use crate::build_gate::VerifiedBuild;
use crate::enemies::{EnemySyncRole, shared_enemy_actors};
use crate::enemy_bullets::send::send_enemy_bullets;
use crate::enemy_bullets::show::show_enemy_bullets;
use crate::net;

/// Runs once per script tick on the game thread: the host sends its enemies' bullets, the guest
/// draws the ones that arrive.
pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let shared = shared_enemy_actors();
    let in_play = net.view().partner_in_play();
    let hosting = in_play && shared.role == Some(EnemySyncRole::Host);
    send_enemy_bullets(build, net, hosting.then_some(&shared.actors));
    show_enemy_bullets(build, net, in_play);
}
