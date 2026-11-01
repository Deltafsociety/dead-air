# deadair

An offline-first, intranet Nostr station and terminal dashboard built for digital blackouts, local mesh networks, and air-gapped environments.

Zero server infrastructure required. Zero hosting costs.

```
⚡ DEADAIR  [Alice] (ID: npub1...) [● RELAY: ACTIVE :4869]  [🌐 INTERNET ACTIVE]  [PEERS: 2]  [OUTBOX: 0]
┌─ Local Timeline (12 events) ───────────────────────┐┌─ Relays & Intranet Nodes ────────────────┐
│ [14:22:01] YOU [Alice] (id: 86be7ae9..)            ││ ● ws://127.0.0.1:4869 (1ms)              │
│   │ Testing local intranet broadcast during blackout││ ● wss://relay.damus.io (42ms)            │
│                                                     ││ ● wss://nos.lol (38ms)                   │
│ [14:22:18] ◈ Bob (id: 4f10d291..)                  ││                                          │
│   │ Received loud and clear over the mesh!          ││ Discovered LAN Peers:                    │
│   │                                                 ││  📡 192.168.1.142:4869 [Bob]             │
└─────────────────────────────────────────────────────┘└──────────────────────────────────────────┘
┌─ Compose Broadcast Note [Kind 1] ───────────────────────────────────────────────────────────────┐
│  ✏  Type your message or /command here...█                                                      │
└─────────────────────────────────────────────────────────────────────────────────────────────────┘
 [i/Enter] Write  [r] Toggle Relay  [p] Public Internet Relays  [c] Clear  [q] Quit
```

---

## Key Features

1. **Custom Handles & Nicknames (NIP-01 Metadata)**
   * Set your display handle at any time by typing `/name <your_name>`.
   * Automatically persists in `~/.config/deadair/identity.json` and publishes a signed Kind 0 metadata event to all connected relays.
   * Other stations receive the metadata event and automatically display your chosen handle on their timelines.

2. **Internet Communication via Free Public Relays**
   * Toggle global internet relays on/off by pressing **`[p]`** or typing `/relay public`.
   * Out of the box, connects over secure WebSockets (`wss://`) to free, public, decentralized Nostr relays (`relay.damus.io`, `nos.lol`, `relay.primal.net`).
   * No accounts, no billing, no server hosting. If internet access goes down, `deadair` automatically falls back to local intranet mode without interruption.

3. **Intranet & Blackout Auto-Discovery**
   * Uses UDP broadcasts on `:48690` to automatically find other `deadair` nodes on the local Wi-Fi or LAN.
   * Any computer can run the embedded relay on `0.0.0.0:4869` with `[r]`.

4. **Persistent Offline Outbox**
   * When completely air-gapped without reachable peers, notes are queued in `~/.config/deadair/outbox.json` and automatically flushed when connectivity is restored.

---

## Commands & Shortcuts

### Terminal Commands (type into the input box and hit Enter)
* `/name <nick>` — Set your display name and broadcast your profile to the network.
* `/relay add <url>` — Connect to a custom relay (e.g., `/relay add wss://relay.nostr.band` or `/relay add ws://192.168.1.50:4869`).
* `/relay public` — Toggle public internet relays.
* `/help` — Show command help toast.

### Keyboard Shortcuts
| Key | Action |
| :--- | :--- |
| `i` or `Enter` | Enter compose/write mode |
| `Enter` (in compose) | Broadcast note / execute command |
| `Esc` or `Tab` | Switch between compose and normal navigation mode |
| `r` or `F2` | Toggle embedded relay (`0.0.0.0:4869`) |
| `p` | Toggle public internet relays (Damus, nos.lol, Primal) |
| `c` | Clear local timeline display |
| `q` or `Ctrl+C` | Quit station |

---

## Building & Running

```bash
cargo build --release
./target/release/deadair
```
