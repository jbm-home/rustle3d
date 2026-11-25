//! `cargo xtask fetch-makehuman`: build `assets/avatars/makehuman-body.r3dbody`
//! from MakeHuman's CC0 assets at a pinned commit.
//!
//! Only data files are downloaded (the base mesh, morph targets, the
//! default skeleton and its weights); none of MakeHuman's AGPL code is
//! used. The pack keeps the body only (no helper geometry), in metres with
//! the feet at y = 0, and a reduced skeleton: fingers, toes, face and
//! twist bones are merged into the bone they hang from.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use glam::Vec3;
use r3d_avatar::pack::{BodyPack, Bone, Influences, Target};
use serde_json::Value;

/// MakeHuman commit the assets are taken from.
const COMMIT: &str = "a8bc2d54ff0ac92e78ff71431b1023eda42bf482";
const BASE: &str = "https://raw.githubusercontent.com/makehumancommunity/makehuman";
/// MakeHuman units are decimetres.
const TO_METRES: f32 = 0.1;

const MEASURES: [&str; 14] = [
    "bust-circ",
    "underbust-circ",
    "waist-circ",
    "hips-circ",
    "neck-circ",
    "upperarm-circ",
    "thigh-circ",
    "shoulder-dist",
    "upperleg-height",
    "lowerleg-height",
    "napetowaist-dist",
    "waisttohip-dist",
    "upperarm-length",
    "lowerarm-length",
];

/// The reduced skeleton: our bone name, its parent, and the MakeHuman
/// bones whose head and tail joints it spans. Parents come first.
const BONES: [(&str, Option<&str>, &str, &str); 24] = [
    ("pelvis", None, "root", "root"),
    ("spine1", Some("pelvis"), "spine05", "spine05"),
    ("spine2", Some("spine1"), "spine04", "spine04"),
    ("spine3", Some("spine2"), "spine03", "spine03"),
    ("chest", Some("spine3"), "spine02", "spine02"),
    ("upper_chest", Some("chest"), "spine01", "spine01"),
    ("neck", Some("upper_chest"), "neck01", "neck03"),
    ("head", Some("neck"), "head", "head"),
    (
        "clavicle_l",
        Some("upper_chest"),
        "clavicle.L",
        "clavicle.L",
    ),
    (
        "shoulder_l",
        Some("clavicle_l"),
        "shoulder01.L",
        "shoulder01.L",
    ),
    (
        "upper_arm_l",
        Some("shoulder_l"),
        "upperarm01.L",
        "upperarm02.L",
    ),
    (
        "forearm_l",
        Some("upper_arm_l"),
        "lowerarm01.L",
        "lowerarm02.L",
    ),
    ("hand_l", Some("forearm_l"), "wrist.L", "wrist.L"),
    (
        "clavicle_r",
        Some("upper_chest"),
        "clavicle.R",
        "clavicle.R",
    ),
    (
        "shoulder_r",
        Some("clavicle_r"),
        "shoulder01.R",
        "shoulder01.R",
    ),
    (
        "upper_arm_r",
        Some("shoulder_r"),
        "upperarm01.R",
        "upperarm02.R",
    ),
    (
        "forearm_r",
        Some("upper_arm_r"),
        "lowerarm01.R",
        "lowerarm02.R",
    ),
    ("hand_r", Some("forearm_r"), "wrist.R", "wrist.R"),
    ("thigh_l", Some("pelvis"), "upperleg01.L", "upperleg02.L"),
    ("shin_l", Some("thigh_l"), "lowerleg01.L", "lowerleg02.L"),
    ("foot_l", Some("shin_l"), "foot.L", "foot.L"),
    ("thigh_r", Some("pelvis"), "upperleg01.R", "upperleg02.R"),
    ("shin_r", Some("thigh_r"), "lowerleg01.R", "lowerleg02.R"),
    ("foot_r", Some("shin_r"), "foot.R", "foot.R"),
];

/// Skin weights below this are dropped (before keeping the four largest).
const MIN_WEIGHT: f32 = 1e-3;

fn fetch(path: &str) -> Result<String, String> {
    let url = format!("{BASE}/{COMMIT}/makehuman/data/{path}");
    eprintln!("fetch {path}");
    ureq::get(&url)
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("{url}: {e}"))
}

struct Base {
    /// Every OBJ vertex, helpers included (metres, before the floor shift).
    all: Vec<Vec3>,
    /// Height of the body's lowest vertex, subtracted from everything.
    floor: f32,
    positions: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    /// Original OBJ vertex index → pack vertex index.
    remap: BTreeMap<u32, u32>,
}

fn parse_base(obj: &str) -> Result<Base, String> {
    let mut all = Vec::new();
    let mut faces: Vec<Vec<u32>> = Vec::new();
    let mut group = "";
    for line in obj.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => {
                let c: Vec<f32> = parts.take(3).filter_map(|v| v.parse().ok()).collect();
                if c.len() != 3 {
                    return Err(format!("bad vertex line: {line}"));
                }
                all.push(Vec3::new(c[0], c[1], c[2]) * TO_METRES);
            }
            Some("g") => group = parts.next().unwrap_or(""),
            Some("f") if group == "body" => {
                let face = parts
                    .filter_map(|t| t.split('/').next()?.parse::<u32>().ok())
                    .map(|i| i - 1)
                    .collect();
                faces.push(face);
            }
            _ => {}
        }
    }
    let used: BTreeSet<u32> = faces.iter().flatten().copied().collect();
    let remap: BTreeMap<u32, u32> = used
        .iter()
        .enumerate()
        .map(|(n, o)| (*o, n as u32))
        .collect();
    let mut positions: Vec<Vec3> = used.iter().map(|o| all[*o as usize]).collect();
    let floor = positions.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    for p in &mut positions {
        p.y -= floor;
    }
    let mut triangles = Vec::new();
    for f in &faces {
        let idx: Vec<u32> = f.iter().map(|o| remap[o]).collect();
        for k in 1..idx.len().saturating_sub(1) {
            triangles.push([idx[0], idx[k], idx[k + 1]]);
        }
    }
    Ok(Base {
        all,
        floor,
        positions,
        triangles,
        remap,
    })
}

/// A target file: OBJ vertex index → offset (m).
fn parse_target(text: &str) -> BTreeMap<u32, Vec3> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let v: Vec<&str> = l.split_whitespace().collect();
            let index: u32 = v.first()?.parse().ok()?;
            let d: Vec<f32> = v.get(1..4)?.iter().filter_map(|x| x.parse().ok()).collect();
            (d.len() == 3).then(|| (index, Vec3::new(d[0], d[1], d[2]) * TO_METRES))
        })
        .collect()
}

/// Average of several targets (missing vertices count as zero offset).
fn average(maps: &[BTreeMap<u32, Vec3>]) -> BTreeMap<u32, Vec3> {
    let mut sum: BTreeMap<u32, Vec3> = BTreeMap::new();
    for m in maps {
        for (v, o) in m {
            *sum.entry(*v).or_default() += *o;
        }
    }
    let n = maps.len().max(1) as f32;
    sum.into_iter().map(|(v, o)| (v, o / n)).collect()
}

/// The reduced skeleton read from MakeHuman's default rig.
struct Rig {
    /// For each pack joint, the OBJ vertices whose mean it is.
    joints: Vec<Vec<u32>>,
    bones: Vec<Bone>,
    /// MakeHuman bone name → our bone index.
    owner: BTreeMap<String, u8>,
}

fn parse_rig(skel: &Value) -> Result<Rig, String> {
    let mh_bones = skel["bones"].as_object().ok_or("mhskel: no bones")?;
    let mh_joints = skel["joints"].as_object().ok_or("mhskel: no joints")?;
    let mut joint_names: Vec<String> = Vec::new();
    let mut joint = |name: String| -> Result<u32, String> {
        if let Some(i) = joint_names.iter().position(|n| *n == name) {
            return Ok(i as u32);
        }
        if !mh_joints.contains_key(&name) {
            return Err(format!("mhskel: no joint {name}"));
        }
        joint_names.push(name);
        Ok(joint_names.len() as u32 - 1)
    };
    let mut bones = Vec::new();
    let mut kept: BTreeMap<&str, u8> = BTreeMap::new();
    for (i, (name, parent, head, tail)) in BONES.iter().enumerate() {
        let parent = parent
            .map(|p| {
                BONES
                    .iter()
                    .position(|b| b.0 == p)
                    .map(|k| k as u32)
                    .ok_or(format!("no bone {p}"))
            })
            .transpose()?;
        let joint_of = |bone: &str, end: &str| -> Result<String, String> {
            mh_bones
                .get(bone)
                .and_then(|b| b[end].as_str())
                .map(str::to_owned)
                .ok_or(format!("mhskel: no bone {bone}"))
        };
        bones.push(Bone {
            name: (*name).to_owned(),
            parent,
            head: joint(joint_of(head, "head")?)?,
            tail: joint(joint_of(tail, "tail")?)?,
        });
        kept.insert(head, i as u8);
    }
    // Every MakeHuman bone belongs to the nearest kept bone up its chain.
    let mut owner = BTreeMap::new();
    for name in mh_bones.keys() {
        let mut at = name.as_str();
        let found = loop {
            if let Some(i) = kept.get(at) {
                break Some(*i);
            }
            match mh_bones.get(at).and_then(|b| b["parent"].as_str()) {
                Some(p) => at = p,
                None => break None,
            }
        };
        if let Some(i) = found {
            owner.insert(name.clone(), i);
        }
    }
    let joints = joint_names
        .iter()
        .map(|n| {
            mh_joints[n]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_u64())
                        .map(|v| v as u32)
                        .collect()
                })
                .ok_or(format!("mhskel: bad joint {n}"))
        })
        .collect::<Result<_, String>>()?;
    Ok(Rig {
        joints,
        bones,
        owner,
    })
}

/// Each body vertex's four largest bone weights, normalised.
fn skin_weights(weights: &Value, rig: &Rig, base: &Base) -> Result<Vec<Influences>, String> {
    let per_bone = weights["weights"].as_object().ok_or("mhw: no weights")?;
    let mut sums: Vec<BTreeMap<u8, f32>> = vec![BTreeMap::new(); base.positions.len()];
    for (bone, list) in per_bone {
        let Some(owner) = rig.owner.get(bone) else {
            return Err(format!("mhw: bone {bone} not in the skeleton"));
        };
        for pair in list.as_array().ok_or("mhw: bad list")? {
            let (Some(v), Some(w)) = (pair[0].as_u64(), pair[1].as_f64()) else {
                return Err(format!("mhw: bad weight in {bone}"));
            };
            if let Some(i) = base.remap.get(&(v as u32)) {
                *sums[*i as usize].entry(*owner).or_default() += w as f32;
            }
        }
    }
    sums.iter()
        .enumerate()
        .map(|(v, s)| {
            let mut list: Vec<(u8, f32)> = s
                .iter()
                .map(|(b, w)| (*b, *w))
                .filter(|(_, w)| *w > MIN_WEIGHT)
                .collect();
            list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            list.truncate(4);
            let total: f32 = list.iter().map(|(_, w)| w).sum();
            if total <= 0.0 {
                return Err(format!("vertex {v} has no skin weight"));
            }
            let mut out = Influences::default();
            for (k, (b, w)) in list.iter().enumerate() {
                out.bones[k] = *b;
                out.weights[k] = w / total;
            }
            Ok(out)
        })
        .collect()
}

/// A target's offsets for the body's vertices and for the joints.
fn to_target(name: &str, map: &BTreeMap<u32, Vec3>, base: &Base, rig: &Rig) -> Target {
    let mut target = Target {
        name: name.to_owned(),
        ..Target::default()
    };
    for (o, d) in map {
        if let Some(v) = base.remap.get(o) {
            target.vertices.push(*v);
            target.offsets.push(*d);
        }
    }
    for (j, verts) in rig.joints.iter().enumerate() {
        let sum: Vec3 = verts.iter().filter_map(|v| map.get(v)).sum();
        if sum != Vec3::ZERO {
            target.joints.push(j as u32);
            target.joint_offsets.push(sum / verts.len().max(1) as f32);
        }
    }
    target
}

pub fn run(root: &Path) -> Result<(), String> {
    let base = parse_base(&fetch("3dobjs/base.obj")?)?;
    let skel: Value = serde_json::from_str(&fetch("rigs/default.mhskel")?)
        .map_err(|e| format!("default.mhskel: {e}"))?;
    let skin: Value = serde_json::from_str(&fetch("rigs/default_weights.mhw")?)
        .map_err(|e| format!("default_weights.mhw: {e}"))?;
    let rig = parse_rig(&skel)?;
    let weights = skin_weights(&skin, &rig, &base)?;
    let joints: Vec<Vec3> = rig
        .joints
        .iter()
        .map(|verts| {
            let sum: Vec3 = verts.iter().filter_map(|v| base.all.get(*v as usize)).sum();
            sum / verts.len().max(1) as f32 - Vec3::Y * base.floor
        })
        .collect();

    let get = |path: &str| -> Result<BTreeMap<u32, Vec3>, String> {
        Ok(parse_target(&fetch(&format!("targets/{path}"))?))
    };
    let mut targets = Vec::new();
    let mut files = vec![
        "3dobjs/base.obj".to_owned(),
        "rigs/default.mhskel".to_owned(),
        "rigs/default_weights.mhw".to_owned(),
    ];
    // The adult (MakeHuman's "young", 25 years) targets keep their names;
    // a child's (11 years) and a baby's (1 year) take the age as a suffix.
    for sex in ["female", "male"] {
        for (age, suffix) in [("young", ""), ("child", "-child"), ("baby", "-baby")] {
            let ethnic: Vec<_> = ["african", "asian", "caucasian"]
                .iter()
                .map(|e| {
                    let path = format!("macrodetails/{e}-{sex}-{age}.target");
                    files.push(format!("targets/{path}"));
                    get(&path)
                })
                .collect::<Result<_, _>>()?;
            targets.push(to_target(
                &format!("gender-{sex}{suffix}"),
                &average(&ethnic),
                &base,
                &rig,
            ));
            for w in ["min", "max"] {
                let path =
                    format!("macrodetails/universal-{sex}-{age}-averagemuscle-{w}weight.target");
                targets.push(to_target(
                    &format!("weight-{sex}{suffix}-{w}"),
                    &get(&path)?,
                    &base,
                    &rig,
                ));
                files.push(format!("targets/{path}"));
                let path = format!(
                    "macrodetails/height/{sex}-{age}-averagemuscle-averageweight-{w}height.target"
                );
                targets.push(to_target(
                    &format!("height-{sex}{suffix}-{w}"),
                    &get(&path)?,
                    &base,
                    &rig,
                ));
                files.push(format!("targets/{path}"));
            }
        }
    }
    for m in MEASURES {
        for dir in ["decr", "incr"] {
            let name = format!("measure-{m}-{dir}");
            let map = get(&format!("measure/{name}.target"))?;
            targets.push(to_target(&name, &map, &base, &rig));
            files.push(format!("targets/measure/{name}.target"));
        }
    }
    let pack = BodyPack {
        provenance: format!(
            "MakeHuman assets, CC0 1.0 Universal (https://github.com/makehumancommunity/makehuman, \
             commit {COMMIT}). Body only, metres, feet at y = 0; gender targets are the average of \
             the african/asian/caucasian targets of each age (young, child, baby); the default skeleton reduced to {} bones \
             (fingers, toes, face and twist bones merged into the bone they hang from), four skin \
             weights per vertex. Files: {}",
            rig.bones.len(),
            files.join(", ")
        ),
        positions: base.positions,
        triangles: base.triangles,
        joints,
        bones: rig.bones,
        weights,
        targets,
    };
    let out = root.join("assets/avatars/makehuman-body.r3dbody");
    let bytes = pack.to_bytes();
    std::fs::write(&out, &bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!(
        "wrote {} ({} vertices, {} triangles, {} bones, {} targets, {} KiB)",
        out.display(),
        pack.positions.len(),
        pack.triangles.len(),
        pack.bones.len(),
        pack.targets.len(),
        bytes.len() / 1024
    );
    Ok(())
}
