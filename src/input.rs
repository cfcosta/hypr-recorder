use std::{env, path::PathBuf, time::Duration};

use tempfile::NamedTempFile;
use tokio::{fs, time::interval};

use crate::{
    utils::{run, run_async},
    Error,
    Result,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Save,
    Cancel,
}

pub struct Input {
    temp_file: Option<NamedTempFile>,
    bindings_registered: bool,
}

impl Input {
    pub async fn new() -> Result<Self> {
        let runtime_dir = env::var("XDG_RUNTIME_DIR")
            .map_err(|_| Error::HyprlandNotRunning)?;

        let hyprland_instance = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map_err(|_| Error::HyprlandNotRunning)?;

        let socket_path = PathBuf::from(runtime_dir)
            .join("hypr")
            .join(&hyprland_instance)
            .join(".socket.sock");

        println!("Using Hyprland socket: {}", socket_path.display());

        Ok(Self {
            temp_file: None,
            bindings_registered: false,
        })
    }

    pub async fn register(&mut self) -> Result<()> {
        println!("Registering global keybindings");

        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_string_lossy();

        let enter_cmd =
            format!("keyword bind ,Return,exec,echo 'SAVE' > {temp_path}");
        let escape_cmd =
            format!("keyword bind ,Escape,exec,echo 'CANCEL' > {temp_path}");

        self.cmd(&enter_cmd).await?;
        self.cmd(&escape_cmd).await?;

        self.temp_file = Some(temp_file);
        self.bindings_registered = true;

        println!("Global keybindings registered successfully");
        Ok(())
    }

    pub async fn wait_for_input(&self) -> Result<Action> {
        let temp_file = self.temp_file.as_ref().unwrap();
        let temp_path = temp_file.path();

        let mut interval = interval(Duration::from_millis(50));

        loop {
            interval.tick().await;

            if let Ok(content) = fs::read_to_string(temp_path).await {
                let content = content.trim();
                if !content.is_empty() {
                    let _ = fs::write(temp_path, "").await;

                    match content {
                        "SAVE" => return Ok(Action::Save),
                        "CANCEL" => return Ok(Action::Cancel),
                        _ => {
                            eprintln!("Unknown key action: {}", content);
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

        println!("Cleaning up global keybindings");

        let remove_enter = "keyword unbind ,Return";
        let remove_escape = "keyword unbind ,Escape";

        let mut had_error = false;

        if let Err(e) = self.cmd(remove_enter).await {
            eprintln!("Failed to remove Enter keybinding asynchronously: {}", e);
            had_error = true;
        }

        if let Err(e) = self.cmd(remove_escape).await {
            eprintln!("Failed to remove Escape keybinding asynchronously: {}", e);
            had_error = true;
        }

        if had_error {
            eprintln!("Falling back to blocking keybinding cleanup");
            self.cleanup_blocking();
        } else {
            self.finish_cleanup();
        }

        Ok(())
    }

    async fn cmd(&self, command: &str) -> Result<String> {
        // Use `hyprctl` to send commands (more reliable than direct socket)
        let output = run_async!("hyprctl", "--batch", command)?;

        if output.is_failure() {
            return Err(Error::HyprlandNotRunning);
        }

        let response = output.stdout;

        Ok(response)
    }

    fn cleanup_blocking(&mut self) {
        if !self.bindings_registered {
            return;
        }

        for (command, name) in [
            ("keyword unbind ,Return", "Enter"),
            ("keyword unbind ,Escape", "Escape"),
        ] {
            if let Err(e) = Self::cmd_blocking(command) {
                eprintln!(
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
        println!("Keybinding cleanup completed");
    }

    fn cmd_blocking(command: &str) -> Result<String> {
        let output = run!("hyprctl", "--batch", command)?;

        if output.is_failure() {
            return Err(Error::HyprlandNotRunning);
        }

        let response = output.stdout;

        Ok(response)
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        self.cleanup_blocking();
    }
}
