#include <Adafruit_INA219.h>

Adafruit_INA219 ina219;

void setup() {
  Serial.begin(115200); // INA219 通常建議用高速率

  if (!ina219.begin()) {
    Serial.println("無法找到 INA219,請檢查接線!");
    while (1);
  }

  Serial.println("INA219 測量中...");
}

void loop() {
  float busVoltage_V = 0;
  float current_mA = 0;
  float power_mW = 0;

  // 直接呼叫函式讀取數值，不用再寫複雜的電壓轉換公式
  busVoltage_V = ina219.getBusVoltage_V();      // 讀取電壓
  current_mA = ina219.getCurrent_mA();          // 讀取電流 (mA)
  power_mW = ina219.getPower_mW();              // 讀取功率 (mW)

  // 顯示結果
  Serial.print("電壓: "); Serial.print(busVoltage_V); Serial.print(" V | ");
  Serial.print("電流: "); Serial.print(current_mA); Serial.print(" mA | ");
  Serial.print("功率: "); Serial.print(power_mW); Serial.println(" mW");

  delay(500);
}