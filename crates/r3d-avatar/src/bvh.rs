//! Reading BVH motion capture files.
//!
//! BVH (Biovision Hierarchy) is a plain text format: a tree of joints, each
//! with an offset from its parent and a list of channels, then one line of
//! channel values per frame. Rotation channels are Euler angles in degrees,
//! applied in the order they are listed (`Zrotation Xrotation Yrotation`
//! turns by Z × X × Y); the rest pose is every rotation at zero. The
//! format is described publicly by Meredith and Maddock, "Motion Capture
//! File Formats Explained", University of Sheffield (2001).

use glam::{Quat, Vec3};

use crate::captured::MotionError;

/// More joints than any human skeleton has.
pub const MAX_JOINTS: usize = 512;
/// About fifteen minutes at 120 frames a second.
pub const MAX_FRAMES: usize = 100_000;

/// What a channel of a joint holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Channel {
    Position,
    Rotation(Vec3),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    pub name: String,
    /// Always lower than the joint's own index.
    pub parent: Option<usize>,
    /// From the parent joint, in the rest pose (file units).
    pub offset: Vec3,
    pub channels: Vec<Channel>,
}

/// A motion capture: its joints and its frames.
#[derive(Debug, Clone, PartialEq)]
pub struct Bvh {
    pub joints: Vec<Joint>,
    /// Seconds between frames.
    pub frame_time: f32,
    /// One row of channel values per frame, the joints' channels in order.
    rows: Vec<Vec<f32>>,
}

fn format(detail: impl Into<String>) -> MotionError {
    MotionError::Format {
        detail: detail.into(),
    }
}

struct Tokens<'a> {
    inner: std::str::SplitWhitespace<'a>,
}

impl<'a> Tokens<'a> {
    fn next(&mut self) -> Result<&'a str, MotionError> {
        self.inner
            .next()
            .ok_or_else(|| format("unexpected end of file"))
    }

    fn expect(&mut self, word: &str) -> Result<(), MotionError> {
        let t = self.next()?;
        if t.eq_ignore_ascii_case(word) {
            Ok(())
        } else {
            Err(format(format!("expected {word}, found {t}")))
        }
    }

    fn number(&mut self) -> Result<f32, MotionError> {
        let t = self.next()?;
        t.parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format(format!("expected a number, found {t}")))
    }

    fn vec3(&mut self) -> Result<Vec3, MotionError> {
        Ok(Vec3::new(self.number()?, self.number()?, self.number()?))
    }
}

fn channel(name: &str) -> Result<Channel, MotionError> {
    let lower = name.to_ascii_lowercase();
    let axis = match lower.chars().next() {
        Some('x') => 0,
        Some('y') => 1,
        Some('z') => 2,
        _ => return Err(format(format!("unknown channel {name}"))),
    };
    match (lower.get(1..), Vec3::AXES.get(axis)) {
        (Some("position"), _) => Ok(Channel::Position),
        (Some("rotation"), Some(a)) => Ok(Channel::Rotation(*a)),
        _ => Err(format(format!("unknown channel {name}"))),
    }
}

impl Bvh {
    pub fn parse(text: &str) -> Result<Self, MotionError> {
        let mut t = Tokens {
            inner: text.split_whitespace(),
        };
        t.expect("HIERARCHY")?;
        let mut joints: Vec<Joint> = Vec::new();
        // The open blocks: a joint, or an end site (`None`).
        let mut open: Vec<Option<usize>> = Vec::new();
        loop {
            let word = t.next()?;
            match word.to_ascii_uppercase().as_str() {
                "ROOT" | "JOINT" => {
                    let is_root = word.eq_ignore_ascii_case("ROOT");
                    if is_root != open.is_empty() || (is_root && !joints.is_empty()) {
                        return Err(format(format!("misplaced {word}")));
                    }
                    if joints.len() >= MAX_JOINTS {
                        return Err(format("too many joints"));
                    }
                    let name = t.next()?.to_owned();
                    t.expect("{")?;
                    joints.push(Joint {
                        name,
                        parent: open.last().copied().flatten(),
                        offset: Vec3::ZERO,
                        channels: Vec::new(),
                    });
                    open.push(Some(joints.len() - 1));
                }
                "END" => {
                    t.expect("Site")?;
                    t.expect("{")?;
                    if open.is_empty() {
                        return Err(format("misplaced End Site"));
                    }
                    open.push(None);
                }
                "OFFSET" => {
                    let offset = t.vec3()?;
                    match open.last() {
                        Some(Some(i)) => {
                            if let Some(j) = joints.get_mut(*i) {
                                j.offset = offset;
                            }
                        }
                        Some(None) => {}
                        None => return Err(format("misplaced OFFSET")),
                    }
                }
                "CHANNELS" => {
                    let n = t.number()?;
                    let Some(Some(i)) = open.last().copied() else {
                        return Err(format("misplaced CHANNELS"));
                    };
                    if !(0.0..=6.0).contains(&n) || n.fract() != 0.0 {
                        return Err(format(format!("{n} channels")));
                    }
                    let channels = (0..n as usize)
                        .map(|_| channel(t.next()?))
                        .collect::<Result<Vec<_>, _>>()?;
                    if let Some(j) = joints.get_mut(i) {
                        j.channels = channels;
                    }
                }
                "}" => {
                    if open.pop().is_none() {
                        return Err(format("unbalanced }"));
                    }
                }
                "MOTION" if open.is_empty() && !joints.is_empty() => break,
                _ => return Err(format(format!("unexpected {word}"))),
            }
        }
        t.expect("Frames:")?;
        let frames = t.number()?;
        t.expect("Frame")?;
        t.expect("Time:")?;
        let frame_time = t.number()?;
        if frame_time <= 0.0 || frames < 2.0 || frames.fract() != 0.0 {
            return Err(format("needs at least two frames and a frame time"));
        }
        let frames = (frames as usize).min(MAX_FRAMES);
        let width: usize = joints.iter().map(|j| j.channels.len()).sum();
        let mut rows = Vec::with_capacity(frames);
        // A file cut short keeps its whole frames.
        'rows: for _ in 0..frames {
            let mut row = Vec::with_capacity(width);
            for _ in 0..width {
                match t.inner.next() {
                    Some(v) => row.push(
                        v.parse::<f32>()
                            .ok()
                            .filter(|v| v.is_finite())
                            .ok_or_else(|| format(format!("expected a number, found {v}")))?,
                    ),
                    None => break 'rows,
                }
            }
            rows.push(row);
        }
        if rows.len() < 2 {
            return Err(format("needs at least two frames"));
        }
        Ok(Self {
            joints,
            frame_time,
            rows,
        })
    }

    pub fn frames(&self) -> usize {
        self.rows.len()
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }

    /// Each joint's position in the rest pose (file units).
    pub fn rest_positions(&self) -> Vec<Vec3> {
        let mut out: Vec<Vec3> = Vec::with_capacity(self.joints.len());
        for j in &self.joints {
            let base = j
                .parent
                .and_then(|p| out.get(p))
                .copied()
                .unwrap_or(Vec3::ZERO);
            out.push(base + j.offset);
        }
        out
    }

    /// Each joint's position at `frame`, the root at the origin (file
    /// units): the rest offsets turned by the parents' rotations.
    pub fn posed_positions(&self, frame: usize) -> Vec<Vec3> {
        let rotations = self.world_rotations(frame);
        let mut out: Vec<Vec3> = Vec::with_capacity(self.joints.len());
        for j in &self.joints {
            let at = j
                .parent
                .and_then(|p| Some((*out.get(p)?, *rotations.get(p)?)));
            out.push(at.map_or(Vec3::ZERO, |(base, turn)| base + turn * j.offset));
        }
        out
    }

    /// Each joint's rotation from the rest pose at `frame`, in the file's
    /// axes (forward kinematics).
    pub fn world_rotations(&self, frame: usize) -> Vec<Quat> {
        let row = self.rows.get(frame).map_or(&[][..], Vec::as_slice);
        let mut values = row.iter();
        let mut out: Vec<Quat> = Vec::with_capacity(self.joints.len());
        for j in &self.joints {
            let mut local = Quat::IDENTITY;
            for c in &j.channels {
                let v = values.next().copied().unwrap_or(0.0);
                if let Channel::Rotation(axis) = c {
                    local *= Quat::from_axis_angle(*axis, v.to_radians());
                }
            }
            let parent = j
                .parent
                .and_then(|p| out.get(p))
                .copied()
                .unwrap_or(Quat::IDENTITY);
            out.push((parent * local).normalize());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const ARM: &str = "HIERARCHY
ROOT Hips
{
  OFFSET 0 0 0
  CHANNELS 6 Xposition Yposition Zposition Zrotation Yrotation Xrotation
  JOINT LeftArm
  {
    OFFSET 1 2 0
    CHANNELS 3 Zrotation Xrotation Yrotation
    End Site
    {
      OFFSET 3 0 0
    }
  }
}
MOTION
Frames: 2
Frame Time: 0.5
0 1 0  0 0 0  0 0 0
0 1 0  90 0 0  0 90 0
";

    #[test]
    fn reads_joints_and_turns_them_in_channel_order() {
        let bvh = Bvh::parse(ARM).unwrap();
        assert_eq!(bvh.joints.len(), 2);
        assert_eq!(bvh.frames(), 2);
        assert_eq!(bvh.frame_time, 0.5);
        assert_eq!(bvh.joints[1].parent, Some(0));
        assert_eq!(bvh.rest_positions()[1], Vec3::new(1.0, 2.0, 0.0));
        let rest = bvh.world_rotations(0);
        assert!(rest.iter().all(|q| q.abs_diff_eq(Quat::IDENTITY, 1e-6)));
        // The hips turn 90° about Z; the arm then 90° about its X: the
        // arm, along +X at rest, ends up along +Y.
        let q = bvh.world_rotations(1);
        let arm = q[1] * Vec3::X;
        assert!(arm.abs_diff_eq(Vec3::Y, 1e-5), "{arm}");
        let side = q[1] * Vec3::Y;
        assert!(side.abs_diff_eq(Vec3::Z, 1e-5), "{side}");
    }

    #[test]
    fn rejects_what_is_not_a_bvh_file() {
        for text in [
            "",
            "HIERARCHY ROOT a { OFFSET 0 0 0 CHANNELS 1 Wrotation } MOTION",
            "HIERARCHY ROOT a { OFFSET 0 0 0 } MOTION Frames: 1 Frame Time: 0.1 0",
            "HIERARCHY ROOT a { OFFSET 0 0 nan } MOTION Frames: 2 Frame Time: 0.1",
            "HIERARCHY } MOTION",
            "solid cube",
        ] {
            assert!(Bvh::parse(text).is_err(), "{text}");
        }
        // A file cut short keeps its whole frames.
        let short = format!("{}0 1 0 1 2", ARM.replace("Frames: 2", "Frames: 3"));
        assert_eq!(Bvh::parse(&short).unwrap().frames(), 2);
    }
}
