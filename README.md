# Whisper-thing Audio Recorder

A Rust tool for recording audio with visual feedback and global keybindings on Hyprland/Wayland.

## Features

- **One-click recording**: Start recording immediately when launched
- **Global keybindings**: Press Enter to save or Esc to cancel from anywhere in Hyprland
- **Visual feedback**: Real-time progress notifications via swayosd
- **Auto-timeout**: Automatically saves after 1 minute
- **High-quality audio**: Records 32-bit float WAV files from default microphone
- **Timestamped files**: Saves recordings with timestamp in filename

## Requirements

- **Hyprland** window manager (required for global keybindings)
- **swayosd** for progress notifications
- **hyprctl** for Hyprland IPC communication
- Audio input device (microphone)

## Installation

```bash
cargo build --release
```

## Usage

```bash
cargo run
```

The application will:
1. Start recording immediately from your default microphone
2. Show a progress notification with elapsed time
3. Wait for your input:
   - **Enter**: Save the recording
   - **Escape**: Cancel and discard the recording
   - **Auto-save**: After 60 seconds

Recordings are saved as `recording_YYYYMMDD_HHMMSS.wav` in the current directory.

## Architecture

### Project Structure
```
src/
├── main.rs           # Entry point and main event loop
├── audio.rs          # Audio recording with cpal and hound
├── input.rs          # Hyprland global keybinding management
└── notification.rs   # swayosd progress notifications
```

### Key Dependencies
- **cpal** - Cross-platform audio I/O
- **hound** - WAV file encoding
- **tokio** - Async runtime for event loop
- **anyhow** - Error handling
- **tempfile** - Temporary file management for IPC

### Technical Details

#### Audio Recording
- Uses `cpal` to capture from the default input device
- Records 32-bit float samples at device's native sample rate
- Buffers audio in memory with atomic synchronization
- Saves as WAV using `hound` with proper audio specifications

#### Global Keybindings
- Registers temporary Hyprland keybindings via `hyprctl --batch`
- Uses temporary files for IPC communication between processes
- Automatically cleans up bindings on exit or error
- Polls file system for key events (50ms intervals)

#### Progress Notifications
- Integrates with swayosd for consistent Wayland notifications
- Updates progress bar every 500ms to show recording duration
- Shows completion status (saved/cancelled) with appropriate icons
- Throttled updates for performance

#### Event Loop
- Async coordination using `tokio::select!`
- Handles audio recording, progress updates, and key input concurrently
- Graceful cleanup on all exit paths
