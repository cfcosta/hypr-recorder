use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::{
        Arc,
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cpal::{
    Device,
    Stream,
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use hound::{WavSpec, WavWriter};
use tokio::time::sleep;
use tracing::{debug, error, info};

pub struct AudioRecorder {
    device: Device,
    config: StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    stream: Option<Stream>,
    start_time: Option<Instant>,
}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input device available")?;

        info!("Using input device: {}", device.name()?);

        let config = device
            .default_input_config()
            .context("Failed to get default input config")?
            .into();

        debug!("Input config: {:?}", config);

        Ok(Self {
            device,
            config,
            samples: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: None,
            start_time: None,
        })
    }

    pub async fn start_recording(&mut self) -> Result<()> {
        info!("Starting audio recording...");

        let samples = Arc::clone(&self.samples);
        let is_recording = Arc::clone(&self.is_recording);

        // Clear previous samples
        samples.lock().unwrap().clear();
        is_recording.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());

        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if is_recording.load(Ordering::Relaxed) {
                    let mut samples_guard = samples.lock().unwrap();
                    samples_guard.extend_from_slice(data);
                }
            },
            |err| {
                error!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);

        // Start timeout task
        let is_recording_timeout = Arc::clone(&self.is_recording);
        tokio::spawn(async move {
            sleep(Duration::from_secs(60)).await;
            is_recording_timeout.store(false, Ordering::Relaxed);
            info!("Recording stopped due to 1-minute timeout");
        });

        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<Vec<f32>> {
        info!("Stopping audio recording...");

        self.is_recording.store(false, Ordering::Relaxed);

        if let Some(stream) = self.stream.take() {
            drop(stream);
        }

        let samples = self.samples.lock().unwrap().clone();
        info!("Recorded {} samples", samples.len());

        Ok(samples)
    }

    pub fn save_to_file<P: AsRef<Path>>(
        &self,
        samples: &[f32],
        path: P,
    ) -> Result<()> {
        let spec = WavSpec {
            channels: self.config.channels,
            sample_rate: self.config.sample_rate.0,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let file = File::create(path.as_ref())?;
        let mut writer = WavWriter::new(BufWriter::new(file), spec)?;

        for &sample in samples {
            writer.write_sample(sample)?;
        }

        writer.finalize()?;
        info!("Audio saved to: {}", path.as_ref().display());

        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }
}
