use std::{env, path::PathBuf, process::Command as StdCommand, time::Duration};

use tempfile::NamedTempFile;
use tokio::{fs, time::interval};
use tracing::{debug, info, warn};

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    Save,
    Cancel,
}

pub struct KeyHandler {
    temp_file: Option<NamedTempFile>,
    bindings_registered: bool,
}

impl KeyHandler {
    pub async fn new() -> Result<Self> {
        let runtime_dir = env::var("XDG_RUNTIME_DIR")
            .map_err(|_| Error::HyprlandNotRunning)?;

        let hyprland_instance = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map_err(|_| Error::HyprlandNotRunning)?;

        let socket_path = PathBuf::from(runtime_dir)
            .join("hypr")
            .join(&hyprland_instance)
            .join(".socket.sock");

        info!("Using Hyprland socket: {}", socket_path.display());

        Ok(Self {
            temp_file: None,
            bindings_registered: false,
        })
    }

    pub async fn register_bindings(&mut self) -> Result<()> {
        info!("Registering global keybindings");

        // Create temporary file for communication
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_string_lossy();

        // Register keybindings via Hyprland IPC
        let enter_cmd =
            format!("keyword bind ,Return,exec,echo 'SAVE' > {temp_path}");
        let escape_cmd =
            format!("keyword bind ,Escape,exec,echo 'CANCEL' > {temp_path}");

        self.send_cmd(&enter_cmd).await?;
        self.send_cmd(&escape_cmd).await?;

        self.temp_file = Some(temp_file);
        self.bindings_registered = true;

        info!("Global keybindings registered successfully");
        Ok(())
    }

    pub async fn wait_for_input(&self) -> Result<KeyAction> {
        let temp_file = self.temp_file.as_ref().unwrap();
        let temp_path = temp_file.path();

        debug!("Waiting for key input via file: {}", temp_path.display());

        let mut interval = interval(Duration::from_millis(50));

        loop {
            interval.tick().await;

            if let Ok(content) = fs::read_to_string(temp_path).await {
                let content = content.trim();
                if !content.is_empty() {
                    debug!("Received key input: {}", content);

                    // Clear the file for next input
                    let _ = fs::write(temp_path, "").await;

                    match content {
                        "SAVE" => return Ok(KeyAction::Save),
                        "CANCEL" => return Ok(KeyAction::Cancel),
                        _ => {
                            warn!("Unknown key action: {}", content);
                            continue;
                        }
                    }
                }
            }
        }
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        if !self.bindings_registered {
            return Ok(());
        }

        info!("Cleaning up global keybindings");

        // Remove the keybindings
        let remove_enter = "keyword unbind ,Return";
        let remove_escape = "keyword unbind ,Escape";

        let mut had_error = false;

        if let Err(e) = self.send_cmd(remove_enter).await {
            warn!("Failed to remove Enter keybinding asynchronously: {}", e);
            had_error = true;
        }

        if let Err(e) = self.send_cmd(remove_escape).await {
            warn!("Failed to remove Escape keybinding asynchronously: {}", e);
            had_error = true;
        }

        if had_error {
            warn!("Falling back to blocking keybinding cleanup");
            self.cleanup_blocking();
        } else {
            self.finish_cleanup();
        }

        Ok(())
    }

    async fn send_cmd(&self, command: &str) -> Result<String> {
        debug!("Sending Hyprland command: {}", command);

        // Use `hyprctl` to send commands (more reliable than direct socket)
        let output = tokio::process::Command::new("hyprctl")
            .arg("--batch")
            .arg(command)
            .output()
            .await?;

        if !output.status.success() {
            return Err(Error::HyprlandNotRunning);
        }

        let response = String::from_utf8_lossy(&output.stdout);
        debug!("Hyprland response: {}", response);

        Ok(response.to_string())
    }

    fn cleanup_blocking(&mut self) {
        if !self.bindings_registered {
            return;
        }

        for (command, name) in [
            ("keyword unbind ,Return", "Enter"),
            ("keyword unbind ,Escape", "Escape"),
        ] {
            if let Err(e) = Self::send_cmd_blocking(command) {
                warn!(
                    "Failed to remove {name} keybinding in blocking fallback: {}",
                    e
                );
            }
        }

        self.finish_cleanup();
    }

    fn finish_cleanup(&mut self) {
        self.bindings_registered = false;
        self.temp_file = None;
        info!("Keybinding cleanup completed");
    }

    fn send_cmd_blocking(command: &str) -> Result<String> {
        debug!("Sending Hyprland command (blocking): {}", command);

        let output = StdCommand::new("hyprctl")
            .arg("--batch")
            .arg(command)
            .output()?;

        if !output.status.success() {
            return Err(Error::HyprlandNotRunning);
        }

        let response = String::from_utf8_lossy(&output.stdout);
        debug!("Hyprland response (blocking): {}", response);

        Ok(response.to_string())
    }
}

impl Drop for KeyHandler {
    fn drop(&mut self) {
        self.cleanup_blocking();
    }
}
