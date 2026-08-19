//! **`F-043` — the one line that says a blade landed.**
//!
//! The user, after playing on 2026-08-19: *„attack fehlt aber noch (mit schwertern..)"*. The
//! round that measured it found the swing firing, the swept cast landing, [`TitanHit`] written
//! and the blade going blunt — `scripts/f032-swords.txt` produced the whole table — and **not
//! one pixel on screen changing**. `F-034`'s hit-stop and camera kick fire on a Cortex kill
//! only (`src/render/camera.rs`, *"Only the kill kicks"*), so three of the four acts of that
//! script are, to a player, indistinguishable from a swing at air.
//!
//! So this element answers exactly three questions, and nothing else:
//!
//! | question | how it is answered |
//! |---|---|
//! | did anything land at all? | the line appears. A miss writes no [`TitanHit`], so it stays empty |
//! | was it the kill? | `KILL`, amber, 30 px — against `CUT`/`GRAZE`, crimson, 22/16 px |
//! | was it a good hit or a scratch? | the closing speed, and the word chosen by `feel.strong_hit_m_s` |
//!
//! **Three signals per kind, not one** — word, size and colour all differ. That is `F-171`'s
//! rule about the arm markers applied here (*"they differ in shape not only in colour"*): a
//! player who cannot tell amber from crimson still reads three different lines, and
//! `tests/hud.rs::f043_the_three_hit_kinds_differ_in_word_size_and_colour` is what keeps that
//! true.
//!
//! ## The number is a **speed**, and that is not a placeholder for a damage number
//!
//! `F-043`'s row says *"Schwebende Zahlen"*, and `F-031` — *the* damage formula — is `⬜`. There
//! is no damage number in this game today: [`TitanHit`] carries `speed_m_s` and nothing else,
//! and `gear.ron: blades.damage_per_m_s` has no reader. Printing an invented `142` would be the
//! failure `docs/PLAN-GAME.md` §8 calls *"the bar that is a picture of a bar"* — a number that
//! photographs perfectly and means nothing.
//!
//! The closing speed is the number that **does** exist, it is the one the design says damage
//! will come out of (*"Bewegung ist Schaden"*), and it is labelled `m/s` so nobody can mistake
//! it for a damage total. When `F-031` lands, the damage goes on this line next to the speed and
//! the element does not move.
//!
//! ## Colour follows `docs/conventions.md` §3 and is not a taste decision
//!
//! Amber is *"cortex, weak points, objectives"* — so the **cortex kill is amber**. Crimson is
//! *"danger, damage, critical state"* — so a **cut into the body is crimson**. Neither is
//! chosen here; both come out of `maps.ron`'s `signals:` block through
//! [`signal`](super::signal), like every other colour in this domain.
//!
//! ## Where it stands, and why not over the titan's head
//!
//! Screen-fixed, centred, [`TOP_PCT`] above the crosshair. A number floating over the titan you
//! are cutting is what *"schwebende Zahlen"* literally asks for — and it is **forbidden by
//! `F-170`**: the keep-out box holds for *"any marker tracking a world position"*
//! (`hud::mod`), and a titan you are close enough to cut is by definition in the middle of the
//! screen. FIND-098's exemption is for the arm fan and for nothing else. So the world-space
//! variant is an open question with a rule in its way, not an oversight — see
//! `docs/FINDINGS.md`.
//!
//! ## Fully switchable off, without a settings screen
//!
//! `F-043`'s row asks for *"Vollstaendig abschaltbar"*. `gear.ron: feel.hit_mark_s` at `0.0`
//! switches the whole element off — [`step_flash`] returns [`HitFlash::default`] and the node
//! never leaves `Display::None`. That is data and not code, it needs no menu, and
//! `tests/hud.rs::f043_a_hold_of_zero_seconds_switches_the_whole_element_off` holds it down. A
//! toggle in the settings screen is the nicer half and it is **not built** — `menu` and
//! `shared::settings` belong to somebody else this round.
//!
//! ## What it costs per frame
//!
//! One message drain and two guarded writes. Outside a hit this module does nothing at all:
//! [`HitFlash`] clears to its exact default, `set_if_neq` stops writing, and change detection
//! goes quiet (`docs/lessons/performance.md` rule 1). No query over titans, no ray, no sweep.

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::GameData;
use crate::hud::{signal, HudElement};
use crate::shared::{HitZone, LocalPlayer, PlayerId, TitanHit};

/// Marker on the one text node this module owns.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HitMark;

/// The three things a landed blade can be, and they are three different lines on screen.
///
/// **A miss is not a fourth variant**, and that is deliberate: a swing that finds nothing writes
/// no [`TitanHit`] at all (`blades::cut`), so there is no message to turn into a word. The
/// feedback for a miss is the line **not** appearing, which is the only reading that cannot go
/// stale — a `MISS` word would have to be produced by guessing at the swing's end, and a guess
/// that fires on a hit whose message arrived a tick late is worse than silence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind {
    /// [`HitZone::Cortex`] — the titan is dead. The only zone that kills, at any speed.
    Kill,
    /// A body hit at or above `gear.ron: feel.strong_hit_m_s`.
    Cut,
    /// A body hit below it: it staggers (`F-032`) and it does nothing else.
    Graze,
}

impl HitKind {
    /// The rule, as a pure function: zone plus closing speed plus one threshold → one kind.
    ///
    /// **The zone wins over the speed.** A cortex hit at `min_speed_m_s` kills exactly as dead
    /// as one at 40 m/s — *"the Cortex is the only truth"* ([`HitZone`]) — so a slow kill may
    /// never be drawn as a graze.
    ///
    /// A non-finite speed falls to [`Self::Graze`]: `NaN >= x` is `false`, and the quiet answer
    /// is the right one for a number that is broken upstream.
    pub fn of(zone: HitZone, speed_m_s: f32, strong_m_s: f32) -> Self {
        match zone {
            HitZone::Cortex => HitKind::Kill,
            _ if speed_m_s >= strong_m_s => HitKind::Cut,
            _ => HitKind::Graze,
        }
    }

    /// The word. Short, because it is read out of the corner of an eye mid-swing — the same
    /// argument [`super::arm_aim::miss_label`] makes.
    pub const fn word(self) -> &'static str {
        match self {
            HitKind::Kill => "KILL",
            HitKind::Cut => "CUT",
            HitKind::Graze => "GRAZE",
        }
    }

    /// How big it is drawn, in logical pixels.
    ///
    /// **The kill is nearly twice the graze**, for the reason `objective::VERDICT_PX` is more
    /// than twice `COUNT_PX`: the outcome of the whole exchange may not be a thing you can miss
    /// while watching a titan fall over. The ceiling is [`KILL_PX`] and it is what keeps this
    /// element out of the keep-out box — see [`TOP_PCT`].
    pub const fn font_px(self) -> f32 {
        match self {
            HitKind::Kill => KILL_PX,
            HitKind::Cut => CUT_PX,
            HitKind::Graze => GRAZE_PX,
        }
    }

    /// Which of the three signal colours, by `docs/conventions.md` §3 — **amber is the cortex**,
    /// crimson is damage. The name and not the `Color`, so the value still comes out of
    /// `maps.ron` through [`signal`] and is never a literal in this domain.
    pub const fn signal_name(self) -> &'static str {
        match self {
            HitKind::Kill => "amber",
            HitKind::Cut | HitKind::Graze => "crimson",
        }
    }
}

/// The live hint: what landed, how fast, and how much of its time is left.
///
/// **A countdown and not a flag**, the same choice [`super::arm_aim::ArmMiss`] makes and for the
/// same reason: a hit is an *event*, the HUD is a *state*, and a line that stayed would be the
/// screen asserting something about the world that stopped being true half a second ago.
///
/// Counted on the generic [`Time`] and not on `Time<Fixed>`: this is view state on the frame
/// clock, and a fade that lived in fixed ticks would flicker at a rate the simulation does not
/// share. It also means the mark keeps fading **through** `F-034`'s hit-stop, which freezes the
/// two bodies and not the wall clock — the line is still there to be read while the image sits
/// still, which is the whole point of the hit-stop.
///
/// **One writer:** [`sense_hit_mark`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct HitFlash {
    /// `None` = nothing has landed recently, and the line is not drawn.
    pub kind: Option<HitKind>,
    /// The closing speed of that hit, straight off [`TitanHit`]. Meaningless while `kind` is
    /// `None`, and reset to `0.0` there so two cleared flashes compare equal.
    pub speed_m_s: f32,
    /// Seconds of mark left.
    pub left_s: f32,
}

/// Left edge and width of the line, in percent of the screen. Centred text inside it.
pub const LEFT_PCT: f32 = 25.0;
/// See [`LEFT_PCT`].
pub const WIDTH_PCT: f32 = 50.0;

/// How far above the top edge of the screen the line sits.
///
/// **28 % is not free choice — it is the keep-out box** (`F-170`): the box starts at 40 % of the
/// height, this node is 50 % of the width so it overlaps the box in x by construction, and it
/// therefore has to clear it in y. At 720 px that is 201.6 px for the top edge and about 36 px
/// of line height at [`KILL_PX`], ending at 238 px against a box that starts at 288 px — 50 px
/// of margin, and the margin scales with the screen because the position is a percentage and the
/// font is not. `tests/hud.rs::f043_the_hit_mark_stays_out_of_the_middle_in_every_kind` is what
/// says so at the pixel.
pub const TOP_PCT: f32 = 28.0;

/// The kill's font size, in logical pixels — and the ceiling this element's layout is proved
/// against.
pub const KILL_PX: f32 = 30.0;
/// A body cut at speed.
pub const CUT_PX: f32 = 22.0;
/// A scratch.
pub const GRAZE_PX: f32 = 16.0;

/// Spawns the single node. Hidden until something lands.
pub fn spawn_hit_mark(mut commands: Commands, data: Res<GameData>) {
    // Any of the three would do as a start colour; it is overwritten before the node is ever
    // shown. Crimson, because two of the three kinds are crimson.
    let crimson = signal(&data, "crimson");
    commands.spawn((
        Name::new("hud_hit_mark"),
        HitMark,
        HitFlash::default(),
        HudElement,
        Text::new(""),
        TextFont { font_size: FontSize::Px(CUT_PX), ..default() },
        TextLayout::justify(Justify::Center),
        TextColor(crimson),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(TOP_PCT),
            left: Val::Percent(LEFT_PCT),
            width: Val::Percent(WIDTH_PCT),
            display: Display::None,
            ..default()
        },
    ));
}

/// One step of the mark: a fresh hit restarts it, anything else lets it run out.
///
/// A free function so the rule is testable without a message writer, a local player and a clock
/// having to be arranged first — the same reason [`super::arm_aim::step_miss`] is one.
///
/// **`hold_s <= 0.0` switches the element off entirely**, and it is checked *before* the fresh
/// hit rather than after: `F-043`'s *"vollstaendig abschaltbar"* has to mean no frame at all,
/// not one frame per hit.
///
/// **A fresh hit always wins over a running one**, including the kill that follows its own
/// graze three ticks later (`scripts/f032-swords.txt`, act A: `Torso` at 154, `Cortex` at 157).
/// The second one is the one the player is asking about.
pub fn step_flash(
    flash: HitFlash,
    fresh: Option<(HitKind, f32)>,
    dt_s: f32,
    hold_s: f32,
) -> HitFlash {
    if !(hold_s > 0.0) {
        return HitFlash::default();
    }
    match fresh {
        Some((kind, speed_m_s)) => HitFlash { kind: Some(kind), speed_m_s, left_s: hold_s },
        None => {
            let left_s = flash.left_s - dt_s.max(0.0);
            // Cleared to the exact default and not to a small remainder, so `set_if_neq` stops
            // writing once the mark is over (`CLAUDE.md` rule 6).
            if left_s > 0.0 && flash.kind.is_some() {
                HitFlash { kind: flash.kind, speed_m_s: flash.speed_m_s, left_s }
            } else {
                HitFlash::default()
            }
        }
    }
}

/// What the line says. Pure, so "what does the screen read" is one function and not a system.
///
/// One decimal on the speed: `20.7 m/s` is a number a player can compare between two runs, and
/// `20.67` is a number nobody reads in 0.5 s.
pub fn mark_text(kind: HitKind, speed_m_s: f32) -> String {
    format!("{}  {:.1} m/s", kind.word(), speed_m_s)
}

/// How opaque the line is drawn: full for the first half of its life, then linearly out.
///
/// **The fade is the *"Trefferblitz"* half of `F-043`'s row** — the thing that makes a landed
/// blade read as an impact rather than as a caption. It holds at 1.0 first so the word is
/// legible before it starts going, which a straight linear fade from the first frame is not.
///
/// Returns `0.0` for a cleared flash and for a `hold_s` that is not positive, so the caller
/// never has to special-case the switched-off element.
pub fn fade(flash: HitFlash, hold_s: f32) -> f32 {
    if flash.kind.is_none() || !(hold_s > 0.0) {
        return 0.0;
    }
    let left = (flash.left_s / hold_s).clamp(0.0, 1.0);
    (left * 2.0).min(1.0)
}

/// **Reads every landed blade of the local player and starts the mark.**
///
/// **Filtered by [`PlayerId`]** and not by "the only hit there is": in a session with a team
/// mate his cuts arrive on the same channel, and a line that flashed for someone else's blade
/// would be a lie about the local player's own gear (`docs/multiplayer.md` rule 1).
///
/// The `info!` is what a headless run leaves behind — `scripts/f032-swords.txt` has no screen,
/// and the log is the only evidence such a run can produce. It is written once per landed hit
/// and never per frame.
///
/// **One writer of [`HitFlash`].**
pub fn sense_hit_mark(
    time: Res<Time>,
    data: Res<GameData>,
    mut hits: MessageReader<TitanHit>,
    players: Query<&PlayerId, With<LocalPlayer>>,
    mut marks: Query<&mut HitFlash, With<HitMark>>,
) {
    let hold_s = data.gear.feel.hit_mark_s;
    let strong_m_s = data.gear.feel.strong_hit_m_s;
    let dt = time.delta_secs();

    let mut fresh: Option<(HitKind, f32)> = None;
    if let Some(me) = players.iter().next() {
        for hit in hits.read() {
            if hit.by != *me {
                continue;
            }
            let kind = HitKind::of(hit.zone, hit.speed_m_s, strong_m_s);
            info!(
                "hit mark: {} on {:?} at {:.2} m/s ({:?})",
                kind.word(),
                hit.zone,
                hit.speed_m_s,
                hit.titan
            );
            fresh = Some((kind, hit.speed_m_s));
        }
    } else {
        // No local player yet (the menu before a sortie): drain the cursor anyway, or the first
        // frame after a spawn replays a backlog of somebody else's hits.
        hits.clear();
    }

    for mut flash in &mut marks {
        flash.set_if_neq(step_flash(*flash, fresh, dt, hold_s));
    }
}

/// Puts the word, the number, the size and the colour on screen — and takes them away again.
///
/// Every write is guarded by a comparison: outside a mark this system does nothing at all
/// (`CLAUDE.md` rule 6).
pub fn show_hit_mark(
    data: Res<GameData>,
    mut marks: Query<(&HitFlash, &mut Text, &mut TextColor, &mut TextFont, &mut Node), With<HitMark>>,
) {
    let hold_s = data.gear.feel.hit_mark_s;
    for (flash, mut text, mut colour, mut font, mut node) in &mut marks {
        let Some(kind) = flash.kind else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            if !text.0.is_empty() {
                text.0.clear();
            }
            continue;
        };
        let wanted = mark_text(kind, flash.speed_m_s);
        let ink = signal(&data, kind.signal_name()).with_alpha(fade(*flash, hold_s));
        let size = FontSize::Px(kind.font_px());
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
        if text.0 != wanted {
            text.0 = wanted;
        }
        if colour.0 != ink {
            colour.0 = ink;
        }
        if font.font_size != size {
            font.font_size = size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `F-043` — the three kinds are three different words, three different sizes and two
    /// different colours. A table whose rows read the same explains nothing.
    #[test]
    fn f043_every_hit_kind_gets_its_own_word_and_its_own_size() {
        let all = [HitKind::Kill, HitKind::Cut, HitKind::Graze];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a.word(), b.word(), "{a:?} and {b:?} say the same word");
                assert_ne!(
                    a.font_px(),
                    b.font_px(),
                    "{a:?} and {b:?} are drawn at the same size — a player who cannot tell \
                     amber from crimson reads one line, not three"
                );
            }
            assert!(a.font_px() <= KILL_PX, "{a:?} is bigger than the size the layout is proved at");
        }
        assert_ne!(HitKind::Kill.signal_name(), HitKind::Cut.signal_name());
    }

    /// **The zone wins over the speed.** A slow cortex cut is still the kill.
    #[test]
    fn f043_the_cortex_is_the_kill_at_any_speed() {
        assert_eq!(HitKind::of(HitZone::Cortex, 0.1, 18.0), HitKind::Kill);
        assert_eq!(HitKind::of(HitZone::Cortex, 90.0, 18.0), HitKind::Kill);
        assert_eq!(HitKind::of(HitZone::Torso, 20.67, 18.0), HitKind::Cut);
        assert_eq!(HitKind::of(HitZone::Torso, 17.99, 18.0), HitKind::Graze);
        // A number that is broken upstream gets the quiet answer, not the loud one.
        assert_eq!(HitKind::of(HitZone::Torso, f32::NAN, 18.0), HitKind::Graze);
    }

    /// The mark starts on the hit and clears itself, and the second hit of one pass wins.
    #[test]
    fn f043_the_mark_starts_on_the_hit_and_clears_itself() {
        let hold = 0.5_f32;
        let quiet = HitFlash::default();
        assert_eq!(step_flash(quiet, None, 1.0 / 60.0, hold), quiet, "it invented a hit");

        let hit = step_flash(quiet, Some((HitKind::Cut, 20.67)), 1.0 / 60.0, hold);
        assert_eq!(hit.kind, Some(HitKind::Cut));
        assert_eq!(hit.left_s, hold);
        assert_eq!(fade(hit, hold), 1.0, "the word is not legible on the frame it appears");

        // It survives its hold and not a frame longer.
        let mut running = hit;
        for _ in 0..29 {
            running = step_flash(running, None, 1.0 / 60.0, hold);
        }
        assert_eq!(running.kind, Some(HitKind::Cut), "it vanished too fast");
        assert!(fade(running, hold) < 1.0, "it never started fading");
        for _ in 0..2 {
            running = step_flash(running, None, 1.0 / 60.0, hold);
        }
        assert_eq!(running, HitFlash::default(), "the mark outlived the hit it described");

        // Act A of `scripts/f032-swords.txt`: a torso graze, then the cortex three ticks later.
        let graze = step_flash(quiet, Some((HitKind::Graze, 20.67)), 1.0 / 60.0, hold);
        let kill = step_flash(graze, Some((HitKind::Kill, 21.00)), 1.0 / 60.0, hold);
        assert_eq!(kill.kind, Some(HitKind::Kill), "the graze covered its own kill");
        assert_eq!(kill.left_s, hold);
    }

    /// `F-043` *"vollstaendig abschaltbar"* — one RON value, no menu, and **no frame at all**.
    #[test]
    fn f043_a_hold_of_zero_seconds_switches_the_element_off() {
        for hold in [0.0_f32, -1.0, f32::NAN] {
            let hit = step_flash(HitFlash::default(), Some((HitKind::Kill, 30.0)), 0.0, hold);
            assert_eq!(
                hit,
                HitFlash::default(),
                "hit_mark_s = {hold} still drew a mark for one frame"
            );
            assert_eq!(fade(hit, hold), 0.0);
        }
    }

    /// The line carries the number, and the number is the one that came in.
    #[test]
    fn f043_the_line_carries_the_speed_it_was_given() {
        assert_eq!(mark_text(HitKind::Kill, 21.0), "KILL  21.0 m/s");
        assert_eq!(mark_text(HitKind::Graze, 8.04), "GRAZE  8.0 m/s");
        assert!(
            mark_text(HitKind::Cut, 20.67).contains("20.7"),
            "the speed is not on the line — then the line is a word, not a reading"
        );
    }
}
