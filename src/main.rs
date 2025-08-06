mod audio;
mod input;
mod notification;

use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use audio::AudioRecorder;
use input::{HyprlandKeyHandler, KeyAction};
use notification::RecordingNotification;
use tokio::time::interval;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    info!("Starting Whisper-thing Audio Recorder");

    // Check if running under Hyprland
    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        error!(
            "This application requires Hyprland. Please run it under Hyprland."
        );
        return Err(anyhow::anyhow!("Not running under Hyprland"));
    }

    // Initialize components
    let mut recorder =
        AudioRecorder::new().context("Failed to initialize audio recorder")?;

    let mut notification =
        RecordingNotification::show().context("Failed to show notification")?;

    let mut key_handler = HyprlandKeyHandler::new()
        .await
        .context("Failed to initialize key handler")?;

    // Register global keybindings
    if let Err(e) = key_handler.register_bindings().await {
        error!("Failed to register keybindings: {}", e);
        return Err(e);
    }

    // Start recording
    recorder
        .start_recording()
        .await
        .context("Failed to start recording")?;

    info!("Recording started. Press Enter to save, Esc to cancel.");

    // Main event loop
    let mut progress_interval = interval(Duration::from_millis(100));
    let start_time = Instant::now();
    let mut last_update = Instant::now();

    let result = loop {
        tokio::select! {
            // Update progress every 100ms
            _ = progress_interval.tick() => {
                let elapsed = start_time.elapsed();

                // Auto-save after 1 minute
                if elapsed >= Duration::from_secs(60) {
                    info!("Recording reached 1-minute limit, auto-saving");
                    break save_recording(&mut recorder, &mut notification).await;
                }

                // Update notification progress (throttle to every 500ms for performance)
                if last_update.elapsed() >= Duration::from_millis(500) {
                    if let Err(e) = notification.update_progress(elapsed) {
                        warn!("Failed to update notification: {}", e);
                    }
                    last_update = Instant::now();
                }

                // Check if recording is still active
                if !recorder.is_recording() {
                    info!("Recording stopped externally");
                    break save_recording(&mut recorder, &mut notification).await;
                }
            }

            // Handle key input
            key_result = key_handler.wait_for_input() => {
                match key_result {
                    Ok(KeyAction::Save) => {
                        info!("Save key pressed");
                        break save_recording(&mut recorder, &mut notification).await;
                    }
                    Ok(KeyAction::Cancel) => {
                        info!("Cancel key pressed");
                        break cancel_recording(&mut recorder, &mut notification).await;
                    }
                    Err(e) => {
                        error!("Key handler error: {}", e);
                        break cancel_recording(&mut recorder, &mut notification).await;
                    }
                }
            }
        }
    };

    // Cleanup
    if let Err(e) = key_handler.cleanup().await {
        warn!("Failed to cleanup keybindings: {}", e);
    }

    result
}

async fn save_recording(
    recorder: &mut AudioRecorder,
    notification: &mut RecordingNotification,
) -> Result<()> {
    info!("Saving recording...");

    let samples = recorder
        .stop_recording()
        .context("Failed to stop recording")?;

    if samples.is_empty() {
        warn!("No audio data recorded");
        notification
            .show_completed(false)
            .context("Failed to show completion notification")?;
        return Ok(());
    }

    // Generate output filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("recording_{timestamp}.wav");

    // Save to current directory or home directory
    let output_path = env::current_dir()
        .unwrap_or_else(|_| {
            env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
        })
        .join(&filename);

    recorder
        .save_to_file(&samples, &output_path)
        .context("Failed to save audio file")?;

    info!("Recording saved to: {}", output_path.display());

    notification
        .show_completed(true)
        .context("Failed to show completion notification")?;

    // Keep notification visible for a moment
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn cancel_recording(
    recorder: &mut AudioRecorder,
    notification: &mut RecordingNotification,
) -> Result<()> {
    info!("Cancelling recording...");

    let _ = recorder.stop_recording();

    notification
        .show_completed(false)
        .context("Failed to show completion notification")?;

    // Keep notification visible for a moment
    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
