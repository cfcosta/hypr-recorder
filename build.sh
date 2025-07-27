#!/bin/bash

# Build script for whisper-thing
# This script assumes Rust is installed on the system

echo "Building whisper-thing audio recorder..."

# Check if cargo is available
if ! command -v cargo &>/dev/null; then
  echo "Error: Cargo not found. Please install Rust:"
  echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi

# Build the project
echo "Running cargo check..."
cargo check

if [ $? -eq 0 ]; then
  echo "✅ Code compiles successfully"
  echo "Running cargo build --release..."
  cargo build --release

  if [ $? -eq 0 ]; then
    echo "✅ Build completed successfully"
    echo "Binary location: target/release/whisper-thing"
    echo ""
    echo "To install system-wide:"
    echo "sudo cp target/release/whisper-thing /usr/local/bin/"
    echo ""
    echo "To run:"
    echo "./target/release/whisper-thing"
  else
    echo "❌ Build failed"
    exit 1
  fi
else
  echo "❌ Code has compilation errors"
  exit 1
fi
