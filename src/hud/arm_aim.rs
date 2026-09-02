//! **Retired element** — the per-arm landing markers (`F-026`/`F-171`) are OFF the screen
//! since 2026-09-01. What survives of this module is [`SIGHT_CORE_PX`], the constant the
//! crosshair and the catch band still measure themselves against.
//!
//! # What stood here, and why it was removed
//!
//! Two world-tracked markers — a glyph per arm (dash / ring / wide ring / filled disc), a
//! tether stub on the anchored state, and the `Q`/`E` key letters — stood on the point each
//! arm's rope would fly to, projected through the real camera. `FIND-129` measured the
//! projection to **0.0 px** against the rope's own target, and `FIND-217`/`FIND-222` spent
//! two rounds getting the drawn pixel onto the cursor. The user retired the element itself,
//! 2026-09-01 (`docs/NEXT.md` §5E-c):
//!
//! > *„die kreise können ganz weg! also in der mitte …"*
//!
//! **That supersedes his own 2026-08-19 instruction** (*„wichtig wäre nur dass diese auch
//! genau da sind visuell wo das seil auch landen würde!"*) — newest word wins, both dated in
//! `docs/FINDINGS.md` FIND-227. The X crosshair (`hud::crosshair`, his 2026-09-01 spec) is
//! now the only centre element, and `tests/hud.rs::f171_the_centre_carries_nothing_but_the_x`
//! is the inverted claim: the middle band holds crosshair nodes and NOTHING else, and no
//! `hud_arm_*` node — glyph, tether or `Q`/`E` label — exists in the tree at all.
//!
//! What went with the assembly, named rather than hidden (FIND-227 has the list):
//!
//! - the four-shape state table and its per-arm `Ready`/`Free` reading;
//! - the `F-028`/`F-029` miss words (`NO TARGET`, `TOO FAR`, …) that rode under the letters —
//!   the *messages* (`HookReleased` with `MissReason`) still fire and still reach the log;
//!   whether the word needs a new on-screen home is `docs/QUESTIONS.md` Q-095;
//! - the `DBT_AIMTRACE` motion trace (it measured the drawn glyph, which no longer exists);
//! - `B-038` (marker letters vs rebind) — closed won't-fix, no letters left to follow a rebind.
//!
//! **`vector::aim` and [`ArmAim`](crate::shared::ArmAim) are untouched**: the rope still
//! fires at the per-arm resolved target (`vector::hook` decision 6), and the crosshair still
//! senses `AimPoint`. Only the drawing is gone. The full module as it stood is in the git
//! history (`git log -- src/hud/arm_aim.rs`).

/// **The pixels the player is actually aiming at** — a 6 px half-width square around the
/// exact centre of the screen.
///
/// The one survivor of the marker module, because two other elements are specified against
/// it: [`crate::hud::crosshair`]'s `GAP_FLOOR_PX` keeps every stroke of the X out of it
/// (*„in der mitte nichts"*), and [`crate::hud::catch_band`] leaves this gap around the
/// crosshair rather than drawing through it. `F-024`'s sweep and
/// `tests/hud.rs::the_x_crosshair_hugs_the_centre_and_keeps_the_aim_pixel_free` are written
/// against this number.
pub const SIGHT_CORE_PX: f32 = 6.0;
