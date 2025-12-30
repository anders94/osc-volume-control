# OSC Configuration Examples

## Common AES67 System Configurations

### 1. Yamaha Mixing Consoles (QL/CL/TF Series)

```rust
const OSC_ADDRESS: &str = "/1/fader";              // Channel 1 fader
const OSC_TARGET: &str = "192.168.0.128:9000";     // Console IP
```

**Note**: Yamaha consoles typically listen on port 9000. Enable OSC in Setup → MIDI/HOST → OSC.

---

### 2. Behringer X32/M32

```rust
const OSC_ADDRESS: &str = "/ch/01/mix/fader";      // Channel 1 fader
const OSC_TARGET: &str = "192.168.1.100:10023";    // X32 IP (port 10023)
```

**Note**: X32 uses port 10023 for OSC. No setup required, always active.

---

### 3. Allen & Heath dLive/SQ Series

```rust
const OSC_ADDRESS: &str = "/ch/01/mix/fader";      // Channel 1 fader
const OSC_TARGET: &str = "192.168.1.150:10001";    // A&H IP (port 10001)
```

---

### 4. Reaper DAW

```rust
const OSC_ADDRESS: &str = "/track/1/volume";       // Track 1 volume
const OSC_TARGET: &str = "127.0.0.1:8000";         // Localhost (or DAW IP)
```

**Setup in Reaper**:
1. Options → Preferences → Control/OSC/web
2. Add → OSC (Open Sound Control)
3. Set Mode: "Configure device IP+local port"
4. Local listen port: 8000

---

### 5. QLab (Theatre/Playback)

```rust
const OSC_ADDRESS: &str = "/cue/selected/volume";  // Selected cue volume
const OSC_TARGET: &str = "192.168.1.200:53000";    // QLab computer
```

**Setup in QLab**:
1. Workspace Settings → OSC Controls
2. Enable "Use OSC Controls"
3. Note the port (default 53000)

---

### 6. Dante Controller / Q-SYS

```rust
const OSC_ADDRESS: &str = "/gain/master";          // Custom address
const OSC_TARGET: &str = "192.168.1.50:9000";      // Q-SYS Core IP
```

**Note**: You'll need to set up OSC receive in your Q-SYS design with matching address pattern.

---

### 7. Multiple Targets (Broadcast to Multiple Systems)

To send to multiple systems simultaneously, modify `src/main.rs`:

```rust
// Replace single target with vector
const OSC_TARGETS: &[&str] = &[
    "192.168.1.100:9000",    // Mixing console
    "192.168.1.101:8000",    // DAW
    "192.168.1.102:10023",   // Backup console
];

// Update OscSender to support multiple targets
struct OscSender {
    socket: UdpSocket,
    targets: Vec<SocketAddr>,
}

impl OscSender {
    fn new(targets: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let targets: Result<Vec<SocketAddr>, _> =
            targets.iter().map(|t| t.parse()).collect();
        Ok(Self {
            socket,
            targets: targets?
        })
    }

    fn send_value(&self, address: &str, value: f32) -> Result<(), Box<dyn std::error::Error>> {
        let msg = OscMessage {
            addr: address.to_string(),
            args: vec![OscType::Float(value)],
        };
        let packet = OscPacket::Message(msg);
        let buf = encoder::encode(&packet)?;

        for target in &self.targets {
            self.socket.send_to(&buf, target)?;
        }
        Ok(())
    }
}
```

---

## Testing with oscsend (liblo-tools)

Send a test OSC message from command line:

```bash
# Install liblo-tools
sudo apt install liblo-tools

# Send test message (volume = 0.75)
oscsend 192.168.1.100 9000 /volume/fader/1 f 0.75
```

---

## OSC Message Format Reference

Standard OSC messages for volume/fader control:

- **Float (0.0 - 1.0)**: Most common for faders/volume
  - `0.0` = -∞ dB (muted)
  - `0.75` = 0 dB (unity)
  - `1.0` = +10 dB (maximum)

- **Integer (0-1023)**: Some consoles use 10-bit resolution
- **dB value (-90.0 to +10.0)**: Direct dB control

This implementation uses **Float (0.0 - 1.0)** as it's the most widely compatible format.

---

## Troubleshooting

### No OSC messages received?

1. **Check firewall**:
   ```bash
   sudo ufw allow 9000/udp  # Allow OSC port
   ```

2. **Verify network**: Ensure Pi and target are on same subnet

3. **Test with oscdump**:
   ```bash
   oscdump 9000  # Listen on port 9000
   # Then run your application
   ```

4. **Check console logs**: Application prints OSC status on startup

5. **Verify target device OSC is enabled**: Check device documentation
