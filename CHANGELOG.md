# Changelog

## [0.2.1]

### Added

- **OSC**: Support for `PatMe/L` and `PatMe/R` parameters

### Changed

- **GUI**: Fixed "Test" buttons functionality
- **Firmware**: Increased PWM frequency beyond audible range

## [0.2.0]

### Added

- **GUI mode**: Based on iced crate
  - Status display: OSC listening, BLE connection, battery level (from device BAS service)
  - Haptic values: progress bars and a "Test" button per motor for a short test pulse
  - Max intensity slider (0–100%) to cap haptic feedback strength
  - Quit button
- **Headless mode**: `--headless` flag to run without GUI (OSC to BLE bridge only)
- **BLE**: Battery service (BAS) support; report connection status and battery level to the GUI

### Changed

- BLE client refactor: `Connection` struct, `notify()` / `read_battery()` helpers, `Send + Sync` error types for use with iced's async runtime
- Compactor applies max intensity (GUI slider) as a scale to haptic output
- Removed WiX/dist packaging (Cargo.toml metadata, `dist-workspace.toml`, `wix/`)
- Firmware: Device goes to sleep after 5 minutes of inactivity