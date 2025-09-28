use std::{
    env,
    path::{Path, PathBuf},
};

use tokio::{fs, process::Command};
use tracing::{debug, info};

use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct Transcriber {
    command: String,
    model: Option<String>,
    language: Option<String>,
    extra_args: Vec<String>,
}

impl Transcriber {
    pub fn new() -> Self {
        let command = env::var("WHISPER_COMMAND")
            .unwrap_or_else(|_| "whisper".to_string());
        let model = env::var("WHISPER_MODEL")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let language = env::var("WHISPER_LANGUAGE")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let extra_args = env::var("WHISPER_ARGS")
            .ok()
            .map(|args| {
                args.split_whitespace().map(|s| s.to_string()).collect()
            })
            .unwrap_or_default();

        Self {
            command,
            model,
            language,
            extra_args,
        }
    }

    pub async fn transcribe(&self, audio_path: &Path) -> Result<PathBuf> {
        let output_dir = audio_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut expected_transcript = audio_path.to_path_buf();
        expected_transcript.set_extension("txt");

        info!(
            "Transcribing recording with Whisper: {}",
            audio_path.display()
        );

        let mut args = Vec::new();
        args.push(audio_path.to_string_lossy().to_string());

        if let Some(model) = &self.model {
            args.push("--model".into());
            args.push(model.clone());
        }

        if let Some(language) = &self.language {
            args.push("--language".into());
            args.push(language.clone());
        }

        args.push("--output_format".into());
        args.push("txt".into());
        args.push("--output_dir".into());
        args.push(output_dir.to_string_lossy().to_string());

        args.extend(self.extra_args.clone());

        debug!("Running Whisper command: {} {:?}", &self.command, &args);

        let mut command = Command::new(&self.command);
        command.args(&args);

        let output = command.output().await.map_err(|err| {
            Error::Transcription(format!("Failed to run Whisper: {err}"))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Transcription(format!(
                "Whisper exited with status {}: {}",
                output.status, stderr
            )));
        }

        if fs::metadata(&expected_transcript).await.is_err() {
            let audio_filename = audio_path.file_name().ok_or_else(|| {
                Error::Transcription(
                    "Audio path is missing a file name".to_string(),
                )
            })?;
            let alternate = output_dir
                .join(format!("{}.txt", audio_filename.to_string_lossy()));

            if fs::metadata(&alternate).await.is_ok() {
                fs::rename(&alternate, &expected_transcript).await.map_err(
                    |err| {
                        Error::Transcription(format!(
                            "Failed to move transcript from {} to {}: {}",
                            alternate.display(),
                            expected_transcript.display(),
                            err
                        ))
                    },
                )?;
            } else {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Err(Error::Transcription(format!(
                    "Whisper did not produce a transcript at {}. Stdout: {}",
                    expected_transcript.display(),
                    stdout.trim()
                )));
            }
        }

        info!("Transcript ready: {}", expected_transcript.display());

        Ok(expected_transcript)
    }
}
