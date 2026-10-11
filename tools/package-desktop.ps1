param([switch]$Build, [switch]$Release)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$crateRoot = Join-Path $projectRoot 'rewrite'
if ($Build) {
    $buildArgs = @('build', '--manifest-path', (Join-Path $crateRoot 'Cargo.toml'), '--locked')
    if ($Release) { $buildArgs += '--release' }
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed.' }
}
$profile = if ($Release) { 'release' } else { 'debug' }
$binary = Join-Path $crateRoot "target\$profile\interactive-buddy.exe"
if (-not (Test-Path -LiteralPath $binary)) { throw 'Build the application first.' }
$destination = Join-Path $projectRoot 'dist\Interactive Buddy'
New-Item -ItemType Directory -Path $destination -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $destination 'Interactive Buddy.exe') -Force
Copy-Item -LiteralPath (Join-Path $crateRoot 'assets') -Destination $destination -Recurse -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'REFERENCE.md') -Destination $destination -Force
(Get-Content -LiteralPath (Join-Path $crateRoot 'PARITY.md') -Raw).Replace('../REFERENCE.md', 'REFERENCE.md') |
    Set-Content -LiteralPath (Join-Path $destination 'PARITY.md') -Encoding utf8
@'
# Interactive Buddy Desktop

Double-click **Start Buddy.vbs** to launch. Keep the executable and assets folder together.

Buddy appears above ordinary windows, beside the primary monitor's taskbar. The compact lower-right menu opens File, Skins, Items, Modes, Settings and Help. Ctrl+Alt+Q quits globally and restores affected windows.

Use Open Hand to drag and throw Buddy. Select weapons in Items; buy more using Shop tools. Right-click disables the current tool; Esc also disarms desktop interactions. C clears toys. P pauses, R resets Buddy, and 1/2/3 select assigned quick slots. Favorites, named presets and simulation speed/single-step controls are available in the menus.

The catalog has 69 items, 17 built-in skins and 23 modes, plus a built-in custom Skin Maker. Try Portal Gun, Grappling Hook, Tractor Beam, Launch Pad, Gravity Carousel and Toy Swarm. Buddy dialogue and local app-switch comments have separate toggles.

Settings > Passive Pet enables optional food, water, play, sleep, hygiene, illness and toilet care. Items > Pet Items appears only while it is enabled. Place bowls, toys, beds and litter trays on the floor, clean waste with Poop Scoop, or use Pet Cage to keep Buddy confined. Release him in Passive Pet settings. Needs use dialogue and small colored lights; the corner stat panel is opt-in.

Screen interactions and window effects are opt-in settings. Armed tool clicks are captured instead of clicking applications underneath. Window effects can shake, snapshot, shatter and minimize windows, but never close or kill applications. File > Restore Windows brings them back; quitting also restores them.

Switch to windowed mode in File for a contained, always-on-top game; switch back to desktop mode there. Progress carries over, while the physics scene restarts. Desktop mode currently supports the primary monitor.

Progress saves to buddy-save.json in this folder when using the launcher. Run `"Interactive Buddy.exe" --classic` for the contained game. Double-click **Playtest Buddy.vbs** for an all-unlocked, $1,000 session that does not save or change your normal progress.

This Rust/Bevy application uses original Interactive Buddy art and audio by Shock Value, extracted from the supplied v1.02 SWF. It remains a partial recreation: see PARITY.md for remaining differences and REFERENCE.md for sources. Desktop mode currently supports the primary monitor.
'@ | Set-Content -LiteralPath (Join-Path $destination 'README.md') -Encoding utf8
@'
Option Explicit
Dim shell, files, root
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
shell.CurrentDirectory = root
shell.Run Chr(34) & root & "\Interactive Buddy.exe" & Chr(34), 0, False
'@ | Set-Content -LiteralPath (Join-Path $destination 'Start Buddy.vbs') -Encoding ascii
@'
Option Explicit
Dim shell, files, root
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
shell.CurrentDirectory = root
shell.Run Chr(34) & root & "\Interactive Buddy.exe" & Chr(34) & " --playtest", 0, False
'@ | Set-Content -LiteralPath (Join-Path $destination 'Playtest Buddy.vbs') -Encoding ascii
Write-Output $destination
