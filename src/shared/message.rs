//! The messages between the domains.
//!
//! **Communication runs over components and messages, not over calls.** `combat` sends
//! [`TitanHit`]; `titan` reads it and decides what that means for its body. `combat` does not
//! know how a titan is built (`prompts/init.md` §5 rule 3).
//!
//! They live **here** and not with the sender, because otherwise every receiver would need
//! an edge to the sender and the domain rule would be empty within a week.
//!
//! And they are designed to **fit over a wire**: data, no handles, no function pointers,
//! **no `Entity`** (§6 rules 7 and 8).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::gear::Side;
use super::ids::{BodyId, PlayerId, TitanId};

/// Which part of a titan was hit.
///
/// **The Cortex is the only truth**: a Cortex hit kills, no matter how full the titan is.
/// Everything else is preparation — legs off means he falls; arms off means he cannot grab;
/// eyes means he does not see you.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HitZone {
    Cortex,
    Head,
    Eye,
    ArmLeft,
    ArmRight,
    LegLeft,
    LegRight,
    Torso,
}

/// A blade has hit a titan.
///
/// `speed_m_s` is the reason this is a message and not a call: **damage comes out of speed.**
/// A cut from standing still scratches, the same cut at 30 m/s kills — and the formula for
/// that stands in the RON, not in the code (`prompts/init.md` §1, §4).
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TitanHit {
    pub titan: TitanId,
    pub by: PlayerId,
    pub zone: HitZone,
    pub speed_m_s: f32,
}

/// Please create a titan. Comes from `mission` (spawn waves) or from the `--script` driver
/// (`spawn titan husk 20 0 -40`).
///
/// `kind` is the **logical name** from `assets/data/titan.ron`, not a file name and not a
/// Rust type — otherwise a new titan would need a rebuild (§4).
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub struct SpawnTitan {
    pub kind: String,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

impl SpawnTitan {
    pub fn pos(&self) -> Vec3 {
        Vec3::new(self.pos_x, self.pos_y, self.pos_z)
    }
}

/// Put a player at a coordinate (`warp x y z` in a script, the F3 overlay).
///
/// It lets the user send a coordinate and you stand exactly there — that is worth more than
/// any bug form (§12c).
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct WarpPlayer {
    pub player: PlayerId,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

/// **Put gas back into a player's tank** — a refuel station asking, never a station writing.
///
/// `Gas` has exactly one writer, `vector::gas` (`docs/architecture.md`, authority table). A
/// refuel station is hub furniture, i.e. `mission`, so it cannot fill a tank itself without
/// becoming a second authority on one field — which is a rule-4 violation everywhere and, over
/// a wire, two machines disagreeing about how much fuel a player has (`FINDINGS.md` FIND-063).
/// So the station sends this and `vector::gas::apply_refuel_requests` is the only thing that
/// ever touches the tank.
///
/// `amount` is **one tick's worth of gas, already multiplied by `dt`** — the sender holds the
/// rate (`gear.ron: resupply.gas_per_s`, copied onto the station at spawn) and the receiver
/// holds the tank. Capped at `Gas::max` by [`Gas::refill`](super::state::Gas::refill), so a
/// station may keep asking for a full tank without anything happening.
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefuelRequest {
    pub player: PlayerId,
    pub amount: f32,
}

/// **Hand a player blades back at a rack** — the same seam as [`RefuelRequest`], for the other
/// half of the supply.
///
/// [`Blades`](super::state::Blades) has exactly one writer, `blades` (`docs/architecture.md`,
/// authority table). A rack is hub furniture, i.e. `mission`, so it cannot restock a harness
/// itself without becoming a second authority on one field — the mistake `Gas` cost a repair for
/// on 2026-08-12 (`FINDINGS.md` FIND-063). The rack sends this and
/// `blades::resupply::apply_restock_requests` is the only thing that ever calls
/// [`restock`](crate::blades::resupply::restock).
///
/// **It carries `seconds`, not an amount — and that is the one place it differs from
/// [`RefuelRequest`].** Gas is one scalar with one rate, so the station can multiply by `dt`
/// itself and the receiver needs to know nothing. A harness is three numbers
/// (`gear.ron: resupply.blade_pairs_per_s`, `sharpen_per_s`, and the `blades.start_pairs` cap)
/// plus an integer accumulator, because `Blades::pairs_left` is a `u8` and 1.5 pairs/s at 60 Hz
/// is 0.025 of a pair per tick. Putting that arithmetic in the sender would move `blades`'
/// tuning into `mission`'s hands, which is the authority violation wearing a different hat. So
/// the rack sends the one thing it actually knows — **how long the player stood there** — and
/// the owning domain does the rest.
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BladeRestockRequest {
    pub player: PlayerId,
    /// One tick's worth of standing at the rack, in seconds.
    pub seconds: f32,
}

/// One line in the log to line a screenshot up against (`mark anchored`).
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub struct Mark {
    pub text: String,
    pub tick: u64,
}

/// A hook has anchored (`F-001`).
///
/// Bare `f32` for the point, same as in [`WarpPlayer`]: this type goes over a wire one day.
/// `body` instead of a world position, so that the receiver knows **what** the hook hangs on.
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HookAnchored {
    pub player: PlayerId,
    pub side: Side,
    pub body: BodyId,
    pub point_x: f32,
    pub point_y: f32,
    pub point_z: f32,
    pub tick: u64,
}

impl HookAnchored {
    pub fn point(&self) -> Vec3 {
        Vec3::new(self.point_x, self.point_y, self.point_z)
    }
}

/// Why a hook let go.
///
/// The reason is not log prose: `hud` and `sound` tell from it whether the player let go
/// himself or whether the rope tore.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReleaseReason {
    /// Button released.
    Released,
    /// The rope had to be paid out beyond `vector.hook_range_m`, because a wall won.
    Overextended,
    /// The ray hit nothing anchorable — the tip comes back empty.
    NoAnchor,
    /// **The carrier is gone** (`F-029`: "releases with feedback when the titan dies";
    /// `T-020`: an unloaded area).
    BodyGone,
}

/// A hook has let go (`F-001`).
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HookReleased {
    pub player: PlayerId,
    pub side: Side,
    pub reason: ReleaseReason,
    pub tick: u64,
}

/// A player has driven into geometry.
///
/// `speed_before_m_s` is the number `F-013` computes the stagger from — the same logic as in
/// [`TitanHit`]: **the effect comes out of the speed**, and the formula stands in the RON.
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Impact {
    pub player: PlayerId,
    pub speed_before_m_s: f32,
    pub speed_after_m_s: f32,
    pub normal_x: f32,
    pub normal_y: f32,
    pub normal_z: f32,
    pub tick: u64,
}

impl Impact {
    pub fn normal(&self) -> Vec3 {
        Vec3::new(self.normal_x, self.normal_y, self.normal_z)
    }
}

/// A body has vanished from the spatial index. **Whoever hangs on it has to let go.**
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyGone {
    pub body: BodyId,
    pub tick: u64,
}

/// **Fly this sortie** — the lobby screen asking, never the lobby screen deciding.
///
/// `mission` is the one writer of the mission phase and of `mission::Sortie`
/// (`docs/architecture.md`, authority table). `menu` may **read** the phase — it has to know
/// whether to offer *Abandon* or *Deploy*, and a UI has to be right in the frame it is drawn in
/// — but a menu that set `NextState<MissionPhase>` itself would be a second writer of the one
/// field the whole run hangs on, next to `mission::hub::deploy_on_contact`. So the front door
/// asks and the domain that owns the phase opens it, exactly as a refuel station asks for gas
/// ([`RefuelRequest`]).
///
/// `difficulty` is `None` for a template that has none — the tutorial — and that is the same
/// meaning `mission::SortieOrder::difficulty` gives it: fly the template's own numbers.
///
/// ⚠️ **Read in `Update`, not in `FixedUpdate`.** A menu screen stops `Time<Virtual>`
/// (`menu::apply_screen`), and a stopped virtual clock means `FixedUpdate` never runs
/// (`bevy_time-0.19.0/src/fixed.rs:244-247`) — a reader in the simulation would therefore never
/// see the one message the player pressed a button to send.
#[derive(Message, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployRequest {
    /// A key in `missions.ron: templates`.
    pub template: String,
    /// A key in that template's `difficulties`, or `None` for the direct-entry numbers.
    pub difficulty: Option<String>,
}

/// **Give this sortie up** — the pause screen's *Abandon*, and the first half of its *Quit to
/// lobby*.
///
/// Same seam and same reason as [`DeployRequest`]: `mission` owns the phase, `menu` owns the
/// button. It carries nothing, because there is exactly one sortie (`mission::hub::Sortie`) and
/// giving it up is not a thing one player does for himself.
///
/// **It is not a loss.** `MissionPhase::Lost` is a verdict the mission speaks, with a debrief
/// and a `KillTally` behind it; abandoning skips straight back to the hub and writes no verdict
/// at all — nobody died, nobody won, the run simply did not happen.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbandonSortie;
