#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Hyprland is required but not running")]
    HyprlandNotRunning,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Notification error: {0}")]
    Notification(String),
    #[error("Portal error: {0}")]
    Portal(#[from] ashpd::Error),
    #[error("GStreamer error: {0}")]
    Gstreamer(#[from] gstreamer::glib::Error),
    #[error("GStreamer state change error: {0}")]
    GstreamerState(#[from] gstreamer::StateChangeError),
    #[error("Screen capture error: {0}")]
    ScreenCapture(String),
    #[error("System time error: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
    #[error("Transcription error: {0}")]
    Transcription(String),
}

pub type Result<T> = std::result::Result<T, Error>;
