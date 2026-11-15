//! `cargo xtask fetch-makehuman`: build `assets/avatars/makehuman-body.r3dbody`
//! from MakeHuman's CC0 assets at a pinned commit.
//!
//! Only data files are downloaded (the base mesh and morph targets); none of
//! MakeHuman's AGPL code is used. The pack keeps the body only (no helper
//! geometry), in metres with the feet at y = 0.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use glam::Vec3;
use r3d_avatar::pack::{BodyPack, Target};

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
        positions,
        triangles,
        remap,
    })
}

fn parse_target(text: &str, remap: &BTreeMap<u32, u32>) -> BTreeMap<u32, Vec3> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let v: Vec<&str> = l.split_whitespace().collect();
            let index: u32 = v.first()?.parse().ok()?;
            let d: Vec<f32> = v.get(1..4)?.iter().filter_map(|x| x.parse().ok()).collect();
            let vertex = *remap.get(&index)?;
            (d.len() == 3).then(|| (vertex, Vec3::new(d[0], d[1], d[2]) * TO_METRES))
        })
        .collect()
}

fn to_target(name: &str, map: BTreeMap<u32, Vec3>) -> Target {
    Target {
        name: name.to_owned(),
        vertices: map.keys().copied().collect(),
        offsets: map.values().copied().collect(),
    }
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

pub fn run(root: &Path) -> Result<(), String> {
    let base = parse_base(&fetch("3dobjs/base.obj")?)?;
    let get = |path: &str| -> Result<BTreeMap<u32, Vec3>, String> {
        Ok(parse_target(
            &fetch(&format!("targets/{path}"))?,
            &base.remap,
        ))
    };
    let mut targets = Vec::new();
    let mut files = vec!["3dobjs/base.obj".to_owned()];
    for sex in ["female", "male"] {
        let ethnic: Vec<_> = ["african", "asian", "caucasian"]
            .iter()
            .map(|e| {
                let path = format!("macrodetails/{e}-{sex}-young.target");
                files.push(format!("targets/{path}"));
                get(&path)
            })
            .collect::<Result<_, _>>()?;
        targets.push(to_target(&format!("gender-{sex}"), average(&ethnic)));
        for w in ["min", "max"] {
            let path = format!("macrodetails/universal-{sex}-young-averagemuscle-{w}weight.target");
            targets.push(to_target(&format!("weight-{sex}-{w}"), get(&path)?));
            files.push(format!("targets/{path}"));
            let path = format!(
                "macrodetails/height/{sex}-young-averagemuscle-averageweight-{w}height.target"
            );
            targets.push(to_target(&format!("height-{sex}-{w}"), get(&path)?));
            files.push(format!("targets/{path}"));
        }
    }
    for m in MEASURES {
        for dir in ["decr", "incr"] {
            let name = format!("measure-{m}-{dir}");
            targets.push(to_target(&name, get(&format!("measure/{name}.target"))?));
            files.push(format!("targets/measure/{name}.target"));
        }
    }
    let pack = BodyPack {
        provenance: format!(
            "MakeHuman assets, CC0 1.0 Universal (https://github.com/makehumancommunity/makehuman, \
             commit {COMMIT}). Body only, metres, feet at y = 0; gender targets are the average of \
             the african/asian/caucasian young targets. Files: {}",
            files.join(", ")
        ),
        positions: base.positions,
        triangles: base.triangles,
        targets,
    };
    let out = root.join("assets/avatars/makehuman-body.r3dbody");
    let bytes = pack.to_bytes();
    std::fs::write(&out, &bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!(
        "wrote {} ({} vertices, {} triangles, {} targets, {} KiB)",
        out.display(),
        pack.positions.len(),
        pack.triangles.len(),
        pack.targets.len(),
        bytes.len() / 1024
    );
    Ok(())
}
