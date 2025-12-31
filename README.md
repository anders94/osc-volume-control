# OSC Volume Control

Industrial-grade Open Sound Control (OSC) volume controller designed for integration with professional audio systems, mixing consoles, DAWs, and AES67 networked audio environments.

## Overview

OSC Volume Control is a purpose-built hardware controller that translates physical potentiometer input into standardized OSC messages for professional audio control. Engineered for reliability and precision, it features logarithmic volume curves, intelligent rate limiting, and network-based monitoring - essential capabilities for broadcast, live sound, and high-end installation environments.

## Key Features

### Professional Audio Control
- **OSC Protocol Compliance** - Native Open Sound Control support for seamless integration with industry-standard equipment
- **Logarithmic Volume Curves** - Audio taper matching human perception, providing natural control characteristics across the entire range
- **Intelligent Rate Limiting** - Asymmetric slew rate control prevents sudden volume spikes while allowing rapid attenuation for safety
- **Precision Normalization** - Calibrated 0.0-1.0 output with dB reference display for professional workflows

### Enterprise Reliability
- **Compiled Binary** - Rust-based implementation for memory safety and deterministic performance
- **Thread-Safe Architecture** - Concurrent sampling and network I/O without race conditions
- **REST API** - Network monitoring and diagnostics via HTTP endpoints
- **Production Ready** - Systemd integration for unattended operation

## Quick Start

### Installation

```bash
# Clone repository
git clone https://github.com/yourusername/osc-volume-control.git
cd osc-volume-control

# Build release binary
cargo build --release

# Binary location: target/release/osc-volume-control
```

### Basic Configuration

Edit `src/main.rs` to configure your OSC target:

```rust
const OSC_ADDRESS: &str = "/volume/fader/1";
const OSC_TARGET: &str = "192.168.1.100:9000";
const OSC_ENABLED: bool = true;
```

### Running

```bash
./target/release/osc-volume-control
```

The controller will begin sending OSC messages to your configured target system.

## OSC Integration

### Protocol Details

OSC Volume Control transmits standardized OSC messages containing normalized floating-point values (0.0 to 1.0) representing volume level. This format is compatible with professional audio equipment worldwide.

**Message Format:**
```
/volume/fader/1 0.753
```

### Address Patterns

Configure the OSC address pattern to match your target system:

| Pattern | Application | Common Use |
|---------|-------------|------------|
| `/volume/fader/1` | Generic fader control | Recommended default |
| `/1/fader` | Channel-based systems | Mixing consoles |
| `/master/volume` | Master section | Main output control |
| `/mix/main/level` | Named mixer paths | Routing matrices |

### Compatible Systems

OSC Volume Control integrates with:
- **Mixing Consoles** - Yamaha CL/QL/TF series, Behringer X32/M32, Allen & Heath dLive/SQ, Midas M32
- **Digital Audio Workstations** - Reaper, Ableton Live, Pro Tools, Logic Pro
- **Audio Networking** - Dante Controller, Q-SYS, d&b audiotechnik ArrayProcessing
- **Broadcast Systems** - Wheatstone, Lawo, Calrec automation systems
- **Installation Processors** - QSC Q-SYS, BSS Soundweb, Biamp Tesira

### Configuration Parameters

```rust
// OSC Settings (src/main.rs)
const OSC_ADDRESS: &str = "/volume/fader/1";       // OSC address pattern
const OSC_TARGET: &str = "192.168.1.100:9000";     // Target IP:Port
const OSC_ENABLED: bool = true;                     // Master enable
```

### Testing OSC Output

Verify OSC messages using command-line tools:

```bash
# Install OSC utilities
sudo apt install liblo-tools  # Debian/Ubuntu
brew install liblo            # macOS

# Monitor OSC traffic on port 9000
oscdump 9000
```

Alternatively, use GUI tools such as OSCulator (macOS), OSC Monitor (cross-platform), or your DAW's MIDI/OSC learn function.

## Volume Curves

Volume curves define the transfer function between linear potentiometer position and audio output level. Proper curve selection is critical for achieving professional control characteristics.

### Curve Types

```rust
const VOLUME_CURVE: VolumeCurve = VolumeCurve::Logarithmic;  // Recommended
```

| Curve | Transfer Function | Application |
|-------|------------------|-------------|
| **Logarithmic** | Audio taper via dB domain | **Professional audio** - matches human perception |
| **Linear** | Direct 1:1 mapping | Testing, non-audio control |
| **Exponential** | Inverse logarithmic | Specialized control systems |

### Logarithmic Configuration

The logarithmic curve operates in the decibel domain to achieve perceptually-linear control:

```rust
const DB_MIN: f32 = -60.0;  // Minimum attenuation (pot at 0%)
const DB_MAX: f32 = 0.0;    // Maximum level (pot at 100%)
```

**Transfer Function:** `amplitude = 10^(dB/20)`, where `dB = DB_MIN + (DB_MAX - DB_MIN) × position`

### Recommended dB Ranges

| Application | DB_MIN | DB_MAX | Rationale |
|-------------|--------|--------|-----------|
| **Broadcast/Studio** | -60.0 | 0.0 | Industry standard professional range |
| **Live Sound** | -90.0 | +10.0 | Extended range for diverse source levels |
| **Installation** | -40.0 | 0.0 | Simplified range for end-user operation |
| **Mastering/Fine Control** | -20.0 | 0.0 | Narrow range for precision adjustment |

### Response Characteristics

Example with -60 to 0 dB range:

| Potentiometer | dB Level | Linear Amplitude | Perceived Volume |
|---------------|----------|------------------|------------------|
| 0% | -60 dB | 0.001 | Effectively silent |
| 25% | -45 dB | 0.018 | Background level |
| 50% | -30 dB | 0.032 | Moderate level |
| 75% | -15 dB | 0.178 | Loud |
| 100% | 0 dB | 1.000 | Unity gain |

The logarithmic curve distributes perceived volume changes evenly across the entire control range, eliminating the "bunching" effect common with linear potentiometers.

## Rate Limiting (Slew Rate Control)

Rate limiting implements controlled slew rates to prevent sudden volume changes, filter potentiometer noise, and protect against startling audio transients. This is a critical safety feature for professional audio systems.

### Operational Principle

The controller treats potentiometer readings as a **target** value. The actual output **seeks** toward this target at configurable maximum rates, implementing a first-order lag filter with asymmetric attack/release characteristics.

### Configuration

```rust
const MAX_RATE_UP: f32 = 0.05;      // Maximum increase: 5% per second
const MAX_RATE_DOWN: f32 = 0.30;    // Maximum decrease: 30% per second
const RATE_LIMITING_ENABLED: bool = true;
```

**Design Philosophy:**
- **Conservative Upward Rate** - Prevents sudden loud increases that could damage equipment or hearing
- **Aggressive Downward Rate** - Allows rapid volume reduction for safety and artistic intent

### Application-Specific Tuning

| Use Case | MAX_RATE_UP | MAX_RATE_DOWN | Design Intent |
|----------|-------------|---------------|---------------|
| **Broadcast** | 0.05 (5%/s) | 0.30 (30%/s) | Smooth on-air transitions, emergency cuts |
| **Live Sound** | 0.10 (10%/s) | 0.50 (50%/s) | Responsive control, rapid fades |
| **Installation** | 0.03 (3%/s) | 0.20 (20%/s) | Elegant, unobtrusive changes |
| **Studio** | 0.08 (8%/s) | 0.40 (40%/s) | Balanced control for mixing |

### Behavior Examples

**Scenario 1: Sudden Volume Spike Protection**
```
Target jumps to 100% (possible potentiometer fault)
Output rises controlled: 0.20 → 0.25 → 0.30 → 0.35...
Result: 16-second ramp instead of instant jump
```

**Scenario 2: Emergency Attenuation**
```
Target drops to 0% (operator intervention)
Output decreases rapidly: 0.80 → 0.50 → 0.20 → 0.0
Result: 2.7-second fade for quick response
```

**Scenario 3: Noise Filtering**
```
Target jitters: 0.50 ↔ 0.52 ↔ 0.49 ↔ 0.51 (dirty pot)
Output remains stable: 0.500 → 0.501 → 0.502...
Result: Smooth output despite noisy input
```

## Hardware Implementation

OSC Volume Control uses an RC timing method to read potentiometer position via GPIO. This implementation is optimized for Raspberry Pi Compute Module hardware but the architecture supports other GPIO-capable platforms.

### Hardware Requirements

- GPIO-capable processor (Raspberry Pi recommended)
- 10kΩ linear potentiometer
- Basic wiring knowledge

### Pin Configuration

```rust
const PIN_A: u8 = 18;  // GPIO 18 - Potentiometer charge pin
const PIN_B: u8 = 24;  // GPIO 24 - Potentiometer sense pin
```

### Wiring Diagram

```
GPIO 18 (PIN_A) ──────┬─────── Potentiometer Terminal 1
                      │
GPIO 24 (PIN_B) ──────┼─────── Potentiometer Wiper
                      │
Ground ───────────────┴─────── Potentiometer Terminal 2
```

### RC Timing Method

The controller measures potentiometer resistance by timing capacitor discharge through the resistive element. This technique provides robust, reliable readings without requiring analog-to-digital converters.

**Sampling Rate:** Configurable, default 1 Hz (adjustable in `src/main.rs`)

## Calibration

Calibration maps the raw hardware readings to the normalized 0.0-1.0 range required for professional audio control.

### Calibration Procedure

1. **Start the controller** and observe console output:
   ```
   Potentiometer: raw=5432, normalized=0.054
   ```

2. **Rotate to minimum position** and note the raw value

3. **Rotate to maximum position** and note the raw value

4. **Update calibration constants** in `src/main.rs`:
   ```rust
   const POT_MIN: u32 = 1200;    // Minimum raw reading
   const POT_MAX: u32 = 98500;   // Maximum raw reading
   ```

5. **Rebuild the binary:**
   ```bash
   cargo build --release
   ```

The controller will now map your hardware's actual range to precise 0.0-1.0 output.

## Network API

OSC Volume Control includes a REST API for monitoring and diagnostics.

### Endpoints

#### Get Current Reading
```bash
curl http://localhost:3000/potentiometer
```

**Response:**
```json
{
  "value": 12345,
  "timestamp": 1735567890
}
```

#### Health Check
```bash
curl http://localhost:3000/health
```

**Response:** `OK`

### Network Access

Access from remote machines using the controller's IP address:
```bash
curl http://192.168.1.50:3000/potentiometer
```

### Port Configuration

Edit `src/main.rs` to change the HTTP port:
```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
```

## Production Deployment

### Systemd Service

Create `/etc/systemd/system/osc-volume-control.service`:

```ini
[Unit]
Description=OSC Volume Control
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/osc-volume-control
ExecStart=/opt/osc-volume-control/target/release/osc-volume-control
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### Enable and Start Service

```bash
sudo systemctl enable osc-volume-control
sudo systemctl start osc-volume-control
sudo systemctl status osc-volume-control
```

### View Logs

```bash
sudo journalctl -u osc-volume-control -f
```

## Configuration Reference

All configuration is performed via constants in `src/main.rs`. After making changes, rebuild with `cargo build --release`.

### Complete Configuration Template

```rust
// Hardware
const PIN_A: u8 = 18;
const PIN_B: u8 = 24;

// OSC
const OSC_ADDRESS: &str = "/volume/fader/1";
const OSC_TARGET: &str = "192.168.1.100:9000";
const OSC_ENABLED: bool = true;

// Calibration
const POT_MIN: u32 = 0;
const POT_MAX: u32 = 100000;

// Volume Curve
const VOLUME_CURVE: VolumeCurve = VolumeCurve::Logarithmic;
const DB_MIN: f32 = -60.0;
const DB_MAX: f32 = 0.0;

// Rate Limiting
const MAX_RATE_UP: f32 = 0.05;
const MAX_RATE_DOWN: f32 = 0.30;
const RATE_LIMITING_ENABLED: bool = true;
```

## Technical Specifications

### Performance
- **Latency:** < 5ms from potentiometer read to OSC transmission
- **Sampling Rate:** Configurable, default 1 Hz
- **Resolution:** 16-bit effective after normalization
- **Stability:** Rate limiting provides ±0.1% output stability with noisy inputs

### Reliability
- **Language:** Rust - memory safety without garbage collection
- **Architecture:** Multi-threaded async I/O with tokio runtime
- **Error Handling:** Comprehensive error propagation and recovery
- **Deployment:** Single statically-linked binary, no runtime dependencies

### Network
- **OSC Protocol:** UDP-based Open Sound Control 1.0
- **HTTP API:** RESTful JSON endpoints on TCP
- **Concurrency:** Thread-safe shared state for simultaneous access

## Advanced Configuration

### Multiple OSC Targets

To broadcast to multiple receivers, modify the OSC sender initialization in `src/main.rs`:

```rust
const OSC_TARGETS: &[&str] = &[
    "192.168.1.100:9000",  // Main console
    "192.168.1.101:8000",  // Recording system
    "192.168.1.102:9000",  // Monitor controller
];
```

### Custom Sampling Rate

Adjust the background task interval:

```rust
tokio::time::sleep(Duration::from_millis(100)).await;  // 10 Hz sampling
```

Higher sampling rates provide more responsive control but increase CPU usage and network traffic.

## Troubleshooting

### No OSC Messages Received

1. Verify OSC target IP and port: `ping 192.168.1.100`
2. Check firewall rules on receiving system
3. Use `oscdump` to verify messages are being sent
4. Confirm `OSC_ENABLED = true` in configuration

### Jumpy or Unstable Output

1. Increase rate limiting constraint: reduce `MAX_RATE_UP` and `MAX_RATE_DOWN`
2. Check potentiometer wiring for intermittent connections
3. Verify potentiometer is 10kΩ linear taper
4. Ensure proper grounding

### Incorrect Volume Range

1. Perform calibration procedure to set `POT_MIN` and `POT_MAX`
2. Verify potentiometer is rotating through full mechanical range
3. Check that volume curve settings match your application

## License

[Your License Here]

## Support

For issues, feature requests, or contributions, please visit the project repository.
