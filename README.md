# Rübli

A fast chess engine written in **Rust**.

## Quick Start

Clone the repository and build:

```bash
git clone https://github.com/1Dmarc1/chess-engine.git
cd chess-engine
cargo build --release
```
The compiled executable will be available at `target/release/ruebli`.

## Usage

Rübli communicates via the UCI protocol and can be loaded directly into any standard chess GUI (such as Cutechess).
