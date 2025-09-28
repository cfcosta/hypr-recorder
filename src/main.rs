mod audio;
mod error;
mod input;
mod notification;

use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};

use audio::AudioRecorder;
use input::{KeyAction, KeyHandler};
use notification::RecordingNotification;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};

pub use crate::error::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    info!("Starting Whisper-thing Audio Recorder");

    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        error!(
            "This application requires Hyprland. Please run it under Hyprland."
        );

        return Err(Error::HyprlandNotRunning);
    }

    let mut recorder = AudioRecorder::new()?;

    let mut notification = RecordingNotification::show()?;

    let mut key_handler = KeyHandler::new().await?;

    if let Err(e) = key_handler.register_bindings().await {
        error!("Failed to register keybindings: {}", e);
        return Err(e);
    }

    recorder.start_recording().await?;

    info!("Recording started. Press Enter to save, Esc to cancel.");

    let mut progress_interval = interval(Duration::from_millis(50));
    let start_time = Instant::now();
    let mut last_update = Instant::now();

    let result = loop {
        tokio::select! {
            _ = progress_interval.tick() => {
                let elapsed = start_time.elapsed();

                if elapsed >= Duration::from_secs(60) {
                    info!("Recording reached 1-minute limit, auto-saving");
                    break save_recording(&mut recorder, &mut notification).await;
                }

                if last_update.elapsed() >= Duration::from_millis(100) {
                    if let Err(e) = notification.update_progress(elapsed) {
                        warn!("Failed to update notification: {}", e);
                    }
                    last_update = Instant::now();
                }

                if !recorder.is_recording() {
                    info!("Recording stopped externally");
                    break save_recording(&mut recorder, &mut notification).await;
                }
            }

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

    let samples = recorder.stop_recording()?;

    if samples.is_empty() {
        warn!("No audio data recorded");
        notification.show_completed(false)?;
        return Ok(());
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("recording_{timestamp}.wav");

    let output_path = env::home_dir()
        .map(|d| d.join("Recordings"))
        .or(env::current_dir().ok())
        .unwrap_or(
            env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp")),
        )
        .join(&filename);

    recorder.save_to_file(&samples, &output_path)?;

    info!("Recording saved to: {}", output_path.display());

    notification.show_completed(true)?;

    sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn cancel_recording(
    recorder: &mut AudioRecorder,
    notification: &mut RecordingNotification,
) -> Result<()> {
    info!("Cancelling recording...");

    let _ = recorder.stop_recording();

    notification.show_completed(false)?;

    sleep(Duration::from_secs(1)).await;

    Ok(())
}
