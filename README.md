# Interactive Buddy native recreation

A Rust/Bevy recreation based on the supplied local Interactive Buddy v1.02 SWF extraction. The original game is by Shock Value; the historical [Newgrounds v1.01 submission](https://www.newgrounds.com/portal/view/218014) and asset provenance are documented in [REFERENCE.md](REFERENCE.md) and [rewrite/assets/README.md](rewrite/assets/README.md). This project is a partial native port, not a claim of complete 1:1 parity; see [rewrite/PARITY.md](rewrite/PARITY.md).

<img width="1920" height="1032" alt="IB" src="https://github.com/user-attachments/assets/c6a9aad4-399b-4e3d-8be3-512aac90bcf0" />

## Download

Download **v0.2.0** from [GitHub Releases](https://github.com/MrWeazelHead/Interactive-Buddy-Desktop-Rust/releases/latest), extract the Windows x64 ZIP, and double-click **Start Buddy.vbs**. **Playtest Buddy.vbs** starts an all-unlocked session without changing normal progress.

The repository tracks rewritten code and catalog metadata, not supplied art/audio, SWF exports, decompilation, binaries or personal saves. The playable media bundle is distributed separately in the release, as with v0.1.0. For source builds, supply `rewrite/assets/` first; see [BUILD.md](BUILD.md).

## Run

On Windows, double-click root **`Buddy.vbs`** during development to start the desktop overlay without a console window; it falls back to `run.cmd` when the built executable is absent. The package script creates `dist/Interactive Buddy/` with **`Interactive Buddy.exe`**, assets, **`Start Buddy.vbs`**, and a README. Double-click **`run.cmd`** when you want a console or command-line options. The default launch is desktop mode: Buddy floats above other windows and the native window-effects system can snapshot, shatter, and minimize windows after impacts. This is an opt-in interaction with windows on your desktop; use **Restore Windows** in File, either visible **Exit** button, or **Ctrl+Alt+Q** to quit and bring back windows minimized by the game. It does not close or kill windows.

From the repository root:

```powershell
cd rewrite
cargo run --locked
```

Use `cargo run --locked -- --classic` to open the traditional contained game window instead. Desktop mode is the default; `--desktop` can be used explicitly. On Windows, desktop mode uses an always-on-top overlay pinned to the bottom-right corner: a **menu button** and, beside it, a **close button**. The close button sits outside the menu column so quitting is never a mis-click away from a section. Clicking the menu button expands a column of icon dots above it — File, Skins, Items, Modes, Settings, and Help — and clicking it again collapses them. Hovering a dot shows its name. Only the dots, the two corner buttons, and an open panel intercept clicks; the gaps between dots, the hover caption, and the status readout all pass through to the desktop. Press **Esc** to stop desktop play, or **Ctrl+Alt+Q** to quit from anywhere.

The **Settings** section has four companion switches: **Screen interactions** (arm clicking anywhere on the desktop rather than only on Buddy and the overlay), **Bottom-left overlay** (the status readout), **Mouse passthrough**, and **Sound**. All four persist across runs.

**Right-click puts the tool away**, replacing H in both desktop and classic mode. It selects Open Hand, cancels firing and dragging, and drops a held toy without throwing it. Desktop right-click is polled globally, including short clicks consumed by the tool hook, so Buddy does not need keyboard focus. Mouse passthrough overrides armed-tool capture.

Mouse passthrough hands the entire overlay back to the desktop. The menu and close buttons stay clickable while it is on, and the menu button turns amber with a crossed-out pointer so the state is visible without opening anything; clicking it also switches passthrough back off. **Ctrl+Alt+P** is the keyboard equivalent.

Use `cargo run --locked -- --demo` to unlock all catalog entries with $1,000 for the session. Demo mode does not save progress. Use `cargo run --locked -- --smoke-test` to capture automated review screenshots under `rewrite/screenshots/` and exit. In a normal session, press **F12** to save `rewrite/screenshots/manual.png`. Run the test suite with `cargo test --locked`.

Normal progress saves automatically to `rewrite/buddy-save.json`.

The native smoke check covers the room, item purchase, gun menu, skin switching, an explosion, and script execution. See [VALIDATION.md](VALIDATION.md) for the tested build and limitations.

## Companion content pack

The catalog now has **69 items, 17 built-in skins and 23 modes**, plus your custom skins. Companion-pack content:

| Type | Content | Gameplay |
|---|---|---|
| Items | Beach Balls, Anvils | Light, bouncy throws versus heavy, low-bounce drops |
| Items | Bubble Wand | Gentle bubbles improve mood and cool burns near Buddy |
| Items | Repulsor | Pushes Buddy and loose toys away from the cursor |
| Items | Confetti Cannon | Repeatable colorful bursts improve Buddy's mood |
| Modes | Wind Tunnel | Gusts push Buddy and loose toys sideways |
| Modes | Magnetic Cursor | Loose toys follow the pointer, including with Dynamic Camera |
| Modes | Party Time | Immediate and ongoing large confetti, dancing and a visible PARTY indicator |
| Skins | Robot, Cosmic, Slime | Complete six-part sprite sets |
| Items | Rubber Ducks | Lightweight bouncy toys |
| Items | Ice Wand | Cools burns, dampens motion nearby and gently improves mood |
| Items | Sparkler | Small warm particles and a gentle mood boost |
| Modes | Zero Gravity | Removes gravity from Buddy and loose toys |
| Modes | Super Bounce | Toys retain almost all their bounce on arena walls/floor |
| Modes | Glitter Trail | Moving Buddy leaves short-lived colorful sparkles |
| Skins | Neon, Ice, Ember | Three more complete six-part sprite sets |
| Items | Food Bowl, Water Bowl, Toy Basket, Pet Bed | Place/refill persistent care stations; free and owned by default |
| Items | Feather Wand, Pet Brush | Nearby play and gentle brushing build trust in Passive Pet mode |
| Modes | Bubble Shower | Continuous rising bubbles around Buddy |
| Modes | Toy Orbit | Harmless balls and ducks orbit Buddy; explosives are excluded |

### More involved sandbox tools and modes

- **Tractor Beam:** hold to draw Buddy and safe loose toys toward the cursor within a 200-unit range; release or disarm to stop. Explosives are excluded.
- **Launch Pad:** place up to three pads that launch Buddy and safe toys upward on contact; each has a cooldown and expires after 20 simulated seconds.
- **Gravity Carousel:** gravity rotates through a full circle every 20 simulated seconds, respecting configured strength/sign and Low Gravity; Zero Gravity and Gravity Shifter take priority.
- **Toy Swarm:** safe loose toys flock together using cohesion, velocity alignment and short-range separation, with capped forces.
- **Portal Gun:** click twice to make a linked pair. Buddy and loose toys teleport while preserving velocity; portals have cooldowns to prevent immediate ping-pong.
- **Grappling Hook:** hold the mouse to anchor a capped spring that pulls Buddy toward the selected point; release or disarm to detach.
- **Stasis Field:** click to place up to three temporary zones that slow Buddy and loose toys inside them.
- **Pinball Arena:** fixed bumpers launch Buddy/toys and award a runtime score, with cooldowns to avoid scoring every frame.
- **Tethered Toys:** elastic tethers connect safe loose toys to Buddy.
- **Chain Reaction:** explosions prime nearby explosive toys with staggered fuses.
- **Bazooka** now fires an aimed rocket from the cursor, with its own launcher/projectile art and a stronger collision explosion. **Missiles** remains the sky-strike tool. These retain the existing desktop-effects setting; the new portals/stasis/tethers themselves never manipulate native windows.

### Passive Pet (optional)

<img width="1920" height="1029" alt="petmode" src="https://github.com/user-attachments/assets/5960cf2c-7474-410f-baa2-c7dd938832f8" />


The floating stat box is **off by default**, including old saves. Buddy instead asks for food, water, play, washing, medicine, sleep or a litter tray when needed, honoring the existing dialogue cooldown and notice priority. Tiny **2px need lights** below his mood face appear only for unmet needs: orange food, blue water, purple play, mint hygiene, red health, yellow energy and brown toilet. Mood itself still represents trust. **Settings > Passive Pet** independently toggles the opt-in **corner Stat panel**, **Need dialogue** and **Need lights**; the full numeric care overview remains in that settings page. Old `show_stats` settings reset once; new `stat_panel` choices persist normally.

Open **Settings > Passive Pet** and turn it on. Food, water and play satisfaction fall while simulation runs; higher bars mean those needs are satisfied. Buddy walks to the corresponding care station below **45%**, uses it until **85%**, and refuses when already satisfied. Care stations and tools now live exclusively under **Items > Pet Items**, which is hidden unless Passive Pet is enabled. Select Food Bowl, Water Bowl, Toy Basket, Pet Bed or Litter Tray, close the menu, then left-click to place it on the floor. Clicking near an existing station refills it. Bowls/baskets have finite supplies; up to 16 care stations are saved in normal play. Stations are separate from loose physics toys.

Trust is Buddy's feeling toward you and drives his mood. Care, brushing and play improve trust; neglect and painful hits reduce it. Below 30 trust he keeps his distance from your pointer; above 70 he approaches and occasionally hops when his needs are met. A bed restores energy and slowly improves health; tired pets can still walk to care stations. Toilet urgency rises with simulation time. Buddy seeks a litter tray when needed, or leaves bounded floor waste if accidents are enabled. Use Poop Scoop near waste to remove it, Pet Soap near Buddy to restore hygiene, and Pet Medicine to treat reversible illness. Waste worsens hygiene; low hygiene lowers pet health, but there is no permanent illness or death. The panel offers accidents and sleep toggles as well as the care stats.

Gunshots, hard thrown-object impacts and nearby explosions immediately reduce trust, including when needs decay is disabled. The panel lets you cycle needs decay **Off/0.5x/1x/2x**, toggle autonomous seeking (nearby eating/drinking still works), shy/playful behavior and the needs display, reset needs/trust or clear stations. Needs freeze while paused, in menus, when the mode is off and while the app is closed—there is no offline penalty. Passive Pet is off by default and does not replace normal gameplay.

**Pet Cage** is a free Pet Item: click to place one cage on the floor and move Buddy inside. It confines the complete rig even during dragging, impacts, portals and rotating gravity. Care stations are moved/clamped inside so he can still eat, drink, play and sleep. His needs continue normally. Place it again to relocate it, or use **Settings > Passive Pet > Release from cage** to remove it; confinement is inactive when Passive Pet is off. The cage position is saved in ordinary play, but not in playtest.

### Custom Skin Maker

Open **Skins > Skin Maker** (also available in Settings). Name your skin, choose a palette color and drag across the **16×16** paint grid. Switch between all six body parts; Fill paints the current part. The preview uses the same generated textures as Buddy. **Save + Equip** stores your skin and adds it to the Skins list. Load and edit an existing skin or delete it. Up to eight custom skins persist in your normal save; playtest skins remain session-only. The painter preserves the original six sprite resolutions, transparency silhouettes and shading; it does not import arbitrary PNG files or edit the underlying shape.

**File > Loadout / Favorites** adds/removes the equipped tool from Favorites, assigns three quick slots, and saves up to four named presets. Favorites is an Items category; slot buttons work in either surface. Keyboard slots are **1/2/3** in classic mode and **Ctrl+Alt+1/2/3** globally on desktop. Presets store tool, skin, modes, gravity and simulation speed; saving the same name replaces it. Script mode must be turned off before saving a preset. Presets and slots persist in normal play, but are session-only in playtest.

**Settings > Physics** cycles **0.5x / 1x / 2x** simulation speed, pauses/resumes, and advances one physics frame even with the panel open. Single-step also pauses; use **.** in classic mode or **Ctrl+Alt+.** on desktop. Resume and close the panel for continuous play. Opening menus still pauses ordinary simulation, and stepping does not activate tools under a modal.

**Settings > Dialogue options** has independent Buddy dialogue, window comments and random-chatter switches, plus an **8/16/30 simulation-second** minimum gap. Random lines react to mood and active modes, or simply let Buddy hang out; gameplay notices remain independent.

Double-click **`Playtest Buddy.vbs`** for a console-free, all-unlocked desktop session, or run:

```powershell
cd rewrite
cargo run --locked -- --playtest
# Contained playtest instead:
cargo run --locked -- --classic --playtest
```

Playtest owns **every** catalog entry, starts with $1,000 and Open Hand, and starts with modes off so you can test them individually. It never writes your normal save, including when switching surfaces. Normal launches still use your normal purchases and saved options.

**C** or **File > Clear toys** removes loose objects, bullets, particles and pending window effects without resetting Buddy, purchases or mood. Settings has separate **Buddy dialogue** and **Window comments** switches in desktop and classic mode. Notices now expire even while paused.

Buddy comments on dragging, painful hits, burns, exhaustion, skin changes and the new tools. In desktop mode he also comments after a foreground app stays stable for two polls: e.g. **"Oh, you're going to Discord!"** Opening or focusing a window can trigger this; it is not a hook into app launches. Windows app names are recognized locally; unknown titles become "another window", and document names, URLs and chat titles are never repeated. Chatter defaults to a 16-second simulation-time cooldown, configurable to 8 or 30, and never replaces purchase notices, warnings or save errors. Disabling dialogue does not disable gameplay notices.

New art is generated locally by `python tools/generate_content_assets.py` (requires Pillow). All **75 PNGs** preserve their corresponding base sprite's exact dimensions and transparency; `rewrite/assets/content_manifest.json` records the comparisons. These are new companion-pack drawings/recolors, not original SWF exports. Modes use existing particle rendering and need no missing sprite placeholders. Party Time bursts immediately when enabled; close the menu to let the ongoing confetti and dancing run, since menus pause simulation. Tomatoes, basketballs, explosives and other differently shaped toys now have distinct sprites instead of unrelated baseball/grenade art; Machine Gun and Shotgun have distinct gun drawings instead of pistol lookalikes. Fireballs also have separate companion art; original exports are preserved.

## Controls

In classic mode, use the top menus to select tools and open the item, skin, and mode stores. In desktop mode, use the bottom-right menu button; choose **Screen interactions** in Settings to arm interaction beyond Buddy and the overlay. Starting to use a tool in the arena closes the panel automatically, so the menu gets out of the way when you start playing; merely moving the pointer across the desktop does not, so a panel stays reachable with hover-armed tools like the Fist. Hold the left mouse button in the arena to use the selected tool. Open Hand grabs and drags the Buddy; release to throw. Throwable objects launch with velocity based on the full drag from its starting point. Hold **Shift** to aim guns toward the Buddy; hold **Ctrl** with it to aim at the head. Press **P** to pause, **R** to reset the Buddy, and **Esc** to close a menu or panel (and disarm desktop play). The visible **Exit** buttons and **Ctrl+Alt+Q** quit the desktop app.

Buddy's mood face and a speech bubble follow him in desktop mode: purchases, cheat confirmations, script results and errors are shown next to him. Separate orange notices report the object's 40-item and bullets' 256-projectile ceilings rather than silently discarding the input, and a save failure is reported in the lower-left corner and in the File panel.

The supported keyboard cheats are entered while the game has focus: **Space, Up, Space, Up, Left, Right, Left, Right** gives $1,000; **Space, Right, Down, Left, Down, Right, Up, Left** unlocks all catalog entries; **Up, Up, Up, Space, Space, Left, Right, Up** makes the Buddy happy. Replacing the last Up with Down makes the Buddy sad, or with Space resets its emotion.

The supplied catalog and art/audio exports are documented in `rewrite/assets/asset_manifest.json`, `rewrite/assets/tools/tool_manifest.json`, and `rewrite/assets/catalog.json`. The 20 mood-face frames are copied from source sprite 673 into `rewrite/assets/mood/`. Regenerate the skin/audio and tool exports with `python tools/prepare_assets.py` and `python tools/prepare_tool_assets.py` from the repository root.

## Desktop fixes

Notices — purchases, cheat confirmations, warnings, script errors — appear in Buddy's speech bubble in **both** the desktop overlay and the classic room, and nowhere else. There used to be a banner across the top of the screen as well, which meant every notice appeared twice at once; the banner was also the *only* notice the classic room ever drew, because the bubble was desktop-only. `ellipsize` shortens an over-long plate on a word boundary, so the instruction at the end of a message survives instead of being sliced down to `Pr`.

**Blood and Gore** sprays blood when Buddy is hit hard enough. The mode had been working as written and still looked broken: it spawned five droplets of radius 1.5, which the particle renderer draws at 0.75–1.5 pixels, drifting with no gravity. It now sprays nine droplets of radius 2–4.2 along the incoming force, tilted hard downward so every droplet falls, plus a splat that stays where it landed.

**Lock to window** switches to a movable, always-on-top contained game. **Desktop mode** switches back. Progress carries over; the physics scene restarts during the switch.

Selecting a weapon or object automatically enables desktop tool use. Press **Esc** to disarm. Armed tool clicks are captured so they do not also activate controls in applications underneath. Shots and missiles reach Buddy in front of open windows. Open windows are solid to Buddy: their borders are drawn faintly and he bounces off them instead of sinking through, which also means a window dragged underneath him shoves him back out. The **close button** beside the menu button quits from anywhere, expanded or collapsed, as does **Quit Buddy** in the File section; **Ctrl+Alt+Q** quits globally.

The desktop overlay uses per-pixel Windows layered presentation, without an opaque background or version label.
