//! hud — gas, blade state, target markers, crosshair
//!
//! **Reads only.** And reads the state **of the local player** through the
//! [`LocalPlayer`](crate::shared::LocalPlayer) marker — this is the one place in the code that
//! knows who "I" am.
//!
//! PC-only means: more information at once, because no thumb covers half the screen
//! (Bible 3.5).
//!
//! **Where the seam stands:** [`gas_bar`] is registered and empty. As long as it is empty the
//! gas level is nowhere on screen — and `F-018` stays 🟨 ("logic tested, pixels unseen"), no
//! matter how green its test is. A number in the terminal is not a picture.

pub mod gas_bar;

use bevy::prelude::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, gas_bar::spawn_gas_bar)
            .add_systems(Update, gas_bar::update_gas_bar);
    }
}
