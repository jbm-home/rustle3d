//! The simulation thread (docs/architecture.md): it owns the solver, steps
//! it in real time while running, and publishes particle positions that the
//! 3D view draws. The UI thread only sends control messages and copies the
//! latest positions, so rendering never waits for a solver step.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use glam::Vec3;
use r3d_sim::Solver;

/// Simulated time per step (s).
const FRAME_DT: f32 = 1.0 / 60.0;

enum Control {
    /// Replace the solver (paused) with this one.
    Load(Box<dyn Solver>),
    Run(bool),
    /// Hold the cloth around a particle (and run).
    Grab(usize),
    /// Move the held cloth's target.
    GrabTo(Vec3),
    Release,
    Clear,
    Stop,
}

/// What the thread publishes after every step.
#[derive(Debug, Clone, Default)]
pub(crate) struct Published {
    pub positions: Vec<Vec3>,
    pub loaded: bool,
    pub running: bool,
    /// Simulated time (s).
    pub time: f32,
    /// Wall time of the last step (ms).
    pub step_ms: f64,
    /// Largest distance between sewn points (m).
    pub max_seam_gap: f32,
    /// Where the solver runs (`"cpu"` or `"gpu"`).
    pub backend: &'static str,
    /// Why the solver stopped stepping (its device failed); the positions
    /// are those of its last good step.
    pub failed: Option<String>,
}

pub(crate) struct SimThread {
    tx: Sender<Control>,
    published: Arc<Mutex<Published>>,
    handle: Option<JoinHandle<()>>,
}

impl SimThread {
    pub fn spawn() -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let published = Arc::new(Mutex::new(Published::default()));
        let out = Arc::clone(&published);
        let handle = std::thread::Builder::new()
            .name("r3d-sim".into())
            .spawn(move || run(&rx, &out))?;
        Ok(Self {
            tx,
            published,
            handle: Some(handle),
        })
    }

    /// Update the published state right away, so callers see the effect
    /// of a control before the thread gets to it.
    fn publish_now(&self, update: impl FnOnce(&mut Published)) {
        update(
            &mut self
                .published
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
    }

    /// Load a new solver, paused, replacing the current one.
    pub fn load(&self, solver: Box<dyn Solver>) {
        let positions = solver.positions().to_vec();
        let backend = solver.backend();
        self.publish_now(|p| {
            *p = Published {
                positions,
                loaded: true,
                backend,
                ..Published::default()
            }
        });
        let _ = self.tx.send(Control::Load(solver));
    }

    pub fn set_running(&self, running: bool) {
        self.publish_now(|p| p.running = running && p.loaded);
        let _ = self.tx.send(Control::Run(running));
    }

    /// Hold the cloth around `particle` and run the simulation, so it
    /// responds.
    pub fn grab(&self, particle: usize) {
        self.publish_now(|p| p.running = p.loaded);
        let _ = self.tx.send(Control::Grab(particle));
    }

    pub fn grab_to(&self, target: Vec3) {
        let _ = self.tx.send(Control::GrabTo(target));
    }

    pub fn release(&self) {
        let _ = self.tx.send(Control::Release);
    }

    pub fn clear(&self) {
        self.publish_now(|p| *p = Published::default());
        let _ = self.tx.send(Control::Clear);
    }

    /// The latest published state.
    pub fn latest(&self) -> Published {
        self.published
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for SimThread {
    fn drop(&mut self) {
        let _ = self.tx.send(Control::Stop);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn publish(
    out: &Mutex<Published>,
    solver: Option<&dyn Solver>,
    running: bool,
    step_ms: f64,
    failed: Option<&String>,
) {
    let mut p = out.lock().unwrap_or_else(PoisonError::into_inner);
    p.loaded = solver.is_some();
    p.failed = failed.cloned();
    p.running = running && solver.is_some();
    p.step_ms = step_ms;
    match solver {
        Some(s) => {
            p.positions.clear();
            p.positions.extend_from_slice(s.positions());
            p.backend = s.backend();
            p.time = s.time();
            p.max_seam_gap = s.stats().max_seam_gap;
        }
        None => *p = Published::default(),
    }
}

fn run(rx: &Receiver<Control>, out: &Mutex<Published>) {
    let mut solver: Option<Box<dyn Solver>> = None;
    let mut failed: Option<String> = None;
    let mut running = false;
    let mut step_ms = 0.0;
    let frame = Duration::from_secs_f32(FRAME_DT);
    let mut next = Instant::now();
    loop {
        // Idle: block until told something. Running: wait only until the
        // next frame is due.
        let message = if running && solver.is_some() {
            match rx.recv_timeout(next.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => return,
            }
        };
        match message {
            Some(Control::Load(s)) => {
                solver = Some(s);
                failed = None;
                running = false;
            }
            Some(Control::Run(r)) => {
                running = r;
                next = Instant::now();
            }
            Some(Control::Grab(i)) => {
                if let Some(s) = solver.as_mut() {
                    s.grab(i);
                    if !running {
                        running = true;
                        next = Instant::now();
                    }
                }
            }
            Some(Control::GrabTo(target)) => {
                if let Some(s) = solver.as_mut() {
                    s.move_grab(target);
                }
            }
            Some(Control::Release) => {
                if let Some(s) = solver.as_mut() {
                    s.release();
                }
            }
            Some(Control::Clear) => {
                solver = None;
                failed = None;
                running = false;
            }
            Some(Control::Stop) => return,
            None => {
                if let Some(s) = solver.as_mut()
                    && failed.is_none()
                {
                    let start = Instant::now();
                    match s.step(FRAME_DT) {
                        Ok(()) => step_ms = start.elapsed().as_secs_f64() * 1000.0,
                        Err(e) => failed = Some(e.to_string()),
                    }
                }
                // Late frames drop simulated time rather than catch up.
                next = (next + frame).max(Instant::now());
            }
        }
        publish(out, solver.as_deref(), running, step_ms, failed.as_ref());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use glam::Vec2;
    use r3d_sim::scene::{Particle, Triangle};
    use r3d_sim::{CpuSolver, FabricPreset, SimScene};

    fn triangle() -> Box<dyn Solver> {
        let p = |x: f32, y: f32| Particle {
            position: Vec3::new(x, 1.0, y),
            rest: Vec2::new(x, y),
            pinned: false,
        };
        Box::new(CpuSolver::new(SimScene {
            particles: vec![p(0.0, 0.0), p(0.1, 0.0), p(0.0, 0.1)],
            triangles: vec![Triangle {
                vertices: [0, 1, 2],
                fabric: 0,
                warp: Vec2::Y,
            }],
            fabrics: vec![FabricPreset::Cotton.params()],
            ..SimScene::default()
        }))
    }

    fn wait_for(sim: &SimThread, what: impl Fn(&Published) -> bool) -> Published {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let p = sim.latest();
            if what(&p) || Instant::now() > deadline {
                return p;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn loads_paused_runs_and_clears() {
        let sim = SimThread::spawn().unwrap();
        sim.load(triangle());
        let p = wait_for(&sim, |p| p.loaded);
        assert!(p.loaded && !p.running && p.time == 0.0);
        sim.set_running(true);
        let p = wait_for(&sim, |p| p.time > 0.1);
        assert!(p.running && p.positions[0].y < 1.0, "{p:?}");
        sim.set_running(false);
        let t = wait_for(&sim, |p| !p.running).time;
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(sim.latest().time, t);
        sim.clear();
        assert!(!wait_for(&sim, |p| !p.loaded).loaded);
    }
}
