# Changelog

## [0.2.0] Unreleased

### Added

- **GUI mode**: Optional iced-based window when not running with `--headless`.
  - Status: OSC listening, BLE connection, battery level (from device BAS service).
  - Haptic values: progress bars and a "Test" button per motor for a short test pulse.
  - Max intensity slider (0–100%) to cap haptic feedback strength.
  - Quit button.
- **Headless mode**: `--headless` flag to run without GUI (OSC → BLE bridge only).
- **BLE**: Battery service (BAS) support; report connection and battery level to the GUI.

### Changed

- BLE client refactor: `Connection` struct, `notify()` / `read_battery()` helpers, `Send + Sync` error types for use with iced's async runtime.
- Bridge runs on iced's tokio runtime when GUI is used (no extra thread).
- Compactor applies max intensity (GUI slider) as a scale to haptic output.
- Removed WiX/dist packaging (Cargo.toml metadata, `dist-workspace.toml`, `wix/`).

