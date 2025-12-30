use rppal::gpio::{Gpio, Mode};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::net::{UdpSocket, SocketAddr};
use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use rosc::{OscMessage, OscPacket, OscType, encoder};

const PIN_A: u8 = 18;
const PIN_B: u8 = 24;

// OSC Configuration
const OSC_ADDRESS: &str = "/volume/fader/1";
const OSC_TARGET: &str = "192.168.1.100:9000";  // Change to your target device
const OSC_ENABLED: bool = true;

// Calibration values for normalizing potentiometer reading to 0.0-1.0
const POT_MIN: u32 = 0;
const POT_MAX: u32 = 100000;  // Adjust based on your actual readings

#[derive(Clone)]
struct PotentiometerReader {
    gpio: Arc<Gpio>,
}

#[derive(Serialize)]
struct PotReading {
    value: u32,
    timestamp: u64,
}

struct OscSender {
    socket: UdpSocket,
    target: SocketAddr,
}

impl OscSender {
    fn new(target: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let target: SocketAddr = target.parse()?;
        Ok(Self { socket, target })
    }

    fn send_value(&self, address: &str, value: f32) -> Result<(), Box<dyn std::error::Error>> {
        let msg = OscMessage {
            addr: address.to_string(),
            args: vec![OscType::Float(value)],
        };
        let packet = OscPacket::Message(msg);
        let buf = encoder::encode(&packet)?;
        self.socket.send_to(&buf, self.target)?;
        Ok(())
    }
}

fn normalize_value(raw: u32, min: u32, max: u32) -> f32 {
    if max <= min {
        return 0.0;
    }
    let clamped = raw.clamp(min, max);
    ((clamped - min) as f32) / ((max - min) as f32)
}

impl PotentiometerReader {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let gpio = Gpio::new()?;
        Ok(Self {
            gpio: Arc::new(gpio),
        })
    }

    fn discharge(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut pin_a = self.gpio.get(PIN_A)?.into_input();
        let mut pin_b = self.gpio.get(PIN_B)?.into_output();
        pin_b.set_low();
        thread::sleep(Duration::from_millis(4));
        Ok(())
    }

    fn charge_time(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let mut pin_b = self.gpio.get(PIN_B)?.into_input();
        let mut pin_a = self.gpio.get(PIN_A)?.into_output();
        pin_a.set_high();

        let mut count: u32 = 0;
        // Timeout after reasonable count to prevent infinite loop
        let max_count: u32 = 1_000_000;

        while pin_b.is_low() && count < max_count {
            count += 1;
        }

        Ok(count)
    }

    fn analog_read(&self) -> Result<u32, Box<dyn std::error::Error>> {
        self.discharge()?;
        self.charge_time()
    }
}

// Shared state for the HTTP server
struct AppState {
    last_reading: Arc<Mutex<u32>>,
}

async fn get_potentiometer(State(state): State<Arc<AppState>>) -> Json<PotReading> {
    let value = *state.last_reading.lock().unwrap();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Json(PotReading { value, timestamp })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting GPIO Potentiometer Reader (Rust)");
    println!("Pins: A={}, B={}", PIN_A, PIN_B);

    // Initialize OSC sender
    let osc_sender = if OSC_ENABLED {
        match OscSender::new(OSC_TARGET) {
            Ok(sender) => {
                println!("OSC enabled: sending to {} on address {}", OSC_TARGET, OSC_ADDRESS);
                Some(sender)
            }
            Err(e) => {
                eprintln!("Warning: Could not initialize OSC sender: {}", e);
                println!("Continuing without OSC support");
                None
            }
        }
    } else {
        println!("OSC disabled in configuration");
        None
    };

    let reader = PotentiometerReader::new()?;
    let last_reading = Arc::new(Mutex::new(0u32));
    let last_reading_clone = Arc::clone(&last_reading);

    // Background task to continuously read the potentiometer
    tokio::spawn(async move {
        loop {
            match reader.analog_read() {
                Ok(value) => {
                    let normalized = normalize_value(value, POT_MIN, POT_MAX);
                    println!("Potentiometer: raw={}, normalized={:.3}", value, normalized);
                    *last_reading_clone.lock().unwrap() = value;

                    // Send OSC message
                    if let Some(ref sender) = osc_sender {
                        if let Err(e) = sender.send_value(OSC_ADDRESS, normalized) {
                            eprintln!("OSC send error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading potentiometer: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // HTTP server
    let app_state = Arc::new(AppState { last_reading });

    let app = Router::new()
        .route("/potentiometer", get(get_potentiometer))
        .route("/health", get(|| async { "OK" }))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("HTTP server listening on http://0.0.0.0:3000");
    println!("Endpoints:");
    println!("  GET /potentiometer - Get current potentiometer reading");
    println!("  GET /health        - Health check");

    axum::serve(listener, app).await?;

    Ok(())
}
