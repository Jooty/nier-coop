use core::ffi::c_void;

use replicant_game::attack_gauge::{
    ADD_ATTACK_GAUGE_NODE_RVA, ATTACK_GAUGE_MANAGER_RVA, BREAK_ATTACK_GAUGE_NODE_RVA,
    CLOSE_ATTACK_GAUGE_RVA, OPEN_ATTACK_GAUGE_RVA, OPEN_TWO_ACTOR_ATTACK_GAUGE_RVA,
};
use replicant_proto::{AttackGaugeNode, AttackGaugeOpen};

use crate::build_gate::VerifiedBuild;

type AddNode =
    unsafe extern "system" fn(*mut c_void, i32, f32, i32, *const f32, i32, i32, i32) -> i32;
type Open = unsafe extern "system" fn(*mut c_void, *mut c_void, f32, i32, i32, u8, u8) -> u8;
type OpenTwoActors = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    *mut c_void,
    f32,
    i32,
    i32,
    i32,
    u8,
    u8,
) -> u8;
type Close = unsafe extern "system" fn(*mut c_void, u8);
type BreakNode = unsafe extern "system" fn(*mut c_void, i32, *mut c_void);

#[repr(align(16))]
struct NodeOffset([f32; 4]);

/// The game's own attack gauge functions, called on the game thread with the gauge manager.
#[derive(Debug, Clone, Copy)]
pub(super) struct GaugeCalls {
    module_base: u64,
}

impl GaugeCalls {
    pub(super) fn new(build: &VerifiedBuild) -> Self {
        Self {
            module_base: build.module_base(),
        }
    }

    pub(super) fn add_node(&self, node: &AttackGaugeNode) -> i32 {
        let offset = NodeOffset(node.offset);
        #[expect(
            unsafe_code,
            reason = "calls the gauge's add-node function at its verified address"
        )]
        // SAFETY: the RVA starts add-node (manager, bone, size in xmm2, max hp, 16-byte offset
        // pointer, kind, hp, extra -> index) in the verified build, read from its call sites; the
        // offset outlives the call; game thread, inside the script manager tick.
        unsafe {
            self.function::<AddNode>(ADD_ATTACK_GAUGE_NODE_RVA)(
                self.manager(),
                node.bone,
                node.size,
                node.max_hp,
                offset.0.as_ptr(),
                node.kind,
                node.hp,
                node.extra,
            )
        }
    }

    pub(super) fn open(&self, boss: u64, partner: Option<u64>, open: &AttackGaugeOpen) -> bool {
        let [first_point, second_point, third_point] = open.map_points;
        let [first_option, second_option] = open.options;
        #[expect(
            unsafe_code,
            reason = "calls the gauge's open function at its verified address"
        )]
        // SAFETY: both RVAs start the open functions (manager, boss, seconds in xmm2, 2 map points,
        // 2 options; or manager, boss, second actor, seconds in xmm3, 3 map points, 2 options),
        // read from their call sites; the actors passed this tick's liveness check; game thread.
        let opened = unsafe {
            match partner {
                None => self.function::<Open>(OPEN_ATTACK_GAUGE_RVA)(
                    self.manager(),
                    boss as usize as *mut c_void,
                    open.total_seconds,
                    first_point,
                    second_point,
                    first_option,
                    second_option,
                ),
                Some(partner) => self.function::<OpenTwoActors>(OPEN_TWO_ACTOR_ATTACK_GAUGE_RVA)(
                    self.manager(),
                    boss as usize as *mut c_void,
                    partner as usize as *mut c_void,
                    open.total_seconds,
                    first_point,
                    second_point,
                    third_point,
                    first_option,
                    second_option,
                ),
            }
        };
        opened != 0
    }

    pub(super) fn close(&self) {
        #[expect(
            unsafe_code,
            reason = "calls the gauge's close function at its verified address"
        )]
        // SAFETY: the RVA starts the gauge's close function (manager, keep-result byte) in the
        // verified build; 0 is what the gauge's own timeout passes. Game thread.
        unsafe {
            self.function::<Close>(CLOSE_ATTACK_GAUGE_RVA)(self.manager(), 0);
        }
    }

    pub(super) fn break_node(&self, dummy: u64, node: i32, attacker: u64) {
        #[expect(
            unsafe_code,
            reason = "calls the gauge's node-break function at its verified address"
        )]
        // SAFETY: the RVA starts the function the dummy's hit function calls when a node's hp
        // drops below 1 (dummy actor, node index, attacking actor), in the verified build. The
        // dummy and the player passed the liveness check this tick; game thread.
        unsafe {
            self.function::<BreakNode>(BREAK_ATTACK_GAUGE_NODE_RVA)(
                dummy as usize as *mut c_void,
                node,
                attacker as usize as *mut c_void,
            );
        }
    }

    fn manager(&self) -> *mut c_void {
        (self.module_base + u64::from(ATTACK_GAUGE_MANAGER_RVA)) as usize as *mut c_void
    }

    #[expect(
        unsafe_code,
        reason = "turns a verified function address into a typed pointer"
    )]
    unsafe fn function<F: Copy>(&self, rva: u32) -> F {
        let address = (self.module_base + u64::from(rva)) as usize;
        // SAFETY: the caller names the RVA of a function of type F in the verified build.
        unsafe { core::mem::transmute_copy::<usize, F>(&address) }
    }
}
