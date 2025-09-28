use std::{process::Command, time::Duration};

use tracing::info;

use crate::{Error, Result};

pub struct Notification {
    is_active: bool,
}

impl Notification {
    pub fn show() -> Result<Self> {
        info!("Showing recording notification via swayosd");

        Self::show_progress(0, 0)?;

        Ok(Self { is_active: true })
    }

    pub fn update(&mut self, elapsed: Duration) -> Result<()> {
        if !self.is_active {
            return Ok(());
        }

        let elapsed_secs = elapsed.as_secs();
        let progress_percent =
            (elapsed_secs as f32 / 60.0 * 100.0).min(100.0) as u32;

        Self::show_progress(progress_percent, elapsed_secs)?;
        Ok(())
    }

    pub fn complete(&mut self, saved: bool) -> Result<()> {
        self.is_active = false;

        let (message, icon) = if saved {
            ("Recording Saved", "audio-input-microphone")
        } else {
            ("Recording Cancelled", "dialog-warning")
        };

        info!("Showing completion notification: saved={}", saved);

        let status = Command::new("swayosd-client")
            .args(["--custom-message", message, "--custom-icon", icon])
            .output()?;

        if !status.status.success() {
            return Err(Error::Notification(format!(
                "swayosd-client failed with status: {}",
                status.status
            )));
        }

        Ok(())
    }

    fn show_progress(percent: u32, elapsed_secs: u64) -> Result<()> {
        let message = format!("Recording: {elapsed_secs}s / 60s");

        let status = Command::new("swayosd-client")
            .args([
                "--custom-progress",
                &percent.to_string(),
                "--custom-progress-text",
                &message,
                "--custom-icon",
                "audio-input-microphone",
            ])
            .output()?;

        if !status.status.success() {
            return Err(Error::Notification(format!(
                "swayosd-client failed with status: {}",
                status.status
            )));
        }

        Ok(())
    }
}
