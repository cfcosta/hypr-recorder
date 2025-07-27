use std::time::Duration;

use anyhow::{Context, Result};
use notify_rust::{Notification, NotificationHandle, Timeout};
use tracing::{debug, error, info};

pub struct RecordingNotification {
    handle: Option<NotificationHandle>,
}

impl RecordingNotification {
    pub fn show() -> Result<Self> {
        info!("Showing recording notification");

        let notification = Notification::new()
            .summary("🎤 Recording Audio")
            .body("Press Enter to save, Esc to cancel\nRecording: 0s / 60s")
            .icon("audio-input-microphone")
            .timeout(Timeout::Never)
            .show()
            .context("Failed to show notification")?;

        Ok(Self {
            handle: Some(notification),
        })
    }

    pub fn update_progress(&mut self, elapsed: Duration) -> Result<()> {
        let elapsed_secs = elapsed.as_secs();
        let progress_percent =
            (elapsed_secs as f32 / 60.0 * 100.0).min(100.0) as u8;

        // Create progress bar visualization
        let bar_length = 20;
        let filled_length =
            (bar_length as f32 * progress_percent as f32 / 100.0) as usize;
        let empty_length = bar_length - filled_length;

        let progress_bar = format!(
            "[{}{}]",
            "█".repeat(filled_length),
            "░".repeat(empty_length)
        );

        let body = format!(
            "Press Enter to save, Esc to cancel\n{} {}s / 60s ({}%)",
            progress_bar, elapsed_secs, progress_percent
        );

        debug!("Updating notification progress: {}%", progress_percent);

        if let Some(ref mut handle) = self.handle {
            // Update the existing notification
            let updated = Notification::new()
                .summary("🎤 Recording Audio")
                .body(&body)
                .icon("audio-input-microphone")
                .timeout(Timeout::Never)
                .show()
                .context("Failed to update notification")?;

            *handle = updated;
        }

        Ok(())
    }

    pub fn show_completed(&mut self, saved: bool) -> Result<()> {
        let (summary, body, icon) = if saved {
            (
                "✅ Recording Saved",
                "Audio recording has been saved successfully",
                "audio-input-microphone",
            )
        } else {
            (
                "❌ Recording Cancelled",
                "Audio recording was cancelled",
                "dialog-warning",
            )
        };

        info!("Showing completion notification: saved={}", saved);

        let notification = Notification::new()
            .summary(summary)
            .body(body)
            .icon(icon)
            .timeout(Timeout::Milliseconds(3000))
            .show()
            .context("Failed to show completion notification")?;

        self.handle = Some(notification);
        Ok(())
    }

    pub fn hide(&mut self) {
        if let Some(handle) = self.handle.take() {
            // The notification will be automatically cleaned up when handle is dropped
            drop(handle);
            debug!("Notification hidden");
        }
    }
}

impl Drop for RecordingNotification {
    fn drop(&mut self) {
        self.hide();
    }
}
