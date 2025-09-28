mod error;
mod input;
mod notification;
mod recorder;
mod transcriber;
mod utils;

use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};

use input::{Action, Input};
use notification::Notification;
use recorder::Recorder;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};
use transcriber::Transcriber;

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

    let mut recorder = Recorder::new()?;

    let mut notification = Notification::show()?;

    let mut key_handler = Input::new().await?;

    let transcriber = Transcriber::new();

    if let Err(e) = key_handler.register().await {
        error!("Failed to register keybindings: {}", e);
        return Err(e);
    }

    recorder.start().await?;

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
                    if let Err(e) = key_handler.cleanup().await {
                        warn!("Failed to cleanup keybindings before auto-save: {}", e);
                    }
                    break save_recording(&mut recorder, &mut notification, &transcriber)
                        .await;
                }

                if last_update.elapsed() >= Duration::from_millis(100) {
                    if let Err(e) = notification.update(elapsed) {
                        warn!("Failed to update notification: {}", e);
                    }
                    last_update = Instant::now();
                }

                if !recorder.is_recording() {
                    info!("Recording stopped externally");
                    if let Err(e) = key_handler.cleanup().await {
                        warn!(
                            "Failed to cleanup keybindings before external stop save: {}",
                            e
                        );
                    }
                    break save_recording(&mut recorder, &mut notification, &transcriber)
                        .await;
                }
            }

            key_result = key_handler.wait_for_input() => {
                match key_result {
                    Ok(Action::Save) => {
                        info!("Save key pressed");
                        if let Err(e) = key_handler.cleanup().await {
                            warn!(
                                "Failed to cleanup keybindings before manual save: {}",
                                e
                            );
                        }
                        break save_recording(&mut recorder, &mut notification, &transcriber)
                            .await;
                    }
                    Ok(Action::Cancel) => {
                        info!("Cancel key pressed");
                        if let Err(e) = key_handler.cleanup().await {
                            warn!(
                                "Failed to cleanup keybindings before cancel: {}",
                                e
                            );
                        }
                        break cancel_recording(&mut recorder, &mut notification).await;
                    }
                    Err(e) => {
                        error!("Key handler error: {}", e);
                        if let Err(cleanup_err) = key_handler.cleanup().await {
                            warn!(
                                "Failed to cleanup keybindings after error: {}",
                                cleanup_err
                            );
                        }
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
    recorder: &mut Recorder,
    notification: &mut Notification,
    transcriber: &Transcriber,
) -> Result<()> {
    info!("Saving recording...");

    let samples = recorder.stop()?;

    if samples.is_empty() {
        warn!("No audio data recorded");
        notification.complete(false)?;
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

    recorder.save(&samples, &output_path)?;

    info!("Recording saved to: {}", output_path.display());

    let transcript_path = match transcriber.start(&output_path).await {
        Ok(path) => path,
        Err(e) => {
            error!("Failed to transcribe recording: {}", e);
            let _ = notification.complete(false);
            return Err(e);
        }
    };

    info!("Transcription saved to: {}", transcript_path.display());

    notification.complete(true)?;

    sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn cancel_recording(
    recorder: &mut Recorder,
    notification: &mut Notification,
) -> Result<()> {
    info!("Cancelling recording...");

    let _ = recorder.stop();

    notification.complete(false)?;

    sleep(Duration::from_secs(1)).await;

    Ok(())
}
