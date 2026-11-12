use automata_game::address::GameAddress;
use automata_game::exe::ExeVariant;
use nc_hook::build_gate::VerifiedBuild;

#[derive(Debug, Clone, Copy)]
pub struct GameCalls {
    pub(super) build: VerifiedBuild,
    pub(super) variant: ExeVariant,
}

impl GameCalls {
    pub fn new(build: VerifiedBuild, variant: ExeVariant) -> Self {
        Self { build, variant }
    }

    pub fn variant(&self) -> ExeVariant {
        self.variant
    }

    pub(super) fn address_of(&self, address: GameAddress) -> usize {
        self.build.module_base() as usize + address.rva(self.variant) as usize
    }
}
