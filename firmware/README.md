# PatMe-in-VR Firmware

Firmware for the PatMe-in-VR haptic device.  
It exposes a Bluetooth Low Energy (BLE) service that accepts haptic intensities as floats and drives one or more motors via PWM. This firmware is designed to work together with the Rust host application in the root of this repository.

## Hardware assumptions

- **MCU**: ESP32 (Arduino core), with:
  - BLE support
  - PWM-capable GPIOs for each haptic channel
  - e.g. ESP32-C3, ESP32-C6
- **Haptic channels**:
  - Total number of haptic drivers of `n_haptics`
  - Pins in `pins[]` must be PWM-capable and wired to the haptic drivers
- **Control pins**:
  - `restart_pin`: when pulled HIGH, restarts BLE for a new connection


## Building and flashing

1. Open `firmware/firmware.ino` in the Arduino IDE or PlatformIO.
2. Select an **ESP32** board that matches your hardware (e.g. "ESP32 Dev Module").
3. Ensure the **Arduino ESP32 core** and **BLE library** (`BLEDevice.h` etc.) are installed.
4. Configure the correct serial port and board settings.
5. Compile and upload.
6. The device should appear over BLE scan as `PatMe-in-VR`.
