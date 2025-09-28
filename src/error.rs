#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Build stream error: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),
    #[error("Default stream config error: {0}")]
    DefaultStreamConfig(#[from] cpal::DefaultStreamConfigError),
    #[error("Device name error: {0}")]
    DeviceName(#[from] cpal::DeviceNameError),
    #[error("Hyprland is required but not running")]
    HyprlandNotRunning,
    #[error("Hound error: {0}")]
    Hound(#[from] hound::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Missing input device: {0}")]
    MissingInputDevice(String),
    #[error("Notification error: {0}")]
    Notification(String),
    #[error("Play stream error: {0}")]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error("Transcription error: {0}")]
    Transcription(String),
}

pub type Result<T> = std::result::Result<T, Error>;
