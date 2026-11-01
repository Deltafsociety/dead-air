# DEADAIR

[English](README.md) | [فارسی](README.fa.md)

An offline-first, intranet Nostr terminal station built for digital blackouts, local mesh networks, and national intranet partitions.

Zero server infrastructure required. Zero hosting costs.

```
DEADAIR  [Alice] (ID: npub1...) [RELAY: ACTIVE :4869]  [NET: INTRANET]  [PEERS: 2]  [OUTBOX: 0]
+-- Local Timeline (12 events) -----------------------++-- Relays and Intranet Nodes ----------------+
| [14:22:01] YOU [Alice] (id: 86be7ae9..)             || [+] ws://127.0.0.1:4869 (1ms)              |
|   | Testing local intranet broadcast during blackout|| [+] ws://5.234.12.80:4869 (14ms)           |
|                                                     ||                                            |
| [14:22:18] * Bob (id: 4f10d291..)                   || Discovered LAN Peers:                      |
|   | Received loud and clear over the mesh!          ||  LAN: 192.168.1.142:4869 [Bob]             |
+-----------------------------------------------------++--------------------------------------------+
+-- Compose Broadcast Note [Kind 1] ----------------------------------------------------------------+
|  >> Type your message or /command here..._                                                        |
+---------------------------------------------------------------------------------------------------+
 [i/Enter] Write  [s] Share Address  [b] Bookmarks  [r] Toggle Relay  [p] Public Relays  [q] Quit
```

---

## Key Features

1. **Automatic Domestic IP Detection and Sharing**
   * Automatically resolves your routable domestic network address.
   * Press `[s]` or type `/share` to copy your node connection string: `ws://<your_ip>:4869 [handle]`.
   * Share this address with contacts across Iran via SMS or local channels so they can connect directly to your station.

2. **Persistent Domestic Relay Bookmarks Manager**
   * Save and organize trusted domestic node addresses in `~/.config/deadair/domestic_relays.json`.
   * Add a domestic node with `/bookmark add ws://<peer_ip>:4869 [name]`.
   * Press `[b]` or type `/bookmark connect` to reconnect all saved domestic relays instantly.
   * Auto-connects to saved bookmarks on startup.

3. **Custom Handles and Nicknames (NIP-01 Metadata)**
   * Set your display handle at any time by typing `/name <your_name>`.
   * Automatically persists in `~/.config/deadair/identity.json` and publishes a signed Kind 0 metadata event to all connected relays.
   * Other stations receive the metadata event and automatically display your chosen handle on their timelines.

4. **National Intranet (NIN) and Blackout Resilience**
   * Designed specifically for scenarios where international gateways are severed, but domestic inter-city routing remains available.
   * Any computer or home connection running DEADAIR can act as a domestic relay with `[r]`.
   * Friends across different cities can connect directly via domestic IP addresses.

5. **Internet Communication via Free Public Relays**
   * Toggle global internet relays on or off by pressing `[p]` or typing `/relay public`.
   * Connects over secure WebSockets (`wss://`) to free, community-maintained global relays (`relay.damus.io`, `nos.lol`, `relay.primal.net`).
   * If internet access goes down, DEADAIR automatically falls back to local intranet mode without interruption.

6. **Intranet UDP Auto-Discovery**
   * Uses UDP broadcasts on port 48690 to automatically find other DEADAIR nodes on the local Wi-Fi or LAN.

7. **Persistent Offline Outbox**
   * When completely air-gapped without reachable peers, notes are queued in `~/.config/deadair/outbox.json` and automatically flushed when connectivity is restored.

---

## Commands and Shortcuts

### Terminal Commands (type into the input box and hit Enter)
* `/share`: Display your domestic address and handle for sharing with peers.
* `/ip`: Re-run domestic IP resolution.
* `/name <nick>`: Set your display name and broadcast your profile.
* `/bookmark add <url> [name]`: Save a domestic relay to bookmarks and connect.
* `/bookmark remove <key>`: Remove a relay from bookmarks.
* `/bookmark connect`: Reconnect to all saved domestic bookmarks.
* `/relay add <url>`: Connect to any relay (and auto-bookmark).
* `/relay public`: Toggle public internet relays.
* `/help`: Show command help toast.

### Keyboard Shortcuts
| Key | Action |
| :--- | :--- |
| `i` or `Enter` | Enter compose or write mode |
| `Enter` (in compose) | Broadcast note or execute command |
| `Esc` or `Tab` | Switch between compose and normal navigation mode |
| `s` | Show domestic share address and handle |
| `b` | Reconnect all saved domestic relay bookmarks |
| `r` or `F2` | Toggle embedded relay (`0.0.0.0:4869`) |
| `p` | Toggle public internet relays (Damus, nos.lol, Primal) |
| `c` | Clear local timeline display |
| `q` or `Ctrl+C` | Quit station |

---

## Pre-built Binaries

Pre-compiled static binaries for Linux, Windows, and macOS are available under GitHub Releases:
* Linux (x86_64)
* Windows (x86_64)
* macOS Apple Silicon (ARM64)
* macOS Intel (x86_64)

---

## Building from Source

```bash
cargo build --release
./target/release/deadair
```
