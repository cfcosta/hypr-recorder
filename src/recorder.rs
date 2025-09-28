use std::{
    env,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

use ashpd::{
    Error as AshpdError,
    PortalError as AshpdPortalError,
    desktop::{
        PersistMode,
        Session,
        screencast::{CursorMode, Screencast, SourceType, Stream},
    },
};
use gstreamer::{self as gst, ClockTime, prelude::*};
use tokio::{fs, task::JoinHandle, time::sleep};

use crate::{Error, Result};

const RECORDING_LIMIT_SECS: u64 = 60;

pub struct Recorder {
    pipeline: Option<gst::Pipeline>,
    session: Option<Session<'static, Screencast<'static>>>,
    remote_fd: Option<OwnedFd>,
    recording_path: Option<PathBuf>,
    is_recording: Arc<AtomicBool>,
    start_time: Option<Instant>,
    timeout_task: Option<JoinHandle<()>>,
}

impl Recorder {
    pub async fn new() -> Result<Self> {
        gst::init()?;

        Ok(Self {
            pipeline: None,
            session: None,
            remote_fd: None,
            recording_path: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            start_time: None,
            timeout_task: None,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok(());
        }

        let output_path = Self::recording_path()?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let resources = Self::build_pipeline(&output_path).await?;
        let pipeline = resources.pipeline;
        pipeline.set_state(gst::State::Playing)?;

        self.pipeline = Some(pipeline);
        self.session = Some(resources.session);
        self.remote_fd = Some(resources.remote_fd);
        self.recording_path = Some(output_path);
        self.start_time = Some(Instant::now());
        self.is_recording.store(true, Ordering::Relaxed);

        let is_recording_flag = Arc::clone(&self.is_recording);
        self.timeout_task = Some(tokio::spawn(async move {
            sleep(Duration::from_secs(RECORDING_LIMIT_SECS)).await;
            if is_recording_flag.swap(false, Ordering::Relaxed) {
                println!("Recording stopped due to 1-minute timeout");
            }
        }));

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<Option<PathBuf>> {
        self.finish(false).await
    }

    pub async fn cancel(&mut self) -> Result<()> {
        if let Some(path) = self.finish(true).await? {
            if let Err(err) = fs::remove_file(&path).await {
                eprintln!(
                    "Failed to remove cancelled recording {}: {err}",
                    path.display()
                );
            }
        }

        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    async fn finish(&mut self, discard: bool) -> Result<Option<PathBuf>> {
        if let Some(handle) = self.timeout_task.take() {
            handle.abort();
        }

        let was_recording = self.is_recording.swap(false, Ordering::Relaxed);

        if let Some(pipeline) = self.pipeline.take() {
            pipeline.send_event(gst::event::Eos::new());
            if let Some(bus) = pipeline.bus() {
                let timeout = Some(ClockTime::from_mseconds(100));
                while let Some(msg) = bus.timed_pop(timeout) {
                    match msg.view() {
                        gst::MessageView::Eos(_)
                        | gst::MessageView::Error(_) => break,
                        _ => (),
                    }
                }
            }
            pipeline.set_state(gst::State::Null)?;
        }

        if let Some(session) = self.session.take() {
            let _ = session.close().await;
        }

        self.remote_fd = None;
        self.start_time = None;

        let path = self.recording_path.take();

        if discard {
            return Ok(path);
        }

        if !was_recording {
            if let Some(ref stale) = path {
                let _ = fs::remove_file(stale).await;
            }
            return Ok(None);
        }

        if let Some(path) = path {
            match fs::metadata(&path).await {
                Ok(metadata) if metadata.len() > 0 => Ok(Some(path)),
                Ok(_) | Err(_) => {
                    let _ = fs::remove_file(&path).await;
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn recording_path() -> Result<PathBuf> {
        let timestamp = SystemTime::now().elapsed()?.as_millis();
        let filename = format!("capture_{timestamp}.mp4");

        let base_dir = env::home_dir()
            .map(|dir| dir.join("Recordings"))
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        Ok(base_dir.join(filename))
    }

    async fn build_pipeline(path: &Path) -> Result<PipelineResources> {
        let screencast = Screencast::new().await?;
        let session = screencast.create_session().await?;

        let available_types = screencast.available_source_types().await?;

        let mut requested_types = SourceType::Monitor | SourceType::Window;
        let audio_supported = available_types.contains(SourceType::Virtual);
        if audio_supported {
            requested_types |= SourceType::Virtual;
        }

        requested_types &= available_types;

        if !requested_types.contains(SourceType::Monitor)
            && !requested_types.contains(SourceType::Window)
        {
            return Err(Error::ScreenCapture(
                "Portal did not advertise any monitor or window sources to capture"
                    .into(),
            ));
        }

        screencast
            .select_sources(
                &session,
                CursorMode::Embedded,
                requested_types,
                true,
                None,
                PersistMode::DoNot,
            )
            .await?
            .response()?;

        let start_request = match screencast.start(&session, None).await {
            Ok(request) => request,
            Err(err) => {
                if let AshpdError::Portal(AshpdPortalError::Failed(message)) =
                    &err
                {
                    if message.contains("No streams available") {
                        return Err(Error::ScreenCapture(
                            "Portal did not return any streams. Please ensure you selected a source and that audio capture is enabled in your compositor portal configuration."
                                .into(),
                        ));
                    }
                }

                return Err(err.into());
            }
        };

        let streams = match start_request.response() {
            Ok(streams) => streams,
            Err(err) => {
                if let AshpdError::Portal(AshpdPortalError::Failed(message)) =
                    &err
                {
                    if message.contains("No streams available") {
                        return Err(Error::ScreenCapture(
                            "Portal did not return any streams. Please ensure you selected a source and that audio capture is enabled in your compositor portal configuration."
                                .into(),
                        ));
                    }
                }

                return Err(err.into());
            }
        };

        println!("Portal returned {} stream(s)", streams.streams().len());
        for stream in streams.streams() {
            println!(
                "- node {} type {:?} id {:?}",
                stream.pipe_wire_node_id(),
                stream.source_type(),
                stream.id()
            );
        }

        let remote_fd = screencast.open_pipe_wire_remote(&session).await?;

        let (video_stream, audio_stream) =
            Self::split_streams(streams.streams())?;

        let remote = unsafe { OwnedFd::from_raw_fd(remote_fd.into_raw_fd()) };
        let pipeline =
            Self::create_pipeline(&remote, video_stream, audio_stream, path)?;

        Ok(PipelineResources {
            pipeline,
            session,
            remote_fd: remote,
        })
    }

    fn split_streams(streams: &[Stream]) -> Result<(Stream, Option<Stream>)> {
        let mut video: Option<Stream> = None;
        let mut audio: Option<Stream> = None;

        for stream in streams.iter().cloned() {
            match stream.source_type() {
                Some(SourceType::Monitor) | Some(SourceType::Window) => {
                    video = Some(stream)
                }
                Some(SourceType::Virtual) => audio = Some(stream),
                None => {
                    if stream
                        .id()
                        .map(|id| id.contains("audio"))
                        .unwrap_or(false)
                    {
                        audio = Some(stream);
                    } else {
                        video = Some(stream);
                    }
                }
            }
        }

        match (video, audio) {
            (Some(v), maybe_audio) => {
                if maybe_audio.is_none() {
                    eprintln!(
                        "Portal did not supply an audio stream. Continuing with video-only recording."
                    );
                    eprintln!(
                        "If you expected audio, ensure xdg-desktop-portal-hyprland and PipeWire are configured for audio capture."
                    );
                }
                Ok((v, maybe_audio))
            }
            _ => Err(Error::ScreenCapture(
                "Portal did not provide a video stream".into(),
            )),
        }
    }

    fn create_pipeline(
        remote_fd: &OwnedFd,
        video_stream: Stream,
        audio_stream: Option<Stream>,
        output_path: &Path,
    ) -> Result<gst::Pipeline> {
        let video_fd = Self::dup_fd(remote_fd.as_raw_fd())?;
        let video_path = video_stream.pipe_wire_node_id();
        let location = output_path.display();

        let pipeline_description = if let Some(audio_stream) = audio_stream {
            let audio_fd = Self::dup_fd(remote_fd.as_raw_fd())?;
            let audio_path = audio_stream.pipe_wire_node_id();
            format!(
                "pipewiresrc fd={video_fd} path={video_path} do-timestamp=true ! queue ! videoconvert ! queue ! \
                 x264enc bitrate=8000 speed-preset=ultrafast tune=zerolatency key-int-max=60 ! h264parse ! queue ! mux. \
                 pipewiresrc fd={audio_fd} path={audio_path} do-timestamp=true ! queue ! audioconvert ! audioresample ! \
                 avenc_aac bitrate=128000 compliance=-2 ! queue ! mux. mp4mux name=mux faststart=true ! filesink location=\"{location}\""
            )
        } else {
            format!(
                "pipewiresrc fd={video_fd} path={video_path} do-timestamp=true ! queue ! videoconvert ! queue ! \
                 x264enc bitrate=8000 speed-preset=ultrafast tune=zerolatency key-int-max=60 ! h264parse ! queue ! mp4mux name=mux faststart=true ! filesink location=\"{location}\""
            )
        };

        let element = gst::parse::launch(&pipeline_description)?;
        element.downcast::<gst::Pipeline>().map_err(|_| {
            Error::ScreenCapture("Failed to create GStreamer pipeline".into())
        })
    }

    fn dup_fd(fd: RawFd) -> Result<i32> {
        let duplicated = unsafe { libc::dup(fd) };
        if duplicated < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        Ok(duplicated)
    }
}

struct PipelineResources {
    pipeline: gst::Pipeline,
    session: Session<'static, Screencast<'static>>,
    remote_fd: OwnedFd,
}
