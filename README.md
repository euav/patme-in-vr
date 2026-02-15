# PatMe-in-VR, Haptic Feedback for VRChat

Bridges VRChat's OSC avatar parameters to a Bluetooth LE haptic device, so your friends can **pat you in VR**. The host app is written in Rust; the device firmware is an Arduino (ESP32) sketch.

Inspired by the [Patstrap](https://github.com/danielfvm/Patstrap) project by [danielfvm](https://github.com/danielfvm).


## What it does

Host app listens for OSC avatar parameters from VRChat and forwards them as
haptic intensity values over BLE to a PatMe-in-VR device.

- **Input**: OSC messages with addresses like `/avatar/parameters/haptic_0`
  (and `_1`, etc.) and a single float argument.
- **Processing**: Values are time-smoothed and compacted into a small,
  fixed-size haptic state.
- **Output**: The current haptic state is sent over BLE to the configured
  service/characteristic on the device.

## Configuration

You can configure host app via CLI flags or environment variables.

- **OSC bind address**
  - Flag: `--osc-port <PORT>`
  - Env: `PATME_OSC_PORT`
  - Default: `9001`

- **Haptics count (number of vibros)**
  - Flag: `--haptics-count <N>`
  - Env: `PATME_HAPTICS_SIZE`
  - Default: `2`

- **Send interval (ms)**
  - Flag: `--send-interval-ms <MS>`
  - Env: `PATME_SEND_INTERVAL_MS`
  - Default: `20`

- **Headless mode without GUI**
  - Flag: `--headless`

## Notes

- The host smooths incoming parameters with a decay filter and sends compacted float values to the device BLE characteristic.
- BLE service/characteristic UUIDs are defined in the firmware and matched by the host: see [firmware/firmware.ino](firmware/firmware.ino) and [src/ble.rs](src/ble.rs).
