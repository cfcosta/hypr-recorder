# Whisper-thing Capture

PipeWire-based audio + screen recorder with visual feedback and Hyprland-friendly controls.

## Features

- **Immediate capture**: Starts recording the active monitor and microphone as soon as it launches
- **Global keybindings**: Press Enter to save or Esc to cancel from any Hyprland workspace
- **Visual feedback**: Progress notifications via swayosd with 60-second auto-stop
- **PipeWire pipeline**: Uses the XDG desktop portal + PipeWire to capture the monitor and audio directly
- **H.264 + AAC output**: Encodes to MP4 (`capture_YYYYMMDD_HHMMSS.mp4`) ready for sharing or transcription

## Requirements

- **Hyprland** window manager (for keybinding registration)
- **swayosd** for progress notifications
- **hyprctl** for Hyprland IPC communication
- **PipeWire** with a working XDG desktop portal implementation (e.g. `xdg-desktop-portal-wlr`)
- **GStreamer** runtime with plugins: base/good/bad/ugly, libav (provides `x264enc` + `avenc_aac`)

## Installation

```bash
cargo build --release
```

## Usage

```bash
cargo run
```

Launching the binary will:
1. Ask the Wayland portal for monitor + audio capture permission
2. Start the PipeWire → GStreamer pipeline immediately after approval
3. Display a swayosd progress notification with elapsed time
4. Listen for global keybindings:
   - **Enter** → stop, mux to MP4, kick off Whisper transcription
   - **Escape** → stop and discard the capture
   - **Auto-save** → stop automatically at 60 seconds if you forget

Recordings are stored in `~/Recordings/capture_YYYYMMDD_HHMMSS.mp4`. A transcript (`.txt`) is written next to the MP4 when Whisper succeeds.

## Architecture

### Project Structure
```
src/
├── main.rs         # Entry point and async event loop coordination
├── recorder.rs     # PipeWire portal negotiation + GStreamer pipeline management
├── input.rs        # Hyprland global keybinding registration/polling
├── notification.rs # swayosd progress toasts
├── transcriber.rs  # Whisper CLI orchestration
└── utils.rs        # Process helpers/macros
```

### Key Dependencies
- **cpal** - Cross-platform audio I/O
- **hound** - WAV file encoding
- **tokio** - Async runtime for event loop
- **anyhow** - Error handling
- **tempfile** - Temporary file management for IPC

### Technical Details

#### Capture Pipeline
- Negotiates monitor + audio nodes through the XDG desktop portal (`ashpd`)
- Shares the PipeWire remote with two `pipewiresrc` elements (video + audio)
- Encodes video using `x264enc` and audio with `avenc_aac`
- Writes an MP4 container via `mp4mux`

#### Global Keybindings
- Registers temporary Hyprland keybindings via `hyprctl --batch`
- Uses a temporary file to communicate key presses back into the async loop
- Cleans up bindings on every exit path

#### Progress Notifications
- Integrates with swayosd for consistent Wayland notifications
- Updates progress every ~100 ms while recording is active
- Signals success/failure at the end of each session

#### Event Loop
- Coordinates capture, key input, notifications, and Whisper transcription using `tokio::select!`
- Applies a 60-second safety timeout to avoid runaway recordings
- Ensures all temporary resources are released before exit
