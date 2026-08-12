//! `--screenshot <path>` — **a PNG out of the running game, without a compositor and
//! without hand work.**
//!
//! `docs/ACCEPTANCE.md` says literally: "No image, no 🟧 — no exceptions." A screenshot a
//! human takes at the right moment with the right key is still not evidence but an
//! anecdote: it is not repeatable, not scriptable, and tomorrow it shows something else.
//! `--screenshot <path> --ticks <n>` is both — **the same command delivers the same
//! image**, because the simulation runs on a fixed tick and the trigger is a tick, not a
//! second and not a key.
//!
//! ## The three modes
//!
//! | launch | window | GPU | screenshot target |
//! |---|---|---|---|
//! | default | yes | yes | [`Screenshot::primary_window`] |
//! | `--offscreen` | no | yes | [`Screenshot::image`] — the camera renders into an `Image` |
//! | `--headless` | no | **no** (`backends: None`) | **no image possible** |
//!
//! `--headless` does not stand here without an image out of convenience: it sets
//! `backends: None` in [`crate::base_plugins`], so wgpu never even looks for an adapter.
//! Without an adapter there is no render target and therefore nothing to read back.
//! **`--offscreen` is exactly the answer to that** and the reason this third mode exists
//! (`docs/QUESTIONS.md` Q-009).
//!
//! ## Evidenced against the installed source, not from memory
//!
//! Bevy 0.19 is newer than any memory that touches this project. Every claim in this file
//! comes with a file and a line:
//!
//! - `bevy_render-0.19.0/src/view/window/screenshot.rs:78-80` — [`Screenshot`] is a
//!   **component** on an entity of its own, not a call and not a system.
//! - `.../screenshot.rs:134` — [`save_to_disk`] is an **observer**
//!   (`impl FnMut(On<..>)`) that only runs once the image is really there. Screenshots are
//!   asynchronous.
//! - `.../screenshot.rs:47-53` — [`ScreenshotCaptured`] is the event carrying the finished
//!   `Image`.
//! - `.../screenshot.rs:309-328` — an `Image` is a **valid screenshot target**. That is
//!   precisely why an image can exist at all without a window.
//! - `bevy_camera-0.19.0/src/camera.rs:376-384` — `RenderTarget` is a **required
//!   component** of `Camera`. So you set the target by replacing the component on the
//!   camera entity, not by writing into a field.
//! - `bevy_image-0.19.0/src/image.rs:1232-1246` — `Image::new_target_texture` sets the
//!   three required `TextureUsages`. An `Image` without `RENDER_ATTACHMENT` is not a target.
//! - `bevy_render-0.19.0/src/lib.rs:501-506` — the window is an `Option` when the renderer
//!   is built. **No window is not an error**, it is `compatible_surface: None`.
//!
//! ## Why the exit lives here and not with `--ticks`
//!
//! `crate::exit_after_ticks` would write `AppExit` immediately at `tick >= ticks` — that
//! is, in exactly the moment the screenshot is triggered and **before** anyone has read it
//! back from the GPU. The run would end green and without a file. So when `--screenshot`
//! is set, this file takes over the exit, and it exits only once the PNG is on disk and is
//! not empty.
//!
//! Taking the exit over means owning the **verdict** too, and for a while it did not:
//! [`exit_when_written`] wrote `AppExit::Success` unconditionally, so a picture run with red
//! asserts ended at exit 0 (`FIND-074`). [`shot_verdict`] is that missing half, and it runs
//! *after* the file is on disk — the image is worth more than the exit code's timing.

use std::path::PathBuf;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};

use crate::debug::ScriptRun;
use crate::shared::{Cli, Tick};

/// The resolution of the offscreen target.
///
/// Not a RON number: this is not a game value but the edge length of a test tool (rule 2
/// in `CLAUDE.md` means titan values, blade tiers, gas costs). 1280x720 is big enough to
/// make a scale readable and small enough for the file to fit into a repository.
pub const OFFSCREEN_WIDTH: u32 = 1280;
pub const OFFSCREEN_HEIGHT: u32 = 720;

/// When it triggers if `--screenshot` comes without `--ticks`.
///
/// At 60 Hz that is two seconds. The first frame is no good: camera, light and world are
/// created through `Commands` and exist only at the end of their tick — an image at tick 0
/// is reliably black, and a black PNG passes as "image present" although there is nothing
/// to see.
pub const DEFAULT_SHOT_TICK: u64 = 120;

/// How many frames the PNG is waited for before the run counts as **failed**.
///
/// A run waiting for an image that never comes is worse than one that fails: it blocks a
/// session without reporting anything.
pub const MAX_WAIT_FRAMES: u32 = 900;

/// The one screenshot job of this run.
#[derive(Resource, Debug)]
pub struct ScreenshotJob {
    /// Where the PNG is written.
    pub path: PathBuf,
    /// From which simulation tick on it triggers.
    pub at_tick: u64,
    /// Without a window: the camera renders into an `Image`, and that is the screenshot
    /// target.
    pub offscreen: bool,
    /// The offscreen target, once it stands.
    target: Option<Handle<Image>>,
    /// Whether the screenshot entity has been spawned yet.
    triggered: bool,
    /// Whether [`ScreenshotCaptured`] has arrived — that is, whether the GPU really
    /// delivered.
    captured: bool,
    /// Frames since the trigger, against [`MAX_WAIT_FRAMES`].
    frames: u32,
}

impl ScreenshotJob {
    fn new(start: &Cli, path: PathBuf) -> Self {
        let at_tick = if start.ticks > 0 { start.ticks } else { DEFAULT_SHOT_TICK };
        Self {
            path,
            at_tick,
            offscreen: start.offscreen,
            target: None,
            triggered: false,
            captured: false,
            frames: 0,
        }
    }
}

/// Sits on the camera whose target has already been swapped — so it happens exactly once.
#[derive(Component)]
pub struct ShotTarget;

/// Installs the screenshot systems into the app when `--screenshot` is set.
///
/// Called by [`crate::debug::DebugPlugin`]. Without `--screenshot` nothing happens here at
/// all: a test tool that runs along in normal operation is compute spent on something
/// nobody asked for.
pub fn install(app: &mut App, start: &Cli) {
    let Some(path) = start.image.clone() else {
        return;
    };

    if !start.has_gpu() {
        // Loud, with the reason AND with a non-zero exit code: "--headless --screenshot"
        // looks like a reasonable line and cannot possibly work. A run that does not
        // produce the requested file and still ends green is exactly the kind of silent
        // failure that costs an hour of searching at the wrong end (§9).
        error!(
            "--screenshot {} together with --headless: --headless turns the wgpu adapter off \
             (backends: None), so there is no image to read back at all. --offscreen is \
             what is meant (docs/FRAGEN.md Q-009).",
            path.display()
        );
        app.add_systems(Last, abort_no_gpu);
        return;
    }

    let job = ScreenshotJob::new(start, path);
    info!(
        "screenshot job: {} at tick {} ({})",
        job.path.display(),
        job.at_tick,
        if job.offscreen { "offscreen" } else { "window" }
    );
    if start.ticks == 0 {
        info!("--screenshot without --ticks: it triggers at tick {DEFAULT_SHOT_TICK}");
    }

    app.insert_resource(job);
    if start.offscreen {
        app.add_systems(Update, attach_offscreen_target);
    }
    app.add_systems(Update, trigger_screenshot).add_systems(Last, exit_when_written);
}

/// Aborts when an image was requested that cannot exist.
///
/// The reason is already in the log as an `error!`; all that stands here is the exit code,
/// so that a workflow **sees** the failure instead of missing it in a log line.
fn abort_no_gpu(mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::error());
}

/// Without a window the camera needs a different target — otherwise it renders into a
/// window that does not exist.
///
/// **This is a foreign component on a foreign entity.** `render` owns the camera; `debug`
/// swaps its `RenderTarget` here. That is allowed because it happens exactly once, only
/// under `--offscreen` and only on a test path — in normal operation this system does not
/// even exist in the app (see [`install`]). It has its own line in the allowed list of
/// `docs/architecture.md`.
fn attach_offscreen_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut job: ResMut<ScreenshotJob>,
    cameras: Query<Entity, (With<Camera3d>, Without<ShotTarget>)>,
) {
    if cameras.is_empty() {
        // The camera is only created once there is a local player (`render`), and
        // `Commands` are deferred. Do not warn: this is the normal case for the first
        // frames.
        return;
    }

    let target = match &job.target {
        Some(h) => h.clone(),
        None => {
            // Storage format and view format as in the Bevy example
            // `examples/3d/render_to_texture.rs:31-35`. The view format is the decisive
            // part: `Image::try_into_dynamic` knows Rgba8UnormSrgb but **not** Rgba8Unorm
            // (`bevy_image-0.19.0/src/image_texture_conversion.rs:174-197`) — with the
            // wrong format the image comes back from the GPU and then cannot be saved.
            let h = images.add(Image::new_target_texture(
                OFFSCREEN_WIDTH,
                OFFSCREEN_HEIGHT,
                TextureFormat::Rgba8Unorm,
                Some(TextureFormat::Rgba8UnormSrgb),
            ));
            info!("offscreen target: {OFFSCREEN_WIDTH}x{OFFSCREEN_HEIGHT}");
            job.target = Some(h.clone());
            h
        }
    };

    for camera in &cameras {
        commands
            .entity(camera)
            .insert((RenderTarget::Image(target.clone().into()), ShotTarget));
    }
}

/// Triggers the screenshot — exactly once, on a tick, not on a second.
fn trigger_screenshot(mut commands: Commands, tick: Res<Tick>, mut job: ResMut<ScreenshotJob>) {
    if job.triggered || tick.0 < job.at_tick {
        return;
    }

    if job.offscreen && job.target.is_none() {
        // Without a target there is nothing to photograph. Wait instead of making an empty
        // image.
        return;
    }

    // The directory has to exist before `image` writes into it — otherwise saving fails
    // inside an observer, where there is only a log line and no exit code.
    if let Some(dir) = job.path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        error!("{} cannot be created — {e}", dir.display());
    }

    let target = match &job.target {
        Some(h) => Screenshot::image(h.clone()),
        None => Screenshot::primary_window(),
    };

    commands
        .spawn(target)
        .observe(save_to_disk(job.path.clone()))
        .observe(on_screenshot_captured);

    info!("screenshot triggered at tick {}", tick.0);
    job.triggered = true;
}

/// Records that the GPU delivered.
///
/// A second observer alongside [`save_to_disk`] instead of a save path of our own: Bevy's
/// save path is the better tested one, and two observers on the same entity both run at
/// the same synchronization point — so long before `Last`, where [`exit_when_written`]
/// looks.
fn on_screenshot_captured(_: On<ScreenshotCaptured>, mut job: ResMut<ScreenshotJob>) {
    job.captured = true;
}

/// **The verdict of a screenshot run** — and the one rule that is deliberately *narrower*
/// than [`crate::debug::cutoff_verdict`]'s.
///
/// Only one of the two facts that decide a `--ticks` run decides this one:
///
/// 1. **An assert failed → error, always.** This is the invariant that holds under every
///    flag combination, and it is the whole reason this function exists (`FIND-074`):
///    `exit_when_written` used to write [`AppExit::Success`] without ever looking at the
///    script, so `--offscreen --script … --ticks 400 --screenshot x.png` reported **exit 0
///    with red asserts in its own log** (measured: 2 red, 820 429 bytes written, exit 0) —
///    the same false green as `FIND-032`, one file further on. That is
///    `docs/HANDOVER.md` §2 a third time.
/// 2. **The script did not reach its end → NOT an error here**, and that is the opposite of
///    what `cutoff_verdict` does. The asymmetry is the point: a `--ticks` run is cut off
///    *by accident* (the limit was written too small and the run has not shown what the
///    script claims), a screenshot run is cut off *on purpose* — `--ticks 152` picks the
///    one moment that is to be photographed, and the instructions after it are not meant to
///    run. Applying rule 2 here would make **every** picture run exit 1 and the workflow
///    that `docs/ACCEPTANCE.md` demands the images from useless. The narrow invariant is
///    all that is needed: a failed assert is never green.
///
/// Called **after** the PNG is on disk, never before. An early guard would end the run at
/// the failing assert and destroy the image the run exists for — losing the picture would
/// be the worse bug (`FIND-032`, last paragraph).
pub fn shot_verdict(run: &ScriptRun) -> AppExit {
    let failed = run.failures.len();
    if failed == 0 {
        return AppExit::Success;
    }
    error!(
        "screenshot run: {failed} of {} asserts failed — the image was written, the run is \
         still RED. (The script was not required to reach its end; a shot tick cuts it off \
         on purpose.)",
        run.checked
    );
    for m in &run.failures {
        error!("  {m}");
    }
    AppExit::error()
}

/// Ends the run — but only once the file is really there.
///
/// **Not "the screenshot was triggered".** A run that ends because it requested something
/// proves nothing about the result (§9). What is checked is the file: it exists and it is
/// not empty. Only then the script's own verdict decides ([`shot_verdict`]) — in that
/// order, so a red run still leaves its picture behind.
fn exit_when_written(
    mut job: ResMut<ScreenshotJob>,
    run: Res<ScriptRun>,
    mut exit: MessageWriter<AppExit>,
) {
    if !job.triggered {
        return;
    }
    job.frames += 1;

    if job.captured {
        match std::fs::metadata(&job.path) {
            Ok(m) if m.len() > 0 => {
                info!("image written: {} ({} bytes)", job.path.display(), m.len());
                exit.write(shot_verdict(&run));
                return;
            }
            Ok(_) => {
                error!("{} is 0 bytes long", job.path.display());
                exit.write(AppExit::error());
                return;
            }
            // The observer may still be writing in the same round — one more frame.
            Err(_) => {}
        }
    }

    if job.frames >= MAX_WAIT_FRAMES {
        error!(
            "after {MAX_WAIT_FRAMES} frames still no image at {} — the screenshot was \
             triggered but never read back",
            job.path.display()
        );
        exit.write(AppExit::error());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(ticks: u64, offscreen: bool) -> ScreenshotJob {
        let start = Cli { ticks, offscreen, ..Cli::default() };
        ScreenshotJob::new(&start, PathBuf::from("docs/images/probe"))
    }

    #[test]
    fn without_ticks_it_does_not_trigger_at_zero() {
        // An image at tick 0 is reliably black: camera, light and world are created
        // through Commands and exist only at the end of their tick.
        let a = job(0, false);
        assert_eq!(a.at_tick, DEFAULT_SHOT_TICK);
        assert!(a.at_tick > 0, "tick 0 would be a black image");
    }

    #[test]
    fn ticks_determine_the_trigger() {
        assert_eq!(job(300, false).at_tick, 300);
    }

    #[test]
    fn offscreen_is_carried_over() {
        assert!(job(60, true).offscreen);
        assert!(!job(60, false).offscreen);
    }

    #[test]
    fn a_fresh_job_is_neither_triggered_nor_captured() {
        // Otherwise `exit_when_written` would end successfully on the first frame, without
        // a screenshot ever having been requested.
        let a = job(60, false);
        assert!(!a.triggered);
        assert!(!a.captured);
        assert_eq!(a.frames, 0);
    }
}
