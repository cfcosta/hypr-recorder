# Installation Guide

## Prerequisites

1. **Rust toolchain**: Install from https://rustup.rs/
2. **Hyprland**: This application is designed specifically for Hyprland on Wayland
3. **System dependencies**:
   - `libnotify` (for notifications)
   - `alsa-lib` or `pulseaudio` (for audio recording)
   - `hyprctl` (comes with Hyprland)

### On Arch Linux:
```bash
sudo pacman -S rust libnotify alsa-lib
```

### On Ubuntu/Debian:
```bash
sudo apt install rustc cargo libnotify-dev libasound2-dev
```

### On NixOS:
```nix
# Add to your configuration.nix or use nix-shell
environment.systemPackages = with pkgs; [
  rustc
  cargo
  libnotify
  alsa-lib
];
```

## Building

1. Clone or download the project
2. Run the build script:
   ```bash
   ./build.sh
   ```

Or manually:
```bash
cargo build --release
```

## Installation

### System-wide installation:
```bash
sudo cp target/release/whisper-thing /usr/local/bin/
```

### User installation:
```bash
mkdir -p ~/.local/bin
cp target/release/whisper-thing ~/.local/bin/
# Make sure ~/.local/bin is in your PATH
```

## Usage

Simply run the application:
```bash
whisper-thing
```

The application will:
1. Show a notification indicating recording has started
2. Begin recording from your default microphone
3. Display a progress bar showing recording duration
4. Wait for your input:
   - **Enter**: Save the recording
   - **Escape**: Cancel the recording
5. Automatically stop and save after 1 minute

Recordings are saved as WAV files with timestamps in the current directory.

## Troubleshooting

### "Not running under Hyprland" error
- Make sure you're running the application from within a Hyprland session
- Check that `HYPRLAND_INSTANCE_SIGNATURE` environment variable is set

### Audio recording issues
- Check that your microphone is working: `arecord -l`
- Verify audio permissions for your user
- Try running with `RUST_LOG=debug` for detailed logging

### Notification not showing
- Ensure `libnotify` is installed and working: `notify-send "test"`
- Check that a notification daemon is running

### Keybindings not working
- Verify `hyprctl` is available and working: `hyprctl version`
- Check Hyprland logs for any keybinding conflicts
- Make sure no other application is capturing global Enter/Escape keys