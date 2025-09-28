use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
        Mutex,
    },
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device,
    Stream,
    StreamConfig,
};
use hound::{WavSpec, WavWriter};
use tokio::{task::JoinHandle, time::sleep};

use crate::{Error, Result};

pub struct AudioRecorder {
    device: Device,
    config: StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    stream: Option<Stream>,
    timeout_task: Option<JoinHandle<()>>,
}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            Error::MissingInputDevice(
                "Missing recorder input device".to_string(),
            )
        })?;

        println!("Using input device: {}", device.name()?);

        let config = device.default_input_config()?.into();

        println!("Input config: {:?}", config);

        Ok(Self {
            device,
            config,
            samples: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: None,
            timeout_task: None,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok(());
        }

        if let Some(handle) = self.timeout_task.take() {
            handle.abort();
        }

        let samples = Arc::clone(&self.samples);
        let is_recording = Arc::clone(&self.is_recording);

        samples.lock().unwrap().clear();
        is_recording.store(true, Ordering::Relaxed);

        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if is_recording.load(Ordering::Relaxed) {
                    let mut samples_guard = samples.lock().unwrap();
                    samples_guard.extend_from_slice(data);
                }
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);

        let is_recording_timeout = Arc::clone(&self.is_recording);
        self.timeout_task = Some(tokio::spawn(async move {
            sleep(Duration::from_secs(60)).await;
            if is_recording_timeout.swap(false, Ordering::Relaxed) {
                println!("Recording stopped due to 1-minute timeout");
            }
        }));

        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<f32>> {
        if !self.is_recording.swap(false, Ordering::Relaxed) {
            return Ok(Vec::new());
        }

        if let Some(handle) = self.timeout_task.take() {
            handle.abort();
        }

        if let Some(stream) = self.stream.take() {
            drop(stream);
        }

        let samples = self.samples.lock().unwrap().clone();
        println!("Recorded {} samples", samples.len());

        Ok(samples)
    }

    pub fn save<P: AsRef<Path>>(&self, samples: &[f32], path: P) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

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
        println!("Audio saved to: {}", path.as_ref().display());

        Ok(())
    }
}
