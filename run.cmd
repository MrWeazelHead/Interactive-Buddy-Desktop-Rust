@echo off
cd /d "%~dp0rewrite"
if exist "target\debug\interactive-buddy.exe" (
    "target\debug\interactive-buddy.exe" %*
) else (
    cargo run --locked -- %*
)
