# Rübli -- Work in Progress

A fast chess engine written in **Rust**.

## Quick Start

### 1. Download

1. Go to the **[Latest Release](../../releases/latest)** page.
2. Under **Assets**, download the **`full-source.zip`** file *(do not use GitHub's default "Source code (zip)" link as it lacks the actual NNUE file)*.
3. Extract the ZIP archive to your desired location.

### 2. Build the Engine
Open your terminal in the extracted directory and build with Cargo:

```bash
cargo build --release
```
The compiled executable will be available at `target/release/ruebli`.

## Usage

Rübli communicates via the UCI protocol and can be loaded directly into any standard chess GUI (such as Cutechess).
