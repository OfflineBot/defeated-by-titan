//! Der Transport `LocalOnly`: Tastatur und Maus werden zu einem [`Intent`].
//!
//! **Das hier ist die einzige Stelle im Spiel, die eine Taste kennt.** Alles dahinter liest
//! nur noch das `Intent` — und weiss deshalb nicht, ob gerade ein Mensch, ein Skript oder
//! eines Tages das Netz spielt (`prompts/init.md` §6 Regel 2).
//!
//! ⚠️ Die Belegung steht vorerst **hier im Code**, nicht in einer RON. Sie ist keine
//! Balance-Zahl, sondern eine Oberflaechen-Einstellung, und **freie Tastenbelegung ist eine
//! Anforderung der Bibel** (3.5, Barrierefreiheit) — sie wandert in die Optionen, wenn
//! `menu/` sie bekommt. Bis dahin ist sie eine Vorgabe, kein Design.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use super::Posteingang;
use crate::data::GameData;
use crate::shared::{BlickVorgabe, Intent, LocalPlayer, PlayerId, Tasten, Tick};

/// Liest die echten Eingaben und wirft daraus ein [`Intent`] in den Posteingang.
///
/// Laeuft in `FixedPreUpdate`, im Set `EingabeSet::Sammeln` — also **pro Simulationstick**
/// und garantiert **nach** dem Skript-Fahrer (`EingabeSet::Quelle`). Zwischen zwei Ticks
/// gesammelte Mausbewegung geht dabei nicht verloren: `AccumulatedMouseMotion` addiert sie
/// ueber die Bilder auf.
pub fn tastatur_lesen(
    tasten: Res<ButtonInput<KeyCode>>,
    maustasten: Res<ButtonInput<MouseButton>>,
    mausbewegung: Res<AccumulatedMouseMotion>,
    tick: Res<Tick>,
    daten: Res<GameData>,
    mut post: ResMut<Posteingang>,
    mut vorgabe: ResMut<BlickVorgabe>,
    mut blick: Local<Blick>,
    lokal: Query<&PlayerId, With<LocalPlayer>>,
) {
    // Es gibt keinen „den Spieler" — aber es gibt genau einen, der ICH ist. Gibt es ihn
    // (noch) nicht, ist das kein Fehler: die Welt wird gerade erst aufgebaut.
    let Some(ich) = lokal.iter().next().copied() else {
        return;
    };

    let k = &daten.spiel.kamera;
    if let Some((yaw, pitch)) = vorgabe.0.take() {
        // Der „so-tun-als\"-Blickvektor des Skript-Fahrers (§12b). Eine Maus kennt keinen
        // absoluten Winkel — `look 0 -10` schon, und genau das braucht eine reproduzierbare
        // Fahrt.
        blick.yaw = yaw;
        blick.pitch = pitch;
    } else {
        let d = mausbewegung.delta;
        blick.yaw -= d.x * k.maus_grad_pro_punkt.to_radians();
        blick.pitch = (blick.pitch - d.y * k.maus_grad_pro_punkt.to_radians())
            .clamp(-k.pitch_grenze_grad.to_radians(), k.pitch_grenze_grad.to_radians());
    }

    let mut t = Tasten::KEINE;
    t.setzen(Tasten::SPRINGEN, tasten.pressed(KeyCode::Space));
    t.setzen(Tasten::BOOST, tasten.pressed(KeyCode::ShiftLeft));
    t.setzen(Tasten::EINHOLEN, tasten.pressed(KeyCode::ControlLeft));
    t.setzen(Tasten::AUSWEICHEN, tasten.pressed(KeyCode::KeyC));
    t.setzen(Tasten::MARKIEREN, tasten.pressed(KeyCode::KeyQ));
    t.setzen(Tasten::HAKEN_LINKS, maustasten.pressed(MouseButton::Left));
    t.setzen(Tasten::HAKEN_RECHTS, maustasten.pressed(MouseButton::Right));
    t.setzen(Tasten::SCHNITT_LINKS, tasten.pressed(KeyCode::KeyF));
    t.setzen(Tasten::SCHNITT_RECHTS, tasten.pressed(KeyCode::KeyE));

    let vor = f32::from(tasten.pressed(KeyCode::KeyW)) - f32::from(tasten.pressed(KeyCode::KeyS));
    let seit = f32::from(tasten.pressed(KeyCode::KeyD)) - f32::from(tasten.pressed(KeyCode::KeyA));

    post.einwerfen(
        ich,
        Intent {
            bewegen_x: seit,
            bewegen_y: vor,
            yaw: blick.yaw,
            pitch: blick.pitch,
            tasten: t,
            tick: tick.0,
        },
        tick.0,
    );
}

/// Der Blick lebt zwischen den Bildern — er ist die aufsummierte Mausbewegung, nicht ihr
/// Delta. Als `Local` und nicht als `Resource`, damit klar bleibt: das gehoert **diesem**
/// System und niemandem sonst.
#[derive(Default)]
pub struct Blick {
    pub yaw: f32,
    pub pitch: f32,
}
