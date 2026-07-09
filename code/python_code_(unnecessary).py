import requests
import serial
import time
import csv

# --- è¨­å®šå€ ---
PHONE_IP = '192.168.1.155'
SERIAL_PORT = 'COM3'
BAUD_RATE = 9600
INTERVAL = 0.02   # 100 Hz

# HTTP session
session = requests.Session()

# CSV æª”å
filename = time.strftime("experiment_%Y%m%d_%H%M%S.csv")

# é–‹å•Ÿ CSV
csvfile = open(filename, "w", newline="")
writer = csv.writer(csvfile)

# è¡¨é ­
writer.writerow(["time", "accX", "accY", "accZ", "current_mA"])

# Serial åˆå§‹åŒ–
try:
    ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=0.02)
    time.sleep(2)
    print("Serial connected")
except Exception as e:
    print("Serial error:", e)
    ser = None


def get_phyphox_data():
    url = f"http://{PHONE_IP}/get?accX&accY&accZ"

    try:
        r = session.get(url, timeout=0.2)

        if r.status_code == 200:
            data = r.json()

            ax = data['buffer']['accX']['buffer'][-1]
            ay = data['buffer']['accY']['buffer'][-1]
            az = data['buffer']['accZ']['buffer'][-1]

            return ax, ay, az
    except:
        pass

    return None, None, None


print("Start recording:", filename)

start_global = time.time()
counter = 0

try:

    while True:

        start_loop = time.time()

        ax, ay, az = get_phyphox_data()

        current_val = None
        if ser and ser.in_waiting:
            current_val = ser.readline().decode('utf-8', errors='ignore').strip()

        t = time.time() - start_global

        writer.writerow([t, ax, ay, az, current_val])

        counter += 1

        # æ¯200ç­† flush ä¸€æ¬¡
        if counter % 200 == 0:
            csvfile.flush()

        elapsed = time.time() - start_loop
        sleep_time = max(0, INTERVAL - elapsed)
        time.sleep(sleep_time)

except KeyboardInterrupt:
    print("Stopped")

finally:
    csvfile.flush()
    csvfile.close()

    if ser:
        ser.close()

    session.close()
