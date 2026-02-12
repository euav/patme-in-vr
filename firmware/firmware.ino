#include <BLEDevice.h>
#include <BLEUtils.h>
#include <BLEServer.h>

// https://www.uuidgenerator.net/
const char* SERVICE_UUID = "ab96bc38-67c5-44a5-94bf-3146bf493198";
const char* HAPTICS_UUID = "5db0ca73-7963-492d-8a9c-40bb6b84c2f0";
const char* COUNTER_UUID = "c90776b3-8369-42c2-a17c-8583f6b57abf";

const float decay = 1.2;
const float steep = 4.5;
const float speed = 18.0;
const int resolution = 12;

const uint8_t ble_init_pin = 0;
const uint8_t n_haptics = 2;
const uint8_t pins[n_haptics] = {1, 2};
const uint8_t restart_pin = 0;

RTC_DATA_ATTR float current[n_haptics] = {0};
RTC_DATA_ATTR float received[n_haptics] = {0};
RTC_DATA_ATTR uint32_t updated_at = 0;


class HapticCallback : public BLECharacteristicCallbacks {
  void onWrite(BLECharacteristic *characteristic) {
    float *data = reinterpret_cast<float*>(characteristic->getData());
    for (int idx = 0; idx < n_haptics; idx++) {
      if (data[idx] != data[idx]) received[idx] = 0.0;
      else received[idx] = constrain(data[idx], 0.0, 1.0);
    }
    updated_at = micros();
  }
};


void setup_ble() {
  BLEDevice::init("PatMe-in-VR");
  auto server = BLEDevice::createServer();
  auto service = server->createService(SERVICE_UUID);
  auto haptics = service->createCharacteristic(HAPTICS_UUID, BLECharacteristic::PROPERTY_WRITE);
  auto counter = service->createCharacteristic(COUNTER_UUID, BLECharacteristic::PROPERTY_READ);
  haptics->setCallbacks(new HapticCallback());
  counter->setValue(n_haptics);
  service->start();

  auto advertising = BLEDevice::getAdvertising();
  advertising->addServiceUUID(SERVICE_UUID);
  advertising->setScanResponse(true);
  advertising->start();
}

void setup() {
  setup_ble();

  pinMode(restart_pin, INPUT);
  for (int idx = 0; idx < n_haptics; idx++) {
    pinMode(pins[idx], OUTPUT);
    analogWriteResolution(pins[idx], resolution);
    digitalWrite(pins[idx], LOW);
  }
}

void loop() {
  if (digitalRead(restart_pin) == HIGH) {
    BLEDevice::deinit(true);
    setup_ble();
  }

  float elapsed = float(micros() - updated_at) / 1000 / 1000;
  float factor = constrain((exp(steep * (decay - elapsed)) - 1.0) / (exp(steep) - 1.0), 0, 1);
  for (int idx = 0; idx < n_haptics; idx++) {
    float delta = constrain(elapsed * speed, 0, 1) * (factor * received[idx] - current[idx]);
    current[idx] = constrain(current[idx] + delta, 0, 1);
    analogWrite(pins[idx], int((1 << resolution - 1) * current[idx]));
  }
}
