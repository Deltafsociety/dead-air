# DEADAIR

[English](README.md) | [فارسی](README.fa.md)

An offline-first, intranet Nostr terminal station built for digital blackouts, local mesh networks, and national intranet partitions.

Zero server infrastructure required. Zero hosting costs.

```
DEADAIR  [Alice] (ID: npub1...) [RELAY: ACTIVE :4869]  [NET: ONLINE]  [PEERS: 2]  [OUTBOX: 0]
+-- Local Timeline (12 events) -----------------------++-- Relays and Intranet Nodes ----------------+
| [14:22:01] YOU [Alice] (id: 86be7ae9..)             || [+] ws://127.0.0.1:4869 (1ms)              |
|   | Testing local intranet broadcast during blackout|| [+] wss://relay.damus.io (42ms)            |
|                                                     || [+] wss://nos.lol (38ms)                   |
| [14:22:18] * Bob (id: 4f10d291..)                   ||                                            |
|   | Received loud and clear over the mesh!          || Discovered LAN Peers:                      |
|   |                                                 ||  LAN: 192.168.1.142:4869 [Bob]             |
+-----------------------------------------------------++--------------------------------------------+
+-- Compose Broadcast Note [Kind 1] ----------------------------------------------------------------+
|  >> Type your message or /command here..._                                                        |
+---------------------------------------------------------------------------------------------------+
 [i/Enter] Write  [r] Toggle Relay  [p] Public Internet Relays  [c] Clear  [q] Quit
```

---

## Key Features

1. **Custom Handles and Nicknames (NIP-01 Metadata)**
   * Set your display handle at any time by typing `/name <your_name>`.
   * Automatically persists in `~/.config/deadair/identity.json` and publishes a signed Kind 0 metadata event to all connected relays.
   * Other stations receive the metadata event and automatically display your chosen handle on their timelines.

2. **National Intranet (NIN) and Blackout Resilience**
   * Designed specifically for scenarios where international gateways are severed, but domestic inter-city routing remains available.
   * Any computer or home connection running DEADAIR can act as a domestic relay with `[r]`.
   * Friends and contacts across different cities can connect directly via domestic IP addresses (`/relay add ws://<iran_ip>:4869`).

3. **Internet Communication via Free Public Relays**
   * Toggle global internet relays on or off by pressing `[p]` or typing `/relay public`.
   * Connects over secure WebSockets (`wss://`) to free, community-maintained global relays (`relay.damus.io`, `nos.lol`, `relay.primal.net`).
   * No accounts, no billing, no server hosting. If internet access goes down, DEADAIR automatically falls back to local intranet mode without interruption.

4. **Intranet UDP Auto-Discovery**
   * Uses UDP broadcasts on port 48690 to automatically find other DEADAIR nodes on the local Wi-Fi or LAN.
   * Eliminates the need for manual IP configuration in local environments.

5. **Persistent Offline Outbox**
   * When completely air-gapped without reachable peers, notes are queued in `~/.config/deadair/outbox.json` and automatically flushed when connectivity is restored.

---

## Commands and Shortcuts

### Terminal Commands (type into the input box and hit Enter)
* `/name <nick>`: Set your display name and broadcast your profile to the network.
* `/relay add <url>`: Connect to a custom relay (example: `/relay add wss://relay.nostr.band` or `/relay add ws://192.168.1.50:4869`).
* `/relay public`: Toggle public internet relays.
* `/help`: Show command help toast.

### Keyboard Shortcuts
| Key | Action |
| :--- | :--- |
| `i` or `Enter` | Enter compose or write mode |
| `Enter` (in compose) | Broadcast note or execute command |
| `Esc` or `Tab` | Switch between compose and normal navigation mode |
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
