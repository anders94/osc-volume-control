# GPIO Potentiometer Reader (Rust)

Industrial-grade potentiometer reader for Raspberry Pi Compute Module 5 with network API.

## Features

- RC timing method for reading 10k potentiometer via GPIO
- Compiled binary for production reliability
- **OSC (Open Sound Control)** protocol support for AES67 audio systems
- **Logarithmic volume curve** - natural audio taper matching human perception
- **Rate limiting** with separate up/down slew rates - prevents sudden volume spikes and potentiometer noise
- HTTP REST API for network access
- Continuous background sampling
- Thread-safe shared state
- Automatic value normalization with dB display

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

## Rate Limiting (Slew Rate Control)

Rate limiting prevents sudden volume changes and filters out noisy potentiometer readings. This is critical for professional audio to avoid pops, clicks, and startling volume spikes.

### How It Works

The potentiometer reading is treated as a **target** value. The system outputs an **actual** value that smoothly "seeks" toward the target at controlled rates.

- **Upward rate** (conservative): Prevents sudden loud increases
- **Downward rate** (aggressive): Allows quick volume drops for safety

### Configuration

Edit `src/main.rs`:

```rust
const MAX_RATE_UP: f32 = 0.05;      // 5% per second (conservative)
const MAX_RATE_DOWN: f32 = 0.30;    // 30% per second (aggressive)
const RATE_LIMITING_ENABLED: bool = true;
```

### Example Behavior

**Scenario 1: Sudden spike to 100%**
```
Target jumps: 0.20 → 1.0
Actual rises smoothly: 0.20 → 0.25 → 0.30 → 0.35... (over 16 seconds)
```

**Scenario 2: Emergency volume cut**
```
Target drops: 0.80 → 0.0
Actual drops quickly: 0.80 → 0.50 → 0.20 → 0.0 (over 2.7 seconds)
```

**Scenario 3: Noisy potentiometer**
```
Target jitters: 0.50 ↔ 0.52 ↔ 0.49 ↔ 0.51
Actual stays smooth: 0.50 → 0.501 → 0.502... (filters out noise)
```

### Recommended Settings

| Use Case | MAX_RATE_UP | MAX_RATE_DOWN | Rationale |
|----------|-------------|---------------|-----------|
| **Broadcast** | 0.05 (5%/s) | 0.30 (30%/s) | Smooth on-air transitions, fast cuts |
| **Live Sound** | 0.10 (10%/s) | 0.50 (50%/s) | More responsive, still safe |
| **Installation** | 0.03 (3%/s) | 0.20 (20%/s) | Very smooth, elegant changes |
| **Studio** | 0.08 (8%/s) | 0.40 (40%/s) | Balanced control |

### Disabling Rate Limiting

Set `RATE_LIMITING_ENABLED = false` for direct potentiometer control (not recommended for production).

## Volume Curves

Volume curves determine how the linear potentiometer position maps to audio output. This is critical for natural-feeling audio control.

### Why Volume Curves Matter

A linear potentiometer with linear mapping feels wrong for audio:
- Bottom 50% of rotation: barely audible
- Top 50% of rotation: most of the usable volume range
- Not intuitive or natural

A **logarithmic curve** matches human hearing:
- Even distribution of perceived volume across the full rotation
- More control at typical listening levels
- Professional "audio taper" feel

### Available Curves

Edit `src/main.rs`:

```rust
const VOLUME_CURVE: VolumeCurve = VolumeCurve::Logarithmic;  // Recommended
```

| Curve | Description | Use Case |
|-------|-------------|----------|
| **Logarithmic** | Audio taper - matches human perception | **Recommended** - Professional audio control |
| **Linear** | Direct 1:1 mapping | Testing, non-audio applications |
| **Exponential** | Opposite of log - more control at high end | Specialized applications |

### Logarithmic Configuration

When using logarithmic curve, configure the dB range:

```rust
const DB_MIN: f32 = -60.0;  // Full attenuation (pot at 0%)
const DB_MAX: f32 = 0.0;    // Unity gain (pot at 100%)
```

**Common dB Ranges:**

| Application | DB_MIN | DB_MAX | Rationale |
|-------------|---------|---------|-----------|
| **Broadcast/Studio** | -60.0 | 0.0 | Standard professional range |
| **Live Sound** | -90.0 | +10.0 | Extended range for variety of sources |
| **Installation** | -40.0 | 0.0 | Limited range for simpler control |
| **Mastering** | -20.0 | 0.0 | Narrow range for fine adjustment |

### How Logarithmic Works

The logarithmic curve converts linear pot position through dB space:

1. **Linear position** (0.0-1.0) → **dB value** (e.g., -60 to 0 dB)
2. **dB value** → **Linear amplitude** using: `amplitude = 10^(dB/20)`
3. Result is normalized to 0.0-1.0 for OSC output

**Example with -60 to 0 dB range:**

| Pot Position | dB | Linear Output | Perceived Volume |
|--------------|-----|---------------|------------------|
| 0% | -60 dB | 0.001 | Silent |
| 25% | -45 dB | 0.017 | Quiet |
| 50% | -30 dB | 0.089 | Moderate |
| 75% | -15 dB | 0.447 | Loud |
| 100% | 0 dB | 1.000 | Unity/Full |

Notice how the linear output is distributed more evenly across the pot rotation compared to simple linear mapping.

### Console Output

With logarithmic curve enabled, you'll see dB values in the output:

```
Volume curve: Logarithmic
  dB range: -60 to 0 dB
Rate limiting enabled: up=0.05/s, down=0.3/s

Pot: raw=50000, linear=0.500, target=0.089, actual=0.089 (-30.0 dB) [rate limited]
```

This shows:
- **raw**: Raw potentiometer count
- **linear**: Normalized pot position (0.0-1.0)
- **target**: Volume after applying curve
- **actual**: Final output after rate limiting
- **dB**: Equivalent dB level for human reference

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

### Rate Limiting
```rust
const MAX_RATE_UP: f32 = 0.05;                     // Max increase: 5%/second
const MAX_RATE_DOWN: f32 = 0.30;                   // Max decrease: 30%/second
const RATE_LIMITING_ENABLED: bool = true;          // Enable/disable
```

### Volume Curve
```rust
const VOLUME_CURVE: VolumeCurve = VolumeCurve::Logarithmic;  // Linear, Logarithmic, Exponential
const DB_MIN: f32 = -60.0;                         // Minimum dB (at 0%)
const DB_MAX: f32 = 0.0;                           // Maximum dB (at 100%)
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
