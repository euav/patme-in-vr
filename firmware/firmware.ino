#include <BLEDevice.h>
#include <BLEUtils.h>
#include <BLEServer.h>

#define DEEP_SLEEP_TIMEOUT 65535
#define BAS_SERVICE_UUID   "180F"
#define BAS_LEVEL_UUID     "2A19"

// https://www.uuidgenerator.net/
#define PAT_SERVICE_UUID   "ab96bc38-67c5-44a5-94bf-3146bf493198"
#define PAT_HAPTIC_UUID    "5db0ca73-7963-492d-8a9c-40bb6b84c2f0"
#define PAT_NUMBER_UUID    "c90776b3-8369-42c2-a17c-8583f6b57abf"

const int resolution = 12;
const uint8_t restart_pin = 0;
const uint8_t battery_pin = 1;
const uint8_t n_haptics = 2;
const uint8_t pins[n_haptics] = {2, 3};

bool is_connected = false;
uint32_t disconnected_at = 0;
BLECharacteristic* battery_characteristic = nullptr;

RTC_DATA_ATTR float strength[n_haptics] = {0};
RTC_DATA_ATTR float received[n_haptics] = {0};
RTC_DATA_ATTR uint32_t updated_at = 0;


class HapticCallback : public BLECharacteristicCallbacks {
  void onWrite(BLECharacteristic *characteristic) {
    if (characteristic->getValue().length() < n_haptics * sizeof(float)) return;
    const float *data = reinterpret_cast<const float*>(characteristic->getData());
    for (int idx = 0; idx < n_haptics; idx++) {
      if (data[idx] != data[idx]) received[idx] = 0.0f;  // NaN check
      else received[idx] = constrain(data[idx], 0.0f, 1.0f);
    }
    updated_at = micros();
  }
};

class ConnectionCallbacks: public BLEServerCallbacks {
    void onConnect(BLEServer* server) {
        is_connected = true;
    }
    void onDisconnect(BLEServer* server) {
        is_connected = false;
        disconnected_at = micros();
    }
};


void setup_ble() {
  BLEDevice::init("PatMe-in-VR");
  auto server = BLEDevice::createServer();

  auto pat = server->createService(PAT_SERVICE_UUID);
  auto haptic = pat->createCharacteristic(PAT_HAPTIC_UUID, BLECharacteristic::PROPERTY_WRITE);
  auto number = pat->createCharacteristic(PAT_NUMBER_UUID, BLECharacteristic::PROPERTY_READ);
  server->setCallbacks(new ConnectionCallbacks());
  haptic->setCallbacks(new HapticCallback());
  number->setValue(n_haptics);
  pat->start();

  auto bas = server->createService(BAS_SERVICE_UUID);
  battery_characteristic = bas->createCharacteristic(BAS_LEVEL_UUID, BLECharacteristic::PROPERTY_READ);
  bas->start();

  auto advertising = BLEDevice::getAdvertising();
  advertising->addServiceUUID(PAT_SERVICE_UUID);
  advertising->setMinPreferred(0x06);
  advertising->setMaxPreferred(0x0C);
  advertising->start(65 * 1000);
}

void setup() {
  setup_ble();
  pinMode(restart_pin, INPUT);
  pinMode(battery_pin, INPUT);
  for (int idx = 0; idx < n_haptics; idx++) {
    pinMode(pins[idx], OUTPUT);
    analogWriteResolution(pins[idx], resolution);
    digitalWrite(pins[idx], LOW);
  }

  esp_sleep_enable_ext1_wakeup(1ULL << restart_pin, ESP_EXT1_WAKEUP_ANY_HIGH);
}

float elapsed_since(uint32_t instant) {
  return float(micros() - instant) / 1000.0f / 1000.0f;
}

void update_haptic_levels() {
  const float decay = 1.2;
  const float steep = 4.5;
  const float speed = 18.0;
  const float lowest = 0.42;

  float elapsed = elapsed_since(updated_at);
  float factor = constrain((exp(steep * (decay - elapsed)) - 1.0f) / (exp(steep) - 1.0f), 0.0f, 1.0f);
  for (int idx = 0; idx < n_haptics; idx++) {
    float increment = constrain(elapsed * speed, 0, 1) * (factor * received[idx] - strength[idx]);
    strength[idx] = constrain(strength[idx] + increment, 0.0f, 1.0f);
    float cropped = strength[idx] ? lowest + (1.0f - lowest) * strength[idx] : 0.0f;
    analogWrite(pins[idx], (int)(((1UL << resolution) - 1) * cropped));
  }
}

uint8_t battery_level(float voltage) {
    /* SlimeVR Code is placed under the MIT license
     * Copyright (c) 2020 Eiren Rain and SlimeVR Contributors
     * https://github.com/SlimeVR/SlimeVR-Tracker-ESP/blob/main/src/batterymonitor.cpp
     */

    float level;
    if (voltage > 3.975f) {
      level = (voltage - 2.920f) * 0.8f;
    } else if (voltage > 3.678f) {
      level = (voltage - 3.300f) * 1.25f;
    } else if (voltage > 3.489f) {
      level = (voltage - 3.400f) * 1.7f;
    } else if (voltage > 3.360f) {
      level = (voltage - 3.300f) * 0.8f;
    } else {
      level = (voltage - 3.200f) * 0.3f;
    }

    return constrain(uint8_t(100.0f * (level - 0.05f) / 0.95f), 0, 100);
}

void update_battery_level() {
  const float voltage_divider = 2.0f;
  const float exp_window = 0.3f;
  static float voltage = 0.0f;

  float reading = voltage_divider * (float)analogReadMilliVolts(battery_pin) / 1000.0f;
  voltage = exp_window * reading + (1.0f - exp_window) * voltage;
  battery_characteristic->setValue(battery_level(voltage));
}

void loop() {
  if (!is_connected && elapsed_since(disconnected_at) > 65.0f) {
    BLEDevice::deinit(true);
    esp_deep_sleep_start();
  }

  if (digitalRead(restart_pin) == HIGH) {
    BLEDevice::deinit(true);
    setup_ble();
  }

  update_haptic_levels();
  update_battery_level();
}
