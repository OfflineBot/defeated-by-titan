# bevy — was die installierte Version 0.19.0 wirklich heisst, und was sie kostet

Stand: 2026-08-09 · Stufe: 🟨 (jede Zeile hier ist in der installierten Quelle nachgesehen;
gelaufen ist davon noch nichts)

> ⚠️ **Bevys API dreht sich zwischen Minor-Versionen hart.** In der Doku der **installierten**
> Version nachsehen (`cargo doc --open -p bevy`, oder direkt in
> `~/.cargo/registry/src/*/bevy_*-0.19.0/src/`), nicht aus dem Gedaechtnis und nicht aus
> Blogposts von vor zwei Versionen. **Prüfen, nicht annehmen** (`prompts/init.md` §3).

Alles hier ist gegen `~/.cargo/registry/src/index.crates.io-*/bevy_*-0.19.0/` geprueft, mit
Datei und Zeile. Was nicht belegt ist, steht nicht drin.

## Namen, die zuletzt gewandert sind

| Sache | in 0.19.0 | Beleg |
|---|---|---|
| Gepufferte Nachrichten | **`Message`**, nicht `Event` | `bevy_ecs/src/message/mod.rs:100` (`pub trait Message`) |
| Registrieren | **`App::add_message::<M>()`** | `bevy_app/src/app.rs:435` |
| Schreiben / Lesen | **`MessageWriter<M>` / `MessageReader<M>`** | `bevy_ecs/src/message/message_writer.rs:62`, `message_reader.rs:34` |
| Delta in Sekunden | **`time.delta_secs()`** | `bevy_time/src/time.rs:283` |
| Feste Schrittweite | **`Time::<Fixed>::from_hz(f64)`** | `bevy_time/src/fixed.rs:105` |
| Feste Schedules | `FixedPreUpdate`, `FixedUpdate`, `FixedPostUpdate` | `bevy_app/src/main_schedule.rs:118/133/141` |
| Sichtbares | **`Mesh3d(pub Handle<Mesh>)`** und **`MeshMaterial3d<M>(pub Handle<M>)`** — einzelne Komponenten, keine Bundles | `bevy_mesh/src/components.rs:102`, `bevy_pbr/src/mesh_material.rs:41` |
| Kamera | **`Camera3d`** mit `#[require(Camera, Projection)]` | `bevy_camera/src/components.rs:25` |
| Text | **`Text(pub String)`**, dazu `TextFont`, `TextColor(pub Color)` | `bevy_ui/src/widget/text.rs:111`, `bevy_text/src/text.rs:376/1066` |
| Rueckgabe von `App::run()` | **`AppExit`** (`Success` \| `Error(NonZero<u8>)`) | `bevy_app/src/app.rs:192`, `1568` |

## Die Falle, die uns wirklich getroffen haette: `AmbientLight` ist ein **Component**

In 0.19 ist `AmbientLight` **kein `Resource` mehr**, sondern ein Component mit
`#[require(Camera)]` — es gehoert an die **Kamera** und ueberschreibt dort die Vorgabe
`GlobalAmbientLight` (`bevy_light/src/ambient_light.rs:9-12`). Wer `insert_resource` schreibt,
bekommt keinen Compilerfehler an der erwarteten Stelle und eine Szene, die einfach dunkler ist
als gedacht.

`DirectionalLight` verlangt seinerseits `Transform` und `Visibility` per `#[require(..)]`
(`bevy_light/src/directional_light.rs:67-73`) — sie muessen nicht mehr von Hand mitgegeben
werden.

## Ohne Grafiksitzung starten

Beides zusammen, sonst sucht wgpu trotzdem einen Adapter:

| Teil | exakt |
|---|---|
| kein Fenster | `WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, .. }` — `bevy_window/src/lib.rs:74`, `158-175` |
| kein Renderer | `RenderPlugin { render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { backends: None, ..default() })), .. }` — `bevy_render/src/settings.rs:41`, `223-228` |
| die Schleife | `ScheduleRunnerPlugin::run_loop(Duration)` bzw. `::run_once()` — `bevy_app/src/schedule_runner.rs:57`, `64` |

⚠️ **`RenderCreation::Automatic` nimmt eine `Box<WgpuSettings>`**, nicht `WgpuSettings`
(`settings.rs:227`). Das ist genau die Sorte Detail, die man aus dem Gedaechtnis falsch
schreibt.

`ExitCondition::DontExit` ist noetig, **weil ohne Fenster sonst sofort beendet wird**: die
Vorgabe `OnAllClosed` sieht null Fenster und faehrt herunter (`bevy_window/src/lib.rs:72-74`).

### Drei Fallen genau an dieser Stelle

1. **`primary_window: None` allein reicht nicht.** `WinitPlugin::build` baut den Event-Loop
   **unbedingt** und mit `.expect("Failed to build event loop")`
   (`bevy_winit/src/lib.rs:90-128`); winit liefert unter Linux ohne `WAYLAND_DISPLAY` und ohne
   `DISPLAY` einen Fehler (`winit-0.30.13/src/platform_impl/linux/mod.rs:754-766`). Es panikt
   also, **bevor ein einziges System laeuft**. Einziger Ausweg: `disable::<WinitPlugin>()`.
2. **`ScheduleRunnerPlugin` ist NICHT in `DefaultPlugins`**, solange das Feature `bevy_window`
   an ist (`bevy_internal/src/default_plugins.rs:19-20`). Nach `disable::<WinitPlugin>()`
   treibt sonst **niemand** die App an: `App::run()` faellt auf `run_once` zurueck und macht
   genau **ein** Update (`bevy_app/src/app.rs:159`, `1539-1550`). Also `.add(...)` — und
   **nicht** `.set(...)`, denn `set` und `disable` **paniken**, wenn der Plugin gar nicht in
   der Gruppe ist (`bevy_app/src/plugin_group.rs:312-319`, `496-508`).
3. **`disable::<WinitPlugin>()` selbst braucht einen `cfg`-Riegel.** Baut jemand mit
   `--no-default-features`, gibt es weder den Modulpfad `bevy::winit` noch den Eintrag in der
   Gruppe — der Aufruf waere ein Compilerfehler bzw. eine Panik. Deshalb steht er in
   `src/lib.rs` hinter `#[cfg(any(feature = "x11", feature = "wayland"))]`.

### Und ein Feld, das nur wie ein Schalter aussieht

`DirectionalLight::contact_shadows_enabled` bewirkt **allein nichts**. Kontaktschatten sind ein
Screenspace-Verfahren und brauchen zusaetzlich eine `ContactShadows`-Komponente an der Kamera.
Der Schalter, der wirklich Rechenzeit kostet, heisst `shadow_maps_enabled`.

## Der Skript-Fahrer schreibt in **echte** Eingaben

`ButtonInput::press` und `::release` sind **oeffentlich**
(`bevy_input/src/button_input.rs:149`, `172`). Damit kann eine `--script`-Fahrt in dieselben
Eingaben schreiben, die ein Mensch ausloest — **kein zweiter, falscher Weg zu spielen**
(`prompts/init.md` §12b). Fuer die Maus gibt es `AccumulatedMouseMotion { delta: Vec2 }`
(`bevy_input/src/mouse.rs:218-221`), das die Bewegung zwischen zwei Bildern aufsummiert.

## Was der Bau auf dieser Maschine gekostet hat

`bevy = "0.19.0"` mit `default`-Features baut auf Maschine A **nicht**. Die Sammelfeatures
`2d`, `3d` und `ui` ziehen alle `default_platform` nach, und darin stecken `x11`, `wayland`
und `bevy_gilrs` fest verdrahtet (`bevy-0.19.0/Cargo.toml:2736`, `2756-2768`).

| Versuch | Ergebnis | Zeit `[debian]` |
|---|---|---|
| `bevy = "0.19.0"` (default) | Abbruch in `wayland-sys`: `wayland-client.pc` fehlt | 9m22s |
| Featureliste von Hand, aber mit `audio` | Abbruch in `alsa-sys`: `alsa.pc` fehlt | 13m40s |

Auf dieser Maschine gibt es genau drei `.pc`-Dateien (`openssl`, `libuv`, `libcrypt`) und kein
passwortloses `sudo`. Die Loesung steht in `Cargo.toml`: `default-features = false`, eine
Basisliste **ohne alles, was eine Systembibliothek zur Bauzeit braucht**, und darueber eigene
Features — `x11` (Vorgabe), `wayland`, `klang`. `cargo build --no-default-features` braucht gar
nichts.

**Merksatz:** Bei Bevy ist die Frage nie „welches Feature will ich?", sondern „was zieht dieses
Feature nach?". `cargo tree -e features` beantwortet sie, ein Blogpost nicht.

## Die Fallen aus `prompts/init.md` §3, unveraendert gueltig

- **`add_plugins((..))` nimmt maximal ~15 Elemente pro Tupel**, ein System maximal ~16
  Parameter. Beides schlaegt als unlesbarer Trait-Fehler zu. Loesung: verschachteln
  (`((A, B), C, …)`) bzw. Parameter in ein `SystemParam`-Struct buendeln.
- **Commands sind verzoegert.** Was man diesen Frame spawnt, existiert erst am Ende des
  Frames. Ein Test oder Skript, das spawnt und im selben Atemzug prueft, prueft ins Leere —
  deshalb steht im Fahrer `wait` hinter jedem `spawn`.
- **`cargo run`, NIE `./target/debug/<name>`.** Das nackte Binary sucht `assets/` relativ zum
  Arbeitsverzeichnis und findet nichts: leere Welt, keine Fehlermeldung, sieht exakt wie ein
  Render-Bug aus. (`src/data/mod.rs` faengt genau das ab und sagt es laut.)
- **Audio:** Bevys Default-Decoder ist Vorbis allein. Wer `.wav` benutzt, braucht das Feature
  `wav` — sonst laedt jeder Klang fehlerfrei und spielt **Stille**.
- **RON kennt kein `include`.** Wird eine Datendatei zu gross, splittet man sie **in Rust**
  (mehrere Dateien lesen und zusammenfuegen), nicht in RON.
- **Ohne `[profile.dev.package."*"] opt-level = 3` ist ein Debug-Build unspielbar.** Bevy
  selbst macht Batching, Transform-Propagation und Rendern; der eigene Crate bleibt auf
  `opt-level = 1` billig zu uebersetzen.

Verwandt: [`umgebung.md`](umgebung.md) · [`performance.md`](performance.md) ·
[`workflow.md`](workflow.md) · [`../architektur.md`](../architektur.md) ·
[`../konventionen.md`](../konventionen.md)
