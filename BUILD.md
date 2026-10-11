# Building Interactive Buddy

Requires Windows x64, Rust with edition 2024 support, and the game assets.

This repository tracks rewritten Rust/Bevy source, Cargo manifests, catalog metadata and the companion asset generator. Supplied Flash art/audio, SWF/decompilation and binaries are intentionally omitted from Git history.

1. Download and extract the Windows release from GitHub Releases.
2. Copy its `assets` directory into `rewrite/assets`, merging it with the tracked catalog metadata. Use assets from the same version as the source.
3. From the repository root:

```powershell
cd rewrite
cargo test --locked
cargo build --release --locked
cargo run --release --locked -- --playtest
```

`--playtest` unlocks everything without reading or writing normal progress. `--classic` uses a contained window instead of the Windows desktop overlay.

To package the optimized executable and assets:

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-desktop.ps1 -Build -Release
```

The package is written to `dist/Interactive Buddy/`. Do not include `buddy-save.json`, logs or screenshots when publishing a ZIP. The package script supports development builds without `-Release` as well.

To regenerate the new companion sprites, install Python and Pillow and run `python tools/generate_content_assets.py` with the base assets present. `rewrite/assets/content_manifest.json` records the exact base dimensions.
