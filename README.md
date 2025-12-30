# GPIO Potentiometer Reader (Rust)

Industrial-grade potentiometer reader for Raspberry Pi Compute Module 5 with network API.

## Features

- RC timing method for reading 10k potentiometer via GPIO
- Compiled binary for production reliability
- **OSC (Open Sound Control)** protocol support for AES67 audio systems
- HTTP REST API for network access
- Continuous background sampling
- Thread-safe shared state
- Automatic value normalization (0.0 - 1.0) for standard audio control

## Hardware Setup

- **Pin A (GPIO 18)**: Connected to one side of potentiometer
- **Pin B (GPIO 24)**: Connected to potentiometer wiper
- **Ground**: Connected to other side of potentiometer

## Building

On the Raspberry Pi CM5:

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build release binary
cargo build --release

# Binary will be at: target/release/gpio-potentiometer
```

## Running

```bash
# Run directly
cargo run --release

# Or run the compiled binary
./target/release/gpio-potentiometer
```

The application will:
1. Start reading the potentiometer every second (printed to stdout)
2. Start an HTTP server on port 3000

## API Endpoints

### Get Potentiometer Reading
```bash
curl http://localhost:3000/potentiometer
```

Response:
```json
{
  "value": 12345,
  "timestamp": 1735567890
}
```

### Health Check
```bash
curl http://localhost:3000/health
```

Response: `OK`

## OSC (Open Sound Control) Integration

The application sends OSC messages automatically to integrate with AES67 audio systems, mixing consoles, and DAWs.

### OSC Configuration

Edit `src/main.rs` to configure OSC:

```rust
const OSC_ADDRESS: &str = "/volume/fader/1";       // OSC address pattern
const OSC_TARGET: &str = "192.168.1.100:9000";     // Target IP:Port
const OSC_ENABLED: bool = true;                     // Enable/disable OSC

// Calibration - adjust based on actual potentiometer readings
const POT_MIN: u32 = 0;
const POT_MAX: u32 = 100000;
```

### How It Works

1. Potentiometer is read every second
2. Raw value is normalized to 0.0 - 1.0 range (standard for audio faders)
3. OSC message sent to configured target with format: `/volume/fader/1 0.753`

### OSC Address Patterns

Common patterns for audio control:

- `/volume/fader/1` - Main volume control (recommended)
- `/1/fader` - Channel 1 fader
- `/master/volume` - Master volume
- `/mix/main/level` - Main mix level

Choose the pattern that matches your target system.

### Compatible Systems

Works with any OSC-capable system:
- Mixing consoles (Yamaha, Behringer, Allen & Heath, etc.)
- DAWs (Reaper, Ableton Live, Pro Tools, etc.)
- Audio routing software (Dante Controller, Q-SYS, etc.)
- Custom AES67 control applications

### Testing OSC Output

Monitor OSC messages on Linux/macOS:
```bash
# Install oscdump (from liblo-tools)
sudo apt install liblo-tools  # Debian/Ubuntu
brew install liblo            # macOS

# Listen for OSC messages on port 9000
oscdump 9000
```

Or use any OSC monitor application (e.g., OSCulator, OSC Monitor, etc.)

### Calibration

1. Run the application and observe the raw values:
   ```
   Potentiometer: raw=5432, normalized=0.054
   ```

2. Turn potentiometer to minimum and note the raw value
3. Turn potentiometer to maximum and note the raw value
4. Update `POT_MIN` and `POT_MAX` in `src/main.rs`
5. Rebuild: `cargo build --release`

### Multiple OSC Targets

To send to multiple targets, modify the `OscSender` in `src/main.rs` to hold multiple sockets. Example:

```rust
// Add multiple targets
const OSC_TARGETS: &[&str] = &[
    "192.168.1.100:9000",  // Mixing console
    "192.168.1.101:8000",  // DAW
];
```

## Network Access

To access from another machine on your network:

```bash
curl http://<raspberry-pi-ip>:3000/potentiometer
```

## Production Deployment

### Systemd Service

Create `/etc/systemd/system/gpio-potentiometer.service`:

```ini
[Unit]
Description=GPIO Potentiometer Reader
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/home/pi/gpio-potentiometer
ExecStart=/home/pi/gpio-potentiometer/target/release/gpio-potentiometer
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable gpio-potentiometer
sudo systemctl start gpio-potentiometer
sudo systemctl status gpio-potentiometer
```

## Advantages over Python

- **Performance**: Compiled binary, ~100x faster execution
- **Memory Safety**: Rust prevents common bugs (null pointers, race conditions)
- **Reliability**: No interpreter, no runtime errors from missing dependencies
- **Small Footprint**: Single statically-linked binary
- **Industrial Grade**: Type safety and error handling built into language

## Configuration

All configuration is done via constants in `src/main.rs`:

### GPIO Pins
```rust
const PIN_A: u8 = 18;  // GPIO pin A
const PIN_B: u8 = 24;  // GPIO pin B
```

### OSC Settings
```rust
const OSC_ADDRESS: &str = "/volume/fader/1";       // OSC address pattern
const OSC_TARGET: &str = "192.168.1.100:9000";     // Target IP:Port
const OSC_ENABLED: bool = true;                     // Enable/disable OSC
const POT_MIN: u32 = 0;                             // Calibration min
const POT_MAX: u32 = 100000;                        // Calibration max
```

### Sampling Rate
Change the interval in the background task:
```rust
tokio::time::sleep(Duration::from_secs(1)).await;  // 1 second intervals
```

### HTTP Port
```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
```

After making changes, rebuild:
```bash
cargo build --release
```
