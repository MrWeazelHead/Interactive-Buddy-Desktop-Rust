# Interactive Buddy native recreation

A Rust/Bevy recreation based on the supplied local Interactive Buddy v1.02 SWF extraction. The original game is by Shock Value; the historical [Newgrounds v1.01 submission](https://www.newgrounds.com/portal/view/218014) and asset provenance are documented in [REFERENCE.md](REFERENCE.md) and [rewrite/assets/README.md](rewrite/assets/README.md). This project is a partial native port, not a claim of complete 1:1 parity; see [rewrite/PARITY.md](rewrite/PARITY.md).

<img width="1920" height="1032" alt="IB" src="https://github.com/user-attachments/assets/c6a9aad4-399b-4e3d-8be3-512aac90bcf0" />

## Run

On Windows, double-click root **`Buddy.vbs`** during development to start the desktop overlay without a console window; it falls back to `run.cmd` when the built executable is absent. The package script creates `dist/Interactive Buddy/` with **`Interactive Buddy.exe`**, assets, **`Start Buddy.vbs`**, and a README. Double-click **`run.cmd`** when you want a console or command-line options. The default launch is desktop mode: Buddy floats above other windows and the native window-effects system can snapshot, shatter, and minimize windows after impacts. This is an opt-in interaction with windows on your desktop; use **Restore Windows** in File, either visible **Exit** button, or **Ctrl+Alt+Q** to quit and bring back windows minimized by the game. It does not close or kill windows.

From the repository root:

```powershell
cd rewrite
cargo run --locked
```

Use `cargo run --locked -- --classic` to open the traditional contained game window instead. Desktop mode is the default; `--desktop` can be used explicitly. On Windows, desktop mode uses an always-on-top overlay, with a narrow vertical menu at the lower-right and its selected options panel above and to the left of Buddy. Click the active menu name again to collapse its options while leaving the menu strip visible. The collapsed Buddy tab has an adjacent **Exit** button; the expanded menu also has **Exit**. **Play anywhere** arms desktop interaction so tools can be used across the desktop; in normal mode, clicks are limited to Buddy and the overlay UI. Press **Esc** to stop desktop play, or **Ctrl+Alt+Q** to quit from anywhere.

Use `cargo run --locked -- --demo` to unlock all catalog entries with $1,000 for the session. Demo mode does not save progress. Use `cargo run --locked -- --smoke-test` to capture automated review screenshots under `rewrite/screenshots/` and exit. In a normal session, press **F12** to save `rewrite/screenshots/manual.png`. Run the test suite with `cargo test --locked`.

Normal progress saves automatically to `rewrite/buddy-save.json`.

The native smoke check covers the room, item purchase, gun menu, skin switching, an explosion, and script execution. See [VALIDATION.md](VALIDATION.md) for the tested build and limitations.

## Controls

In classic mode, use the top menus to select tools and open the item, skin, and mode stores. In desktop mode, use the lower-right File, Skins, Items, Modes, Settings, and Help strip; choose **Play anywhere** in Settings to arm interaction beyond Buddy and the overlay. Hold the left mouse button in the arena to use the selected tool. Open Hand grabs and drags the Buddy; release to throw. Throwable objects launch with velocity based on the full drag from its starting point. Hold **Shift** to aim guns toward the Buddy; hold **Ctrl** with it to aim at the head. Press **P** to pause, **R** to reset the Buddy, and **Esc** to close a menu or panel (and disarm desktop play). The visible **Exit** buttons and **Ctrl+Alt+Q** quit the desktop app.

The supported keyboard cheats are entered while the game has focus: **Space, Up, Space, Up, Left, Right, Left, Right** gives $1,000; **Space, Right, Down, Left, Down, Right, Up, Left** unlocks all catalog entries; **Up, Up, Up, Space, Space, Left, Right, Up** makes the Buddy happy. Replacing the last Up with Down makes the Buddy sad, or with Space resets its emotion.

The supplied catalog and art/audio exports are documented in `rewrite/assets/asset_manifest.json`, `rewrite/assets/tools/tool_manifest.json`, and `rewrite/assets/catalog.json`. The 20 mood-face frames are copied from source sprite 673 into `rewrite/assets/mood/`. Regenerate the skin/audio and tool exports with `python tools/prepare_assets.py` and `python tools/prepare_tool_assets.py` from the repository root.

## Desktop fixes

**Lock to window** switches to a movable, always-on-top contained game. **Desktop mode** switches back. Progress carries over; the physics scene restarts during the switch.

Selecting a weapon or object automatically enables desktop tool use. Press **Esc** to disarm. Armed tool clicks are captured so they do not also activate controls in applications underneath. Shots and missiles reach Buddy in front of open windows. **Exit** remains accessible beside the collapsed menu and in the expanded menu; **Ctrl+Alt+Q** quits globally.

The desktop overlay uses per-pixel Windows layered presentation, without an opaque background or version label.
