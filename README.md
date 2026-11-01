# deadair

An offline-first, intranet Nostr station and terminal dashboard built for digital blackouts, local mesh networks, and air-gapped environments.

Zero server infrastructure required. Zero hosting costs.

```
⚡ DEADAIR  Intranet & Offline Nostr Station (ID: npub1...) [● RELAY: ACTIVE :4869]  [PEERS: 2]  [OUTBOX: 0]
┌─ Local Timeline (12 events) ───────────────────────┐┌─ Relays & Intranet Nodes ────────────────┐
│ [14:22:01] YOU (npub1...) (id: 86be7ae9..)         ││ ● ws://127.0.0.1:4869 (1ms)              │
│   │ Testing local intranet broadcast during blackout││ ● ws://192.168.1.142:4869 (8ms)          │
│                                                     ││                                          │
│ [14:22:18] ◈ a1f9..02d1 (id: 4f10d291..)           ││ Discovered LAN Peers:                    │
│   │ Node 2 online. Mesh relay linked.               ││  📡 192.168.1.142:4869 [deadair-node]    │
│   │ All local transmissions verified.               ││                                          │
└─────────────────────────────────────────────────────┘└──────────────────────────────────────────┘
┌─ Compose Broadcast Note [Kind 1] ───────────────────────────────────────────────────────────────┐
│  ✏  Type your message here...█                                                                  │
└─────────────────────────────────────────────────────────────────────────────────────────────────┘
 [i/Enter] Write  [r] Toggle Relay  [c] Clear Feed  [Tab] Switch Focus  [q] Quit
```

---

## Architecture & How It Works in Blackouts

1. **Self-Sovereign Identity Without Internet**
   * Keypairs are generated and stored locally in `~/.config/deadair/identity.json`.
   * No central sign-up, email, phone number, or domain verification.
   * Every note is signed using Schnorr signatures over `secp256k1`.

2. **Embedded NIP-01 Relay (`0.0.0.0:4869`)**
   * Each `deadair` node runs an internal WebSocket Nostr relay.
   * Other devices or third-party Nostr clients on the local network (Wi-Fi, Ethernet, ad-hoc hotspot) can connect directly to `ws://<your-ip>:4869`.
   * The relay can be toggled in real time with `[r]`.

3. **Intranet UDP Peer Auto-Discovery (`:48690`)**
   * Uses UDP broadcast on port `48690`.
   * Discovers other `deadair` nodes on the same subnet without manual IP configuration.
   * Automatically connects discovered nodes to the client pool to form an ad-hoc local gossip network.

4. **Persistent Offline Outbox**
   * If completely air-gapped with no reachable relays, transmissions are saved to `~/.config/deadair/outbox.json`.
   * The moment an intranet peer or embedded relay becomes reachable, all pending outbox notes are flushed and gossiped.

---

## Keyboard Controls

| Key | Action |
| :--- | :--- |
| `i` or `Enter` | Enter compose/write mode |
| `Enter` (in compose) | Broadcast note across all connected relays |
| `Esc` or `Tab` | Switch between compose and normal navigation mode |
| `r` or `F2` | Toggle embedded relay on / off (`0.0.0.0:4869`) |
| `c` | Clear local timeline display |
| `q` or `Ctrl+C` | Quit station |

---

## Building & Running

### Requirements
* Rust 1.80+ (edition 2024 supported)

### Development Run
```bash
cargo run
```

### Production Build
```bash
cargo build --release
./target/release/deadair
```
