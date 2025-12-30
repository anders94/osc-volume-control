# GPIO Potentiometer Reader (Rust)

Industrial-grade potentiometer reader for Raspberry Pi Compute Module 5 with network API.

## Features

- RC timing method for reading 10k potentiometer via GPIO
- Compiled binary for production reliability
- HTTP REST API for network access
- Continuous background sampling
- Thread-safe shared state

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

Modify constants in `src/main.rs`:

```rust
const PIN_A: u8 = 18;  // GPIO pin A
const PIN_B: u8 = 24;  // GPIO pin B
```

Change sampling rate on line 85:
```rust
tokio::time::sleep(Duration::from_secs(1)).await;  // 1 second intervals
```

Change HTTP port on line 95:
```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
```
