# Whisper-thing Audio Recorder

A Rust tool for recording audio with visual feedback and global keybindings on Hyprland/Wayland.

## Features
- Records from microphone until Enter (save) or Esc (cancel)
- Shows recording progress via libnotify with volume slider simulation
- 1-minute recording time limit
- Global keybindings work anywhere in Hyprland
- Real-time visual feedback during recording

## Implementation Plan

### Project Structure
```
src/
├── main.rs              # Entry point and main loop
├── audio/
│   ├── mod.rs          # Audio module exports
│   ├── recorder.rs     # Audio recording logic
│   └── format.rs       # Audio format handling
├── notification/
│   ├── mod.rs          # Notification module exports
│   └── progress.rs     # libnotify integration with progress
├── input/
│   ├── mod.rs          # Input module exports
│   └── keybind.rs      # Global keybinding detection
└── state.rs            # Application state management
```

### Dependencies
- `cpal` - Cross-platform audio I/O
- `hound` - WAV file encoding
- `notify-rust` - libnotify bindings
- `tokio` - Async runtime
- `anyhow` - Error handling

### Technical Implementation

#### Audio Recording
Uses `cpal` to capture microphone input in 16-bit PCM format at 44.1kHz. Records to memory buffer with 1-minute timeout, then saves as WAV using `hound`.

#### Notification System
Creates persistent libnotify notification with progress bar updated every 100ms to show recording duration (0-60 seconds).

#### Global Keybindings
Uses Hyprland IPC to register temporary keybindings for Enter/Esc that work globally. Communicates via Unix socket and cleans up on exit.

#### Main Loop
Async event loop coordinating audio recording, progress updates, and key input using `tokio::select!`.

## Usage
```bash
cargo run
```

Press Enter to save recording, Esc to cancel. Recording automatically stops after 1 minute.
