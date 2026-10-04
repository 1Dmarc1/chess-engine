# Rübli

A fast, UCI-compliant chess engine written in **Rust**.

## Quick Start

Clone the repository and build with native CPU optimizations for maximum performance:

```bash
git clone https://github.com/1Dmarc1/chess-engine.git
cd ruebli
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Usage

Rübli communicates via the UCI protocol and can be loaded directly into any standard chess GUI (such as Cutechess).
