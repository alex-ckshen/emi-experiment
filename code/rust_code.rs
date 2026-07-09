use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::time::{Duration, Instant};
use chrono::Local;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, watch};
use tokio::time;

const BAUD_RATE: u32 = 115200;
const INTERVAL: u64 = 10;   // ms
const MATLAB_IP: &'static str = "127.0.0.1:5005";
const PHONE_IP: &'static str = "172.20.10.1";
const PORT_NAME: &'static str = "COM3";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // watch -> newest
    let (phyphox_tx, phyphox_rx) = watch::channel((0.0, 0.0, 0.0));
    let (serial_tx, serial_rx) = watch::channel(0.0);
    let (handle_tx, mut handle_rx) = mpsc::channel::<RawData>(100);

    // serial setting
    let port = serialport::new(PORT_NAME, BAUD_RATE)
        .timeout(Duration::from_millis(50))
        .open()?;

    // serial channel
    tokio::spawn(async move {
        let mut reader = BufReader::new(port);
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).is_ok() {
                if let Ok(val) = line.trim().parse::<f64>() {
                    let _ = serial_tx.send(val);
                }
            }
        }
    });

    // phyphox setting
    let url = format!("http://{PHONE_IP}/get?accX&accY&accZ");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(50))
        .build()?;

    // phyphox channel
    tokio::spawn(async move {
        loop {
            let phy_res = client.get(&url).send().await;
            let (x, y, z) = if let Ok(res) = phy_res {
                if let Ok(body) = res.json::<Value>().await {
                    (
                        body["buffer"]["accX"]["buffer"][0].as_f64().unwrap_or(0.0),
                        body["buffer"]["accY"]["buffer"][0].as_f64().unwrap_or(0.0),
                        body["buffer"]["accZ"]["buffer"][0].as_f64().unwrap_or(0.0),
                    )
                } else { (0.0, 0.0, 0.0) }
            } else { (0.0, 0.0, 0.0) };
            let _ = phyphox_tx.send((x, y, z));
        }
    });

    // udp setting
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    // experiment setting
    let now = Local::now();
    let filename = format!("data_{}.csv", now.format("%d%H%M%S"));

    // data handler
    tokio::spawn(async move {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(&filename)
            .expect("Failed to open file");

        let mut wtr = csv::Writer::from_writer(file);

        while let Some(data) = handle_rx.recv().await {
            // csv write
            let _ = wtr.serialize(data);
            let _ = wtr.flush();

            // udp broadcast
            if let Ok(json) = serde_json::to_string(&data) {
                let msg = format!("{}\n", json);
                if let Err(e) = socket.send_to(&msg.as_bytes(), MATLAB_IP).await {
                    eprintln!("Failed to send message: {}", e);
                }
            }
        }
    });

    println!("Start fetching data...");
    let start = Instant::now();
    // let mut x_filter = Filter::new(0.03, 0.08);
    // let mut y_filter = Filter::new(0.03, 0.08);
    // let mut z_filter = Filter::new(0.03, 0.08);
    let mut current_filter = Filter::new(0.03, 0.06);
    let mut interval = time::interval(Duration::from_millis(INTERVAL));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);  // skip delay

    loop {
        interval.tick().await;

        let current_raw = *serial_rx.borrow();
        let (x, y, z) = *phyphox_rx.borrow();
        let elapsed = (start.elapsed().as_secs_f64() * 1000.0).round() / 1000.0;

        // let x_fil = x_filter.process(x);
        // let y_fil = y_filter.process(y);
        // let z_fil = z_filter.process(z);
        let filtered = current_filter.process(current_raw);

        // let data = RawData::new(elapsed, x_fil, y_fil, z_fil, current_raw, filtered);
        let data = RawData::new(elapsed, x, y, z, current_raw, filtered);
        let _ = handle_tx.send(data).await;
    }
}

#[derive(Copy, Clone)]
struct Filter {
    alpha: f64,
    threshold: f64,
    last: f64,
}

impl Filter {
    fn new(alpha: f64, threshold: f64) -> Self {
        Self { alpha, threshold, last: 0.0 }
    }

    fn process(&mut self, data: f64) -> f64 {
        let diff = (data - self.last).abs();
        if diff <= self.threshold {
            self.last = (1.0 - self.alpha) * self.last + self.alpha * data;
        } else {
            self.last = data;
        }
        self.last
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct RawData {
    timestamp: f64,
    acc_x: f64,
    acc_y: f64,
    acc_z: f64,
    current_raw: f64,
    current_fil: f64,
}

impl RawData {
    fn new(timestamp: f64, acc_x: f64, acc_y: f64, acc_z: f64, raw: f64, filtered: f64) -> Self {
        Self { timestamp, acc_x, acc_y, acc_z, current_raw: raw, current_fil: filtered }
    }
}
