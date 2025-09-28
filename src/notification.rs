use std::time::Duration;

use crate::{Error, Result, utils::run};

pub struct Notification {
    is_active: bool,
}

impl Notification {
    pub fn show() -> Result<Self> {
        println!("Showing recording notification via swayosd");

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

        println!("Showing completion notification: saved={}", saved);

        let output = run!(
            "swayosd-client",
            "--custom-message",
            message,
            "--custom-icon",
            icon
        )?;

        if output.is_failure() {
            return Err(Error::Notification(format!(
                "swayosd-client failed with status {}: {}",
                output.status,
                output.stderr.trim()
            )));
        }

        Ok(())
    }

    fn show_progress(percent: u32, elapsed_secs: u64) -> Result<()> {
        let message = format!("Recording: {elapsed_secs}s / 60s");

        let progress = percent.to_string();

        let output = run!(
            "swayosd-client",
            "--custom-progress",
            progress,
            "--custom-progress-text",
            &message,
            "--custom-icon",
            "audio-input-microphone"
        )?;

        if output.is_failure() {
            return Err(Error::Notification(format!(
                "swayosd-client failed with status {}: {}",
                output.status,
                output.stderr.trim()
            )));
        }

        Ok(())
    }
}
