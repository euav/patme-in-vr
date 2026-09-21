# PatMe in VR firmware for ESP32

Firmware for the haptic device. It pairs with the host application, receives intensity values, and drives vibration motors. It requires a Bluetooth LE-compatible board, such as an ESP32-C3 or ESP32-C6.

## Pinout

| Pin | Role |
|-----|------|
| 0   | Restart / wake (INPUT) — pull HIGH to re-init BLE or wake from deep sleep |
| 1   | Battery voltage sense (INPUT) — via divider to ADC |
| 2, 3 | Haptic outputs (PWM, 12-bit) — connect to motor drivers |

Adjust `pins[]` and `n_haptics` in `firmware.ino` if your wiring differs.

## BLE protocol

- **Service** `ab96bc38-67c5-44a5-94bf-3146bf493198`
  - **Haptic (write)** `5db0ca73-7963-492d-8a9c-40bb6b84c2f0`  
    Host writes `n_haptics` × 4 bytes: little-endian `float32` per channel, 0.0 = off, 1.0 = full.
  - **Number (read)** `c90776b3-8369-42c2-a17c-8583f6b57abf`  
    Single byte: number of haptic channels (e.g. `2`).
- **Battery** standard BAS (0x180F / 0x2A19), percentage 0–100.

## Building and flashing

1. Install the [Arduino IDE](https://www.arduino.cc/en/software/).
2. Install [ESP32 Arduino core](https://docs.espressif.com/projects/arduino-esp32/).
3. Open `firmware.ino` in the Arduino IDE (or add `firmware/` as a sketch).
4. Select your ESP32 board and corresponding COM port, then select **Upload**.
5. The device should appear over BLE scan as `PatMe-in-VR`.

## Behavior

- On disconnect, device enters deep sleep after ~65 s (advertising timeout) to save battery.
- Pull GPIO 0 HIGH to wake or to re-initialize BLE without power cycle.
- Incoming haptic values are smoothed and mapped to PWM with a minimum threshold so motors don’t stall at low duty.
