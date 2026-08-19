//! shared — the types that belong to nobody.
//!
//! **This is the only domain without a plugin.** What lives here is what several domains
//! need without any one of them having to know the others: ids, the input channel, the
//! messages, the math helpers, the launch flags.
//!
//! Why the messages live here and not with the sender: `combat` sends [`TitanHit`], `titan`
//! reads it. If the type lived in `combat`, `titan` would need an edge to `combat` — and the
//! domain rule would be empty within a week (`docs/architecture.md`).
//!
//! Why [`Gas`] and [`Blades`] live here too, although `vector` and `blades` write them:
//! because `hud` and `sound` have to **read** them. Who writes stands in the authority table
//! in `docs/architecture.md` — not in the type.

pub mod schedule;
pub mod geometry;
pub mod gear;
pub mod ids;
pub mod intent;
pub mod math;
pub mod message;
pub mod spatial;
pub mod rope;
pub mod cli;
pub mod rng;
pub mod state;
pub mod layers;
pub mod anchors;
pub mod terrain;
pub mod settings;

pub use schedule::{IntentSystems, SimulationSystems, Tick};
pub use geometry::{player_aabb, AnchorSurface, Block, Ground, Body, BodyMask};
pub use gear::{
    ReelSpeed, RunAccel, BoostAccel, GasGrant, Hook, HookArm, HookState,
    RopeLength, Side, PrevButtons, AimPoint, ArmAim,
};
pub use ids::{IdCounter, BodyId, LocalPlayer, PlayerId, TitanId, TitanKindName};
pub use intent::{LookOverride, Intent, Buttons};
pub use message::{
    Impact, HookReleased, HookAnchored, BodyGone, HitZone, ReleaseReason, MissReason, Mark,
    BladeRestockRequest, RefuelRequest, WarpPlayer, TitanHit, SpawnTitan,
    AbandonSortie, DeployRequest,
};
pub use settings::PlayerSettings;
pub use spatial::{IndexEntry, SpatialIndex, RayResult, RayHit};
pub use rope::{rope_reel_in, rope_step, RopeConstraint, ConstraintResult};
pub use cli::Cli;
pub use rng::Rng;
pub use state::{MovementState, Gas, Blades, Velocity, Health, HitStop, TitanState, StateClock};
pub use layers::{
    GameLayer, LAYER_WORLD, LAYER_PLAYER, LAYER_TITAN_BODY, LAYER_TITAN_CORTEX,
};
pub use anchors::{
    is_anchor_name, ModelAnchors, ModelName, ANCHOR_NAMES, CORTEX_ANCHOR, HOOK_PREFIX,
};
pub use terrain::TerrainField;
