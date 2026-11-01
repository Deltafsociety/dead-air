# Contributing to DEADAIR

[English](CONTRIBUTING.md) | [فارسی](CONTRIBUTING.fa.md)

Thank you for your interest in contributing to DEADAIR. This project provides zero-cost, offline-first, and censorship-resistant communication infrastructure for high-adversity network conditions.

---

## Architectural Principles

1. **Zero Central Dependencies**: Features must never rely on a central server, proprietary API, or single point of failure.
2. **Offline-First Resilience**: Code must fail gracefully when network routes disappear. Unsent state belongs in the local outbox.
3. **No Bloat**: Keep binary size minimal and resource overhead low.
4. **No Emojis and No Em Dashes**: Keep output clean and functional across all terminal emulators and environments.

---

## Development Setup

### Prerequisites
* Rust 1.80 or later
* Git

### Building and Testing
```bash
# Check compilation
cargo check

# Run in debug mode
cargo run

# Build optimized release binary
cargo build --release
```

---

## Submitting Pull Requests

1. Fork the repository and create your branch from `master`.
2. Ensure your code compiles with zero compiler warnings:
   ```bash
   cargo clippy
   ```
3. Maintain documentation integrity. If modifying protocol features or commands, update both English and Persian documentation (`README.md`, `README.fa.md`, `web/index.html`).
4. Avoid generic commentary in source code. Code should be clean, readable, and self-documenting.
5. Submit your pull request with a concise description of the changes and motivation.
