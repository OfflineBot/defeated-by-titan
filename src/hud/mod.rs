//! hud — gas, blade state, health, objective, crosshair
//!
//! **Reads only.** And reads the state **of the local player** through the
//! [`LocalPlayer`](crate::shared::LocalPlayer) marker — this is the one place in the code that
//! knows who "I" am.
//!
//! PC-only means: more information at once, because no thumb covers half the screen
//! (Bible 3.5).
//!
//! ## Five elements, and two of them have no producer yet
//!
//! | element | file | reads | producer |
//! |---|---|---|---|
//! | gas bar, left | [`gas_bar`] | [`Gas`](crate::shared::Gas) | `vector` — **exists** |
//! | blade pips, right | [`blade_pips`] | [`Blades`](crate::shared::Blades) | `blades` — the component exists, the wear does not |
//! | health bar, bottom centre | [`health_bar`] | [`Health`](crate::shared::Health) | job R3-A — **absent today** |
//! | crosshair, centre | [`crosshair`] | [`AimPoint`](crate::shared::AimPoint) | `vector::aim` — **exists** |
//! | objective line, top centre | [`objective`] | mission state | job R3-B — **absent today** |
//!
//! **The failure this module is built against has a name: "the bar that is a picture of a
//! bar"** (`docs/PLAN-GAME.md` §8, F-170) — every element of the list present, and three of
//! them showing a hard-coded number because nothing produces the real one yet. So the rule
//! here is: an element whose producer is missing queries it as `Option` and **hides itself**.
//! An empty screen corner is honest; a full bar that means nothing is not, and it is the kind
//! of lie that survives three rounds because it photographs well.
//!
//! ## The colours are not a free choice
//!
//! Cyan, amber and crimson are the three signal colours of `docs/conventions.md` §3, and they
//! appear nowhere else in the game. They are read from `assets/data/maps.ron`'s `signals:`
//! block through [`signal`] — **never written as a literal here**. That is the whole reason
//! they were made data.
//!
//! ## The middle of the screen stays free
//!
//! No node this module spawns may intersect the central 20 % × 20 % of the screen
//! ([`KEEP_OUT_LOW_PCT`]..[`KEEP_OUT_HIGH_PCT`] on both axes). That is what makes the
//! crosshair four ticks around a hole instead of one node — `tests/hud.rs` computes every
//! element's rect out of `ComputedNode` and falls over if one of them creeps inward.
//!
//! ## The evidence
//!
//! | what | run → picture |
//! |---|---|
//! | the five elements over a populated frame, tank at 82 % | `scripts/f170-hud.txt` → `docs/images/f170-hud.png` |
//! | the three crosshair states, three crops | `scripts/f171-crosshair.txt` → `docs/images/f171-crosshair.png` |
//!
//! Both were decoded, not assumed: against a control run with this plugin switched off, the
//! HUD accounts for 6 368 changed pixels in `f170-hud.png`, 4 500 of them exactly the cyan out
//! of `maps.ron` (sRGB 63, 237, 249) — and the control has **zero**. The gas bar measures
//! 227 px of a 277.6 px track = 81.8 %, next to an F3 overlay reading `gas 82/100` in the same
//! frame. The three crosshair crops measure 302 × 178, 326 × 202 and 356 × 212 px — the same
//! three tuples `tests/hud.rs` asserts, to the pixel.

pub mod blade_pips;
pub mod crosshair;
pub mod gas_bar;
pub mod health_bar;
pub mod objective;

use bevy::prelude::*;

use crate::data::GameData;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                gas_bar::spawn_gas_bar,
                blade_pips::spawn_blade_pips,
                health_bar::spawn_health_bar,
                objective::spawn_objective,
                crosshair::spawn_crosshair,
            ),
        )
        .add_systems(
            Update,
            (
                gas_bar::update_gas_bar,
                blade_pips::update_blade_pips,
                health_bar::update_health_bar,
                objective::update_objective,
                // Sense, then shape, then paint — **three systems on purpose**: the shape is
                // then testable against a state somebody set by hand, without a titan, a
                // physics world and a look direction having to be arranged first, and the
                // colour can be neutralised in the test without touching the geometry.
                (
                    crosshair::sense_crosshair,
                    crosshair::shape_crosshair,
                    crosshair::paint_crosshair,
                )
                    .chain(),
            ),
        );
    }
}

/// On **every** node this domain spawns — containers included.
///
/// `tests/hud.rs` walks exactly this marker to compute the keep-out rectangle. A container
/// without it would be a hole in that test, so the rule is: whatever gets a [`Node`] here
/// gets a `HudElement`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HudElement;

/// The forbidden box in the middle: 20 % of the width by 20 % of the height.
pub const KEEP_OUT_PCT: f32 = 20.0;
/// Left/top edge of the forbidden box, in percent of the screen.
pub const KEEP_OUT_LOW_PCT: f32 = 50.0 - KEEP_OUT_PCT * 0.5;
/// Right/bottom edge of the forbidden box, in percent of the screen.
pub const KEEP_OUT_HIGH_PCT: f32 = 50.0 + KEEP_OUT_PCT * 0.5;

/// A signal colour out of `assets/data/maps.ron`, `signals:`.
///
/// **Panics on a missing key, deliberately** — the same choice `titan::rig` makes for the
/// cortex. A grey stand-in for "amber" would put a HUD on screen that quietly stops obeying
/// `docs/conventions.md` §3, and nothing would ever say so. `serde(default)` is forbidden for
/// game values for the same reason (`CLAUDE.md` rule 2).
///
/// The triples are **linear** RGB, as everywhere else in `maps.ron` — hence
/// `Color::linear_rgb` and not `srgb`. The two differ by a factor of about two in the
/// mid-tones, which is exactly enough to make a cyan look like a different cyan next to the
/// gizmos.
pub fn signal(data: &GameData, name: &str) -> Color {
    let (r, g, b) = data.maps.signals.get(name).copied().unwrap_or_else(|| {
        panic!(
            "maps.ron `signals:` has no key {name:?} — there is nothing to paint the HUD \
             with. The three keys are cyan, amber, crimson (docs/conventions.md §3)"
        )
    });
    Color::linear_rgb(r, g, b)
}

/// The dark plate a bar sits on, and the empty half of every bar.
///
/// **Not a signal colour and deliberately not from `signals:`** — it is the same black-with-
/// alpha the F3 overlay uses (`debug::spawn_overlay`), and its only job is to keep a cyan bar
/// readable over sky as well as over asphalt. Putting it in `maps.ron` would suggest it is a
/// balancing value; it is a legibility constant.
pub const PLATE: Color = Color::srgba(0.0, 0.0, 0.0, 0.65);
