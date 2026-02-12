# PatMe-in-VR, Haptic Feedback for VRChat

Bridges VRChat's OSC to Bluetooth LE haptic device, so your friends can **pat you in VR**. This project consists of host app written in Rust and Arduino firmware for ESP32 board with BLE.

Based on the [Patstrap](https://github.com/danielfvm/Patstrap) project by [danielfvm](https://github.com/danielfvm).


## What it does

Host app listens for OSC avatar parameters from VRChat and forwards them as
haptic intensity values over Bluetooth LE to a PatMe-in-VR device.

- **Input**: OSC messages with addresses like `/avatar/parameters/haptic_0`
  (and `_1`, etc.) and a single float argument.
- **Processing**: Values are time-smoothed and compacted into a small,
  fixed-size haptic state.
- **Output**: The current haptic state is sent over BLE to the configured
  service/characteristic on the device.

## Configuration

You can configure host app via CLI flags or environment variables.

- **OSC bind address**
  - Flag: `--osc-addr <ADDR>`
  - Env: `PATME_OSC_ADDR`
  - Default: `0.0.0.0:9001`


- **Haptics count (number of vibros)**
  - Flag: `--haptics-count <N>`
  - Env: `PATME_HAPTICS_SIZE`
  - Default: `2`

- **Send interval (ms)**
  - Flag: `--send-interval-ms <MS>`
  - Env: `PATME_SEND_INTERVAL_MS`
  - Default: `20`
