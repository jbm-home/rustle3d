//! A photo being rendered in the background: its own thread takes samples
//! on the shared GPU and refreshes the image a few times a second, which
//! the 3D view shows while it refines. Stopping keeps the best image so
//! far.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use r3d_render::photo::{Cloth, Environment, PhotoRenderer, PhotoScene, Settings, Surface};
use r3d_render::{Gpu, Mesh, OrbitCamera};

/// GPU time each batch of samples aims for: short enough that the 3D
/// view's frames, queued behind it, stay smooth.
const BATCH: Duration = Duration::from_millis(40);
/// How often the image is refreshed while refining.
const REFRESH: Duration = Duration::from_millis(400);

/// How far a photo has come.
#[derive(Clone, Debug, Default)]
pub(crate) struct Progress {
    pub samples: u32,
    pub total: u32,
    pub width: u32,
    pub height: u32,
    /// No more samples will come (finished, stopped or failed).
    pub finished: bool,
    pub seconds: f64,
    pub error: Option<String>,
}

struct Shared {
    stop: AtomicBool,
    progress: Mutex<Progress>,
    /// The image, once the renderer exists.
    image: Mutex<Option<wgpu::Texture>>,
}

/// What a photo shows, gathered on the caller's thread.
pub(crate) struct Subject {
    /// Each piece with its surface and its print.
    pub cloth: Vec<(
        Mesh,
        Surface,
        Option<std::sync::Arc<r3d_commands::prints::PrintImage>>,
    )>,
    /// Trims and stitches, each a mesh in a plain colour.
    pub trims: Vec<(Mesh, Surface)>,
    pub body: Option<Mesh>,
    pub camera: OrbitCamera,
    /// The light around instead of the studio.
    pub environment: Option<Environment>,
}

pub(crate) struct PhotoJob {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl PhotoJob {
    /// Start rendering `subject` at `width × height` on `gpu`.
    pub fn start(
        gpu: Gpu,
        subject: Subject,
        settings: Settings,
        width: u32,
        height: u32,
    ) -> std::io::Result<Self> {
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            progress: Mutex::new(Progress {
                total: settings.samples,
                width,
                height,
                ..Progress::default()
            }),
            image: Mutex::new(None),
        });
        let worker = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("photo".into())
            .spawn(move || {
                let started = Instant::now();
                let result = run(&gpu, subject, settings, width, height, &worker, started);
                if let Ok(mut p) = worker.progress.lock() {
                    p.finished = true;
                    p.seconds = started.elapsed().as_secs_f64();
                    p.error = result.err();
                }
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    pub fn progress(&self) -> Progress {
        self.shared
            .progress
            .lock()
            .map(|p| p.clone())
            .unwrap_or_else(|_| Progress {
                finished: true,
                error: Some("photo state lost".into()),
                ..Progress::default()
            })
    }

    /// Stop taking samples; the image keeps the samples so far.
    pub fn stop(&self) {
        self.shared.stop.store(true, Ordering::SeqCst);
    }

    /// The image so far, if rendering has begun.
    pub fn image(&self) -> Option<wgpu::Texture> {
        self.shared.image.lock().ok().and_then(|i| i.clone())
    }
}

impl Drop for PhotoJob {
    fn drop(&mut self) {
        self.stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run(
    gpu: &Gpu,
    mut subject: Subject,
    settings: Settings,
    width: u32,
    height: u32,
    shared: &Shared,
    started: Instant,
) -> Result<(), String> {
    let environment = subject.environment.take();
    let cloth: Vec<Cloth<'_>> = subject
        .cloth
        .iter()
        .map(|(mesh, surface, print)| Cloth {
            mesh,
            surface: *surface,
            print: print.as_ref().map(|p| r3d_render::Printed {
                key: p.key,
                texture: &p.texture,
            }),
        })
        .collect();
    let extras: Vec<(&Mesh, Surface)> = subject.trims.iter().map(|(m, s)| (m, *s)).collect();
    let mut scene = PhotoScene::new(&cloth, &extras, subject.body.as_ref());
    if let Some(env) = environment {
        scene = scene.with_environment(env);
    }
    let mut photo = PhotoRenderer::new(gpu, &scene, &subject.camera, settings, width, height)
        .map_err(|e| e.to_string())?;
    *shared.image.lock().map_err(|e| e.to_string())? = Some(photo.image().clone());
    let mut batch = 1u32;
    let mut shown = Instant::now();
    while !photo.is_done() && !shared.stop.load(Ordering::SeqCst) {
        let t = Instant::now();
        photo.sample(gpu, batch);
        let refresh = shown.elapsed() >= REFRESH || photo.samples_done() <= 2 || photo.is_done();
        if refresh {
            photo.resolve(gpu);
        }
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        // Aim the next batch at BATCH of GPU time.
        let per_sample = t.elapsed().as_secs_f64() / f64::from(batch);
        batch = ((BATCH.as_secs_f64() / per_sample.max(1e-6)) as u32).clamp(1, 16);
        if refresh {
            shown = Instant::now();
        }
        if let Ok(mut p) = shared.progress.lock() {
            p.samples = photo.samples_done();
            p.seconds = started.elapsed().as_secs_f64();
        }
    }
    // The best image so far.
    photo.resolve(gpu);
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    Ok(())
}
