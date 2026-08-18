//! The main lobby — **the front door the deployment pads never had.**
//!
//! > *„und eine main lobby in der man die mission starten kann"* — the user, 2026-08-13
//! (`docs/NEXT.md` §1D req 8).
//!
//! ## What this is, and what it deliberately is not
//!
//! It is **not a second way to start a sortie.** `mission::hub::deploy_on_contact` already
//! starts one when a player stands on a pad, that path is 🟧 with 35 asserts behind it
//! (`scripts/f070-hub.txt`), and rebuilding it here would have given the one thing the whole
//! run hangs on two mechanisms. So this screen picks a template and a difficulty and then
//! **asks**: `shared::DeployRequest`, read by `mission`, which sets `Sortie` and the phase
//! exactly as a pad does. Give the mechanism a front door — do not build a second mechanism.
//!
//! ## Everything on it comes out of `missions.ron`
//!
//! The title is `hub.name`, the mission buttons are `templates`, the difficulty buttons are
//! that template's `difficulties`, and the line under them is that difficulty's `kill_target`
//! and `target_duration_s`. There is no list in this file — a fourth difficulty is file work
//! (§6 rule 2), and it appears on this screen without a line of Rust being touched.
//!
//! **The default choice is the hub's first pad**, `hub.deployments[0]`. That is not a guess:
//! `missions.ron` puts *recruit* straight ahead of the spawn point and calls it *"the door you
//! find without looking for it"*. The screen and the floor therefore recommend the same sortie.
//!
//! ## The pads stay
//!
//! Walking onto a circle is still how the hub deploys, and this screen changes nothing about
//! it. The two doors lead into the same room: `hub::deploy_on_contact` writes `Sortie` and the
//! phase, [`lobby_buttons`] writes a message that makes `mission` do the same.

use bevy::prelude::*;

use super::{plate, Screen};
use crate::data::GameData;
use crate::shared::DeployRequest;

/// What the player has picked, until he picks something else.
///
/// A `Resource` for the same reason [`Screen`] is one: it is the state of *this session's
/// screen*, not of a player — and there is exactly one sortie for everybody in the hub
/// (`mission::hub::Sortie` carries the long form of that argument).
///
/// Both fields are `Option` and both may name something that is not in the file any more; every
/// read goes through [`chosen`], which re-derives against `missions.ron` instead of trusting
/// what is stored. A stale key therefore falls back to the default door and never deploys
/// something nobody picked.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LobbyChoice {
    pub mission: Option<String>,
    pub difficulty: Option<String>,
}

/// What a button on the lobby does.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub enum LobbyAction {
    PickMission(String),
    PickDifficulty(String),
    /// Start it. The one button on this screen that changes the game.
    Deploy,
    Back,
}

/// The choice as it stands **against the file**: a template that exists, and a difficulty that
/// exists inside it.
///
/// `None` for the difficulty is the tutorial's case and means the same thing it means in
/// `mission::SortieOrder`: fly the template's own numbers.
pub fn chosen(data: &GameData, choice: &LobbyChoice) -> Option<(String, Option<String>)> {
    let missions = &data.missions;
    // The default door is the hub's first pad — `missions.ron` calls it "the door you find
    // without looking for it". Falling back to the first template only if there are no pads at
    // all keeps this screen working on a map whose hub has not been laid out yet.
    let default_mission = missions
        .hub
        .deployments
        .first()
        .map(|pad| pad.mission.clone())
        .or_else(|| missions.templates.keys().next().cloned())?;
    let name = choice
        .mission
        .clone()
        .filter(|m| missions.templates.contains_key(m))
        .unwrap_or(default_mission);
    let template = missions.templates.get(&name)?;

    let default_difficulty = missions
        .hub
        .deployments
        .iter()
        .find(|pad| pad.mission == name)
        .map(|pad| pad.difficulty.clone())
        .filter(|d| template.difficulties.contains_key(d))
        .or_else(|| template.difficulties.keys().next().cloned());
    let difficulty = choice
        .difficulty
        .clone()
        .filter(|d| template.difficulties.contains_key(d))
        .or(default_difficulty);
    Some((name, difficulty))
}

/// Builds the plate out of `missions.ron` and the current choice.
pub fn spawn_lobby_screen(commands: &mut Commands, data: &GameData, choice: &LobbyChoice) {
    let missions = &data.missions;
    let picked = chosen(data, choice);
    commands.spawn(plate::root(Screen::Lobby, "lobby")).with_children(|screen| {
        screen.spawn(plate::title(missions.hub.name.clone()));
        screen.spawn(plate::note("Pick a sortie. The deployment pads outside do the same."));

        let Some((mission, difficulty)) = picked else {
            // A file with no templates at all. Loud on screen rather than an empty plate that
            // looks like a bug in the menu.
            screen.spawn(plate::note("assets/data/missions.ron has no templates"));
            return;
        };

        screen.spawn(plate::row()).with_children(|line| {
            for (key, template) in &missions.templates {
                line.spawn((
                    Name::new(format!("lobby_mission_{key}")),
                    LobbyAction::PickMission(key.clone()),
                    plate::button(200.0, *key == mission),
                ))
                .with_child(plate::label(template.name.clone()));
            }
        });

        let template = &missions.templates[&mission];
        if template.difficulties.is_empty() {
            screen.spawn(plate::note("no difficulty levels — the mission's own numbers"));
        } else {
            screen.spawn(plate::row()).with_children(|line| {
                for (key, level) in &template.difficulties {
                    line.spawn((
                        Name::new(format!("lobby_difficulty_{key}")),
                        LobbyAction::PickDifficulty(key.clone()),
                        plate::button(150.0, Some(key) == difficulty.as_ref()),
                    ))
                    .with_child(plate::label(level.name.clone()));
                }
            });
        }

        // The numbers of what is actually about to be flown — out of the file, never out of a
        // sentence in this module.
        let (kills, seconds) = match difficulty.as_ref().and_then(|d| template.difficulties.get(d))
        {
            Some(level) => (level.kill_target, level.target_duration_s),
            None => (template.kill_target, template.target_duration_s),
        };
        screen.spawn(plate::note(format!(
            "{kills} cortex kills - {:.0}:{:02} on the clock",
            (seconds / 60.0).floor(),
            (seconds % 60.0).round() as u32
        )));

        screen
            .spawn((Name::new("lobby_Deploy"), LobbyAction::Deploy, plate::button(plate::BUTTON_W, true)))
            .with_child(plate::label("Deploy"));
        screen
            .spawn((Name::new("lobby_Back"), LobbyAction::Back, plate::button(plate::BUTTON_W, false)))
            .with_child(plate::label("Back  (Esc)"));
    });
}

/// What the buttons do.
///
/// **Deploy asks and does not decide.** It writes [`DeployRequest`] and hands the screen back to
/// the game; `mission` reads the message in `Update` — it has to be `Update`, because this
/// screen has `Time<Virtual>` stopped and `FixedUpdate` is therefore not running at all
/// (`menu::apply_screen`).
///
/// Picking a mission clears the difficulty rather than leaving a key from another template
/// standing. [`chosen`] would have filtered it anyway; clearing it means the *stored* state and
/// the *shown* state are the same thing, which is one fewer place for them to disagree.
pub fn lobby_buttons(
    buttons: Query<(&Interaction, &LobbyAction)>,
    data: Res<GameData>,
    mut choice: ResMut<LobbyChoice>,
    mut screen: ResMut<Screen>,
    mut deploy: MessageWriter<DeployRequest>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            LobbyAction::PickMission(key) => {
                if choice.mission.as_ref() != Some(key) {
                    choice.mission = Some(key.clone());
                    choice.difficulty = None;
                }
            }
            LobbyAction::PickDifficulty(key) => {
                if choice.difficulty.as_ref() != Some(key) {
                    choice.difficulty = Some(key.clone());
                }
            }
            LobbyAction::Deploy => {
                let Some((template, difficulty)) = chosen(&data, &choice) else {
                    error!("Deploy was pressed with nothing to fly — missions.ron has no templates");
                    continue;
                };
                info!("lobby: deploying {template:?} at {difficulty:?}");
                deploy.write(DeployRequest { template, difficulty });
                *screen = Screen::Playing;
            }
            LobbyAction::Back => *screen = Screen::Playing,
        }
    }
}
