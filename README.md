# patme

OSC-to-BLE haptics bridge for the PatPatMe device.

## What it does

`patme` listens for OSC parameters (e.g. from VRChat) and forwards them as
haptic intensity values over Bluetooth Low Energy to a PatPatMe-compatible
device.

- **Input**: OSC messages with addresses like `/avatar/parameters/haptic_0`
  (and `_1`, etc.) and a single float argument.
- **Processing**: Values are time-smoothed and compacted into a small,
  fixed-size haptic state.
- **Output**: The current haptic state is sent over BLE to the configured
  service/characteristic on the device.

## Requirements

- Rust (stable)
- A working BLE adapter on the host machine
- A PatPatMe-compatible BLE device advertising the expected service/characteristic

## Running

```bash
cargo run
```

On Windows PowerShell with debug logging enabled:

```powershell
$env:RUST_LOG="patme=debug"; cargo run
```

On Windows Command Prompt:

```cmd
set RUST_LOG=patme=debug && cargo run
```

## Configuration

You can configure `patme` via CLI flags or environment variables.

- **OSC bind address**
  - Flag: `--osc-addr <ADDR>`
  - Env: `PATME_OSC_ADDR`
  - Default: `0.0.0.0:9001`

- **Filter depth**
  - Flag: `--filter-depth <N>`
  - Env: `PATME_FILTER_DEPTH`
  - Default: `6`

- **Haptics size (number of channels)**
  - Flag: `--haptics-size <N>`
  - Env: `PATME_HAPTICS_SIZE`
  - Default: `2`

- **Compaction interval (ms)**
  - Flag: `--compaction-interval-ms <MS>`
  - Env: `PATME_COMPACTION_INTERVAL_MS`
  - Default: `20`

- **Poll interval (ms)**
  - Flag: `--poll-interval-ms <MS>`
  - Env: `PATME_POLL_INTERVAL_MS`
  - Default: `5`

Examples:

```bash
cargo run -- --osc-addr 127.0.0.1:9001 --filter-depth 8
```

```powershell
$env:PATME_OSC_ADDR="0.0.0.0:9100"
$env:PATME_FILTER_DEPTH="10"
cargo run
```

## Logging

This project uses `env_logger` and the `log` crate. To see more detailed logs,
set `RUST_LOG`:

```bash
RUST_LOG=patme=debug cargo run
```

On shutdown, pressing `Ctrl+C` will trigger a graceful shutdown of the main
tasks.

