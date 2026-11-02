```
   ____  _________    ____  ___    ________ 
  / __ \/ ____/   |  / __ \/   |  /  _/ __ \
 / / / / __/ / /| | / / / / /| |  / // /_/ /
/ /_/ / /___/ ___ |/ /_/ / ___ |_/ // _, _/ 
/_____/_____/_/  |_/_____/_/  |_/___/_/ |_|  
```

> Offline-first Nostr station for terminal users, engineered for blackouts, mesh networks, and domestic intranet survival.

[English](README.md) | [فارسی](README.fa.md)

---

### The Problem

When international internet access is cut off during a blackout, services like Telegram, WhatsApp, Discord, and Signal go completely dark because their servers sit outside the country.

However, domestic routing between local service providers and cities often remains operational.

DEADAIR is built for that exact moment. It runs entirely inside your terminal, requires zero paid server infrastructure, and demands zero domain registrations. Any laptop or workstation running DEADAIR can act as a local or nationwide relay for friends and colleagues without sending a single packet outside domestic boundaries.

---

### Terminal Dashboard

![DEADAIR Terminal Workflow Demo](docs/images/demo.gif)

![DEADAIR Terminal Dashboard Screenshot](docs/images/dashboard.png)

---

### Practical Real-World Workflow

1. **Launch the Station**: Run `deadair` in your terminal. Your cryptographic identity is generated offline on first run.
2. **Set Your Handle**: Type `/name Alice` to choose a human-readable display name.
3. **Start the Relay**: Press `[r]` to open the embedded WebSocket relay on port `4869`.
4. **Share Your Address**: Press `[s]` or type `/share`. Copy your domestic connection string (for example `ws://5.x.x.x:4869 [Alice]`) and send it to your contacts via SMS or local chat.
5. **Connect Across Cities**: Your contacts run `/bookmark add ws://5.x.x.x:4869 Alice`. Everyone is now part of an encrypted, sovereign domestic communication channel.

---

### Core Architecture

* **Zero Central Infrastructure**: No sign-ups, no phone numbers, no passwords. Identity is a standard secp256k1 keypair stored in `~/.config/deadair/identity.json`.
* **Automatic Domestic IP Resolution**: Probes local routing to discover your routable domestic address without manual network debugging.
* **Persistent Relay Bookmarks**: Save trusted domestic nodes in `~/.config/deadair/domestic_relays.json` and reconnect all nodes instantly with `[b]`.
* **Intranet UDP Beaconing**: On local Wi-Fi or LAN subnets, stations discover one another over UDP port 48690 with zero configuration.
* **Store and Forward Outbox**: If all connections drop, notes are queued safely on disk in `~/.config/deadair/outbox.json` and broadcast automatically when a peer reconnects.
* **Global Sync on Demand**: When international connectivity returns, hit `[p]` to toggle global relays (`damus.io`, `nos.lol`, `primal.net`) and bridge notes between domestic and global networks.

---

### Quick Installation

#### One-Liner (Linux and macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/deltafsociety/dead-air/master/install.sh | bash
```

Once installed, launch from any directory:
```bash
deadair
```

#### Pre-built Binaries
Static standalone binaries are available on the [Releases page](https://github.com/deltafsociety/dead-air/releases):
* Linux (x86_64)
* Windows (x86_64)
* macOS Apple Silicon (ARM64)
* macOS Intel (x86_64)

#### Building from Source
```bash
# Direct install via Cargo (guarantees binary is tailored to your CPU)
cargo install --git https://github.com/deltafsociety/dead-air.git

# Or build from local clone
git clone https://github.com/deltafsociety/dead-air.git
cd dead-air
cargo build --release
./target/release/deadair
```

---

### Command and Shortcut Reference

#### Interactive Commands
* `/name <nick>`: Set your display handle and publish your profile.
* `/share`: Output your domestic connection string and handle.
* `/ip`: Re-run domestic network IP detection.
* `/bookmark add <url> [name]`: Bookmark a domestic node and connect immediately.
* `/bookmark remove <key>`: Remove a relay from bookmarks.
* `/bookmark connect`: Reconnect to all bookmarked relays.
* `/relay add <url>`: Connect to any WebSocket relay address.
* `/relay public`: Toggle global internet relays on or off.
* `/help`: Display inline command summary.

#### Keyboard Controls
| Key | Action |
| :--- | :--- |
| `i` or `Enter` | Enter compose mode |
| `Enter` (in compose) | Broadcast message or execute command |
| `Esc` or `Tab` | Switch between write mode and navigation mode |
| `s` | Display domestic share address |
| `b` | Reconnect all saved domestic bookmarks |
| `r` or `F2` | Toggle embedded relay (`0.0.0.0:4869`) |
| `p` | Toggle global public relays |
| `c` | Clear the timeline screen |
| `q` or `Ctrl+C` | Exit DEADAIR |

---

### License

Open source under the MIT License. Built for digital sovereignty and resilient communication.
