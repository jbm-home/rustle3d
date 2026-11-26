//! A photo being rendered in the background: its own thread takes samples
//! on the shared GPU and refreshes the image a few times a second, which
//! the 3D view shows while it refines. Stopping keeps the best image so
//! far.
//!
//! In the photo scene (spec 35) the photo is live: the camera, the light's
//! turn and brightness and the size follow [`PhotoJob::aim`], and refining
//! starts over at each change, at a quarter of the pixels until the
//! changes settle.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use r3d_render::photo::{Cloth, Environment, PhotoRenderer, PhotoScene, Settings, Surface};
use r3d_render::{Gpu, Mesh, OrbitCamera};

/// GPU time each batch of samples aims for: short enough that the 3D
/// view's frames, queued behind it, stay smooth.
const BATCH: Duration = Duration::from_millis(40);
/// The same for the photo scene's live photo, which the view must follow
/// while the camera turns.
const LIVE_BATCH: Duration = Duration::from_millis(12);
/// How often the image is refreshed while refining.
const REFRESH: Duration = Duration::from_millis(400);
/// How long a live photo waits after the last change before refining at
/// full size.
const SETTLE: Duration = Duration::from_millis(250);
/// The side of a live photo's draft, as a fraction of its own.
const DRAFT: f64 = 0.5;
/// How often a finished live photo looks for changes.
const IDLE: Duration = Duration::from_millis(15);

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

/// Where a live photo looks from, and its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Look {
    pub camera: OrbitCamera,
    pub turn_deg: f32,
    pub brightness: f32,
    pub place_size_m: f32,
    pub width: u32,
    pub height: u32,
}

impl Look {
    /// This look at `k` times the size (at least 16 pixels a side).
    fn scaled(&self, k: f64) -> (u32, u32) {
        let side = |v: u32| ((f64::from(v) * k).round() as u32).max(16);
        (side(self.width), side(self.height))
    }
}

/// The largest `format`-shaped image that fits a `view` (the view's own
/// shape without a format), within the photo's limits.
pub(crate) fn fitted(view: (u32, u32), format: Option<(u32, u32)>) -> (u32, u32) {
    let (vw, vh) = (f64::from(view.0.max(1)), f64::from(view.1.max(1)));
    let (fw, fh) = format
        .filter(|(w, h)| *w > 0 && *h > 0)
        .map_or((vw, vh), |(w, h)| (f64::from(w), f64::from(h)));
    let mut k = (vw / fw).min(vh / fh);
    let most = r3d_render::photo::MAX_PIXELS as f64;
    if fw * fh * k * k > most {
        k = (most / (fw * fh)).sqrt();
    }
    let side = f64::from(r3d_render::photo::MAX_SIDE);
    k = k.min(side / fw).min(side / fh);
    (
        ((fw * k).floor() as u32).max(16),
        ((fh * k).floor() as u32).max(16),
    )
}

struct Shared {
    stop: AtomicBool,
    progress: Mutex<Progress>,
    /// The image, once the renderer exists.
    image: Mutex<Option<wgpu::Texture>>,
    /// What a live photo should show next.
    wanted: Mutex<Option<Look>>,
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

impl Subject {
    fn scene(self) -> PhotoScene {
        let cloth: Vec<Cloth<'_>> = self
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
        let extras: Vec<(&Mesh, Surface)> = self.trims.iter().map(|(m, s)| (m, *s)).collect();
        let scene = PhotoScene::new(&cloth, &extras, self.body.as_ref());
        match self.environment {
            Some(env) => scene.with_environment(env),
            None => scene,
        }
    }
}

pub(crate) struct PhotoJob {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    /// Follows [`Self::aim`] (the photo scene's preview).
    live: bool,
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
        let look = Look {
            camera: subject.camera,
            turn_deg: settings.turn_deg,
            brightness: settings.brightness,
            place_size_m: settings.place_size_m,
            width,
            height,
        };
        Self::spawn(gpu, subject, settings, look, false)
    }

    /// Start a live photo of `subject` as `look` says, then as
    /// [`Self::aim`] says.
    pub fn live(
        gpu: Gpu,
        subject: Subject,
        settings: Settings,
        look: Look,
    ) -> std::io::Result<Self> {
        Self::spawn(gpu, subject, settings, look, true)
    }

    fn spawn(
        gpu: Gpu,
        subject: Subject,
        settings: Settings,
        look: Look,
        live: bool,
    ) -> std::io::Result<Self> {
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            progress: Mutex::new(Progress {
                total: settings.samples,
                width: look.width,
                height: look.height,
                ..Progress::default()
            }),
            image: Mutex::new(None),
            wanted: Mutex::new(live.then_some(look)),
        });
        let worker = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("photo".into())
            .spawn(move || {
                let started = Instant::now();
                let scene = subject.scene();
                let result = if live {
                    run_live(&gpu, &scene, settings, look, &worker)
                } else {
                    run(&gpu, &scene, settings, look, &worker, started)
                };
                if let Ok(mut p) = worker.progress.lock() {
                    p.finished = true;
                    p.seconds = started.elapsed().as_secs_f64();
                    p.error = result.err();
                }
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
            live,
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

    /// Whether this is the photo scene's live preview.
    pub fn is_live(&self) -> bool {
        self.live
    }

    /// Show `look` next (a live photo; ignored otherwise).
    pub fn aim(&self, look: Look) {
        if let Ok(mut wanted) = self.shared.wanted.lock()
            && wanted.is_some()
        {
            *wanted = Some(look);
        }
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

fn wait(gpu: &Gpu) -> Result<(), String> {
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Take one batch of samples, aimed at `target` GPU time; returns the
/// next batch's size.
fn refine(
    gpu: &Gpu,
    photo: &mut PhotoRenderer,
    batch: u32,
    resolve: bool,
    target: Duration,
) -> Result<u32, String> {
    let t = Instant::now();
    photo.sample(gpu, batch);
    if resolve {
        photo.resolve(gpu);
    }
    wait(gpu)?;
    let per_sample = t.elapsed().as_secs_f64() / f64::from(batch);
    Ok(((target.as_secs_f64() / per_sample.max(1e-6)) as u32).clamp(1, 16))
}

fn run(
    gpu: &Gpu,
    scene: &PhotoScene,
    settings: Settings,
    look: Look,
    shared: &Shared,
    started: Instant,
) -> Result<(), String> {
    let mut photo = PhotoRenderer::new(gpu, scene, &look.camera, settings, look.width, look.height)
        .map_err(|e| e.to_string())?;
    *shared.image.lock().map_err(|e| e.to_string())? = Some(photo.image().clone());
    let mut batch = 1u32;
    let mut shown = Instant::now();
    while !photo.is_done() && !shared.stop.load(Ordering::SeqCst) {
        let refresh = shown.elapsed() >= REFRESH || photo.samples_done() <= 2;
        batch = refine(gpu, &mut photo, batch, refresh, BATCH)?;
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
    wait(gpu)
}

/// The photo scene's preview: refine, and start over when the look
/// changes, as a draft until the changes settle.
fn run_live(
    gpu: &Gpu,
    scene: &PhotoScene,
    settings: Settings,
    first: Look,
    shared: &Shared,
) -> Result<(), String> {
    let make = |look: &Look, (w, h): (u32, u32)| {
        let settings = Settings {
            turn_deg: look.turn_deg,
            brightness: look.brightness,
            place_size_m: look.place_size_m,
            ..settings
        };
        PhotoRenderer::new(gpu, scene, &look.camera, settings, w, h).map_err(|e| e.to_string())
    };
    let mut shown_look = first;
    let mut draft = true;
    let mut changed = Instant::now();
    let mut photo = make(&first, first.scaled(DRAFT))?;
    // The image goes to the view once it holds the first samples.
    let mut new_image = true;
    let mut batch = 1u32;
    let mut shown = Instant::now();
    while !shared.stop.load(Ordering::SeqCst) {
        let wanted = shared
            .wanted
            .lock()
            .map_err(|e| e.to_string())?
            .unwrap_or(shown_look);
        let size = if wanted != shown_look {
            draft = true;
            changed = Instant::now();
            Some(wanted.scaled(DRAFT))
        } else if draft && changed.elapsed() >= SETTLE {
            draft = false;
            Some((wanted.width, wanted.height))
        } else {
            None
        };
        if let Some((w, h)) = size {
            if photo.aim(
                &wanted.camera,
                wanted.turn_deg,
                wanted.brightness,
                wanted.place_size_m,
            ) {
                photo.resize(gpu, w, h).map_err(|e| e.to_string())?;
            } else {
                // The camera left the studio: build it again around it.
                photo = make(&wanted, (w, h))?;
            }
            shown_look = wanted;
            new_image = true;
            batch = 1;
        }
        if photo.is_done() {
            std::thread::sleep(IDLE);
            continue;
        }
        let refresh = new_image || shown.elapsed() >= REFRESH || photo.is_done();
        batch = refine(gpu, &mut photo, batch, refresh, LIVE_BATCH)?;
        if refresh {
            shown = Instant::now();
        }
        if photo.is_done() {
            photo.resolve(gpu);
            wait(gpu)?;
        }
        if new_image {
            *shared.image.lock().map_err(|e| e.to_string())? = Some(photo.image().clone());
            new_image = false;
        }
        if let Ok(mut p) = shared.progress.lock() {
            p.samples = photo.samples_done();
            (p.width, p.height) = photo.size();
        }
    }
    Ok(())
}

/// Samples per pixel of the photo scene's live photo.
pub(crate) const SCENE_SAMPLES: u32 = 64;

/// The photo scene while it is open.
pub(crate) struct OpenScene {
    pub settings: crate::api::engine::SceneSettings,
    /// The 3D view's camera before, given back on closing.
    pub before: OrbitCamera,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_fits_the_view() {
        // The view's own shape without a format.
        assert_eq!(fitted((1600, 900), None), (1600, 900));
        // A portrait format in a landscape view: as tall as the view.
        assert_eq!(fitted((1600, 900), Some((1080, 1350))), (720, 900));
        // A landscape format in a narrow view: as wide as the view.
        assert_eq!(fitted((800, 1000), Some((1920, 1080))), (800, 450));
        // Within the photo's limits.
        let (w, h) = fitted((8000, 8000), None);
        assert!(
            w <= r3d_render::photo::MAX_SIDE
                && u64::from(w) * u64::from(h) <= r3d_render::photo::MAX_PIXELS
        );
    }
}
