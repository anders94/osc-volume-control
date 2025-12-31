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

// Rate limiting configuration (units per second)
// These prevent sudden volume changes and noisy potentiometer jitter
const MAX_RATE_UP: f32 = 0.05;      // Maximum increase: 0.05/sec (5% per second) - conservative
const MAX_RATE_DOWN: f32 = 0.30;    // Maximum decrease: 0.30/sec (30% per second) - aggressive
const RATE_LIMITING_ENABLED: bool = true;

// Volume curve configuration
// Determines how the linear potentiometer position maps to audio volume
#[derive(Debug, Clone, Copy, PartialEq)]
enum VolumeCurve {
    Linear,      // Direct 1:1 mapping (0.0-1.0)
    Logarithmic, // Audio taper - more control at lower volumes (most natural for audio)
    Exponential, // Inverse of log - more control at higher volumes
}

const VOLUME_CURVE: VolumeCurve = VolumeCurve::Logarithmic;

// dB range for logarithmic curve
// Minimum dB when pot is at 0 (typically -60 to -90)
// Maximum dB when pot is at 1.0 (typically 0 to +10)
const DB_MIN: f32 = -60.0;  // Full attenuation
const DB_MAX: f32 = 0.0;    // Unity gain (0 dB)

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

/// Apply volume curve to linear input (0.0-1.0) to produce audio-appropriate output
fn apply_volume_curve(linear: f32, curve: VolumeCurve, db_min: f32, db_max: f32) -> f32 {
    let linear = linear.clamp(0.0, 1.0);

    match curve {
        VolumeCurve::Linear => linear,

        VolumeCurve::Logarithmic => {
            // Convert linear position to dB, then back to linear amplitude
            // This gives the classic "audio taper" feel
            if linear <= 0.0 {
                return 0.0;
            }

            // Map linear position to dB range
            let db = db_min + (db_max - db_min) * linear;

            // Convert dB to linear amplitude: amplitude = 10^(dB/20)
            let amplitude = 10.0_f32.powf(db / 20.0);

            // Normalize to 0.0-1.0 range based on dB range
            let min_amplitude = 10.0_f32.powf(db_min / 20.0);
            let max_amplitude = 10.0_f32.powf(db_max / 20.0);

            ((amplitude - min_amplitude) / (max_amplitude - min_amplitude)).clamp(0.0, 1.0)
        }

        VolumeCurve::Exponential => {
            // Exponential curve - opposite of log
            // More resolution at the high end
            linear * linear
        }
    }
}

/// Convert linear amplitude (0.0-1.0) to dB for display purposes
fn linear_to_db(linear: f32, db_min: f32, db_max: f32) -> f32 {
    if linear <= 0.0001 {
        return db_min;
    }

    // Map the 0.0-1.0 range to dB range
    let db = db_min + (db_max - db_min) * linear;
    db
}

/// Rate limiter with separate up/down slew rates
/// The potentiometer reading is the "target", and this produces an "actual" value
/// that smoothly tracks toward the target at controlled rates
struct RateLimiter {
    current: f32,
    last_update: std::time::Instant,
    max_rate_up: f32,
    max_rate_down: f32,
}

impl RateLimiter {
    fn new(initial_value: f32, max_rate_up: f32, max_rate_down: f32) -> Self {
        Self {
            current: initial_value,
            last_update: std::time::Instant::now(),
            max_rate_up,
            max_rate_down,
        }
    }

    /// Update with a new target value and return the rate-limited actual value
    fn update(&mut self, target: f32) -> f32 {
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        // Calculate the difference
        let diff = target - self.current;

        if diff.abs() < 0.001 {
            // Close enough, just use the target to avoid slow convergence
            self.current = target;
        } else if diff > 0.0 {
            // Target is higher - moving up (use conservative rate)
            let max_change = self.max_rate_up * delta_time;
            let change = diff.min(max_change);
            self.current += change;
        } else {
            // Target is lower - moving down (use aggressive rate)
            let max_change = self.max_rate_down * delta_time;
            let change = diff.abs().min(max_change);
            self.current -= change;
        }

        // Ensure we stay in valid range
        self.current = self.current.clamp(0.0, 1.0);
        self.current
    }

    fn get_current(&self) -> f32 {
        self.current
    }
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

    // Print volume curve configuration
    println!("Volume curve: {:?}", VOLUME_CURVE);
    if VOLUME_CURVE == VolumeCurve::Logarithmic {
        println!("  dB range: {} to {} dB", DB_MIN, DB_MAX);
    }

    // Initialize rate limiter
    let mut rate_limiter = if RATE_LIMITING_ENABLED {
        println!("Rate limiting enabled: up={}/s, down={}/s", MAX_RATE_UP, MAX_RATE_DOWN);
        Some(RateLimiter::new(0.0, MAX_RATE_UP, MAX_RATE_DOWN))
    } else {
        println!("Rate limiting disabled");
        None
    };

    // Background task to continuously read the potentiometer
    tokio::spawn(async move {
        loop {
            match reader.analog_read() {
                Ok(value) => {
                    // Step 1: Normalize raw reading to 0.0-1.0 (linear)
                    let linear = normalize_value(value, POT_MIN, POT_MAX);

                    // Step 2: Apply volume curve
                    let target = apply_volume_curve(linear, VOLUME_CURVE, DB_MIN, DB_MAX);

                    // Step 3: Apply rate limiting if enabled
                    let actual = if let Some(ref mut limiter) = rate_limiter {
                        let limited = limiter.update(target);
                        let db = linear_to_db(limited, DB_MIN, DB_MAX);
                        println!(
                            "Pot: raw={}, linear={:.3}, target={:.3}, actual={:.3} ({:.1} dB) [rate limited]",
                            value, linear, target, limited, db
                        );
                        limited
                    } else {
                        let db = linear_to_db(target, DB_MIN, DB_MAX);
                        println!(
                            "Pot: raw={}, linear={:.3}, output={:.3} ({:.1} dB)",
                            value, linear, target, db
                        );
                        target
                    };

                    *last_reading_clone.lock().unwrap() = value;

                    // Send OSC message with final processed value
                    if let Some(ref sender) = osc_sender {
                        if let Err(e) = sender.send_value(OSC_ADDRESS, actual) {
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
