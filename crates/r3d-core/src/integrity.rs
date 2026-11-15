//! Structural invariants of a [`Project`].
//!
//! Checked after loading a file (a corrupted or hand-edited file is refused
//! instead of half-loaded) and after every command in tests.

use crate::id::AnyId;
use crate::message::{Localizable, Message};
use crate::model::{EdgeCurve, PathSegment, Project};

/// A broken invariant.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum IntegrityError {
    #[error("{entity} references missing {missing}")]
    DanglingReference { entity: AnyId, missing: AnyId },
    #[error("{entity} belongs to a different panel than its owner")]
    WrongOwner { entity: AnyId },
    #[error("{panel} contour is not a closed loop")]
    OpenContour { panel: AnyId },
    #[error("{panel} contour needs at least 2 edges")]
    ContourTooShort { panel: AnyId },
    #[error("{edge} is not used exactly once by its panel contour")]
    EdgeUsage { edge: AnyId },
    #[error("{entity} has a non-finite coordinate")]
    NonFinite { entity: AnyId },
    #[error("{entity} has an ID not below the document's next ID")]
    IdCounter { entity: AnyId },
    #[error("the project has no garment")]
    NoGarment,
    #[error("{entity} has an out-of-range value")]
    OutOfRange { entity: AnyId },
    #[error("{panel} has an invalid mirror link (missing axis, self or chained source)")]
    BadMirror { panel: AnyId },
    #[error("the avatar has invalid measurements")]
    BadAvatar,
    #[error("{seam} is invalid (missing edge, same edge twice, or an edge sewn twice)")]
    BadSeam { seam: AnyId },
    #[error("{entity} has an invalid value")]
    BadValue { entity: AnyId },
}

impl Localizable for IntegrityError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::DanglingReference { entity, missing } => {
                Message::new("integrity.dangling_reference", text)
                    .arg("entity", entity)
                    .arg("missing", missing)
            }
            Self::WrongOwner { entity } => {
                Message::new("integrity.wrong_owner", text).arg("entity", entity)
            }
            Self::OpenContour { panel } => {
                Message::new("integrity.open_contour", text).arg("panel", panel)
            }
            Self::ContourTooShort { panel } => {
                Message::new("integrity.contour_too_short", text).arg("panel", panel)
            }
            Self::EdgeUsage { edge } => {
                Message::new("integrity.edge_usage", text).arg("edge", edge)
            }
            Self::NonFinite { entity } => {
                Message::new("integrity.non_finite", text).arg("entity", entity)
            }
            Self::IdCounter { entity } => {
                Message::new("integrity.id_counter", text).arg("entity", entity)
            }
            Self::NoGarment => Message::new("integrity.no_garment", text),
            Self::OutOfRange { entity } => {
                Message::new("integrity.out_of_range", text).arg("entity", entity)
            }
            Self::BadAvatar => Message::new("integrity.bad_avatar", text),
            Self::BadSeam { seam } => Message::new("integrity.bad_seam", text).arg("seam", seam),
            Self::BadValue { entity } => {
                Message::new("integrity.bad_value", text).arg("entity", entity)
            }
            Self::BadMirror { panel } => {
                Message::new("integrity.bad_mirror", text).arg("panel", panel)
            }
        }
    }
}

impl Project {
    /// Check every structural invariant.
    pub fn check_integrity(&self) -> Result<(), IntegrityError> {
        use IntegrityError as E;
        if self.garments.is_empty() {
            return Err(E::NoGarment);
        }
        for (id, g) in &self.garments {
            let hex_ok = g.color.as_ref().is_none_or(|c| {
                c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit())
            });
            if !hex_ok {
                return Err(E::BadValue {
                    entity: (*id).into(),
                });
            }
        }
        let mut sewn = std::collections::BTreeSet::new();
        for (id, seam) in &self.seams {
            let ok = seam.a.edge != seam.b.edge
                && self.edges.contains_key(&seam.a.edge)
                && self.edges.contains_key(&seam.b.edge)
                && sewn.insert(seam.a.edge)
                && sewn.insert(seam.b.edge);
            if !ok {
                return Err(E::BadSeam { seam: (*id).into() });
            }
        }
        for (id, pin) in &self.pins {
            let finite = pin.at.is_finite() && pin.position.is_finite();
            if !finite || !self.panels.contains_key(&pin.panel) {
                return Err(E::BadValue {
                    entity: (*id).into(),
                });
            }
        }
        for (id, panel) in &self.panels {
            if let Some(p) = &panel.placement
                && ![p.angle_deg, p.top_cm, p.offset_mm]
                    .iter()
                    .all(|v| v.is_finite())
            {
                return Err(E::BadValue {
                    entity: (*id).into(),
                });
            }
        }
        if let Some(a) = &self.avatar {
            let b = a.body;
            let sizes = [b.height_cm, b.bust_cm, b.waist_cm, b.hips_cm];
            if !(0.0..=1.0).contains(&b.masculinity)
                || sizes.iter().any(|v| !(v.is_finite() && *v > 0.0))
            {
                return Err(E::BadAvatar);
            }
        }
        let ids = self
            .garments
            .keys()
            .map(|k| (AnyId::from(*k), k.raw()))
            .chain(self.panels.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.points.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.edges.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.notches.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(
                self.internal_lines
                    .keys()
                    .map(|k| (AnyId::from(*k), k.raw())),
            )
            .chain(self.seams.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.pins.keys().map(|k| (AnyId::from(*k), k.raw())));
        for (entity, raw) in ids {
            if raw >= self.next_id {
                return Err(E::IdCounter { entity });
            }
        }

        for (id, point) in &self.points {
            if !self.panels.contains_key(&point.panel) {
                return Err(E::DanglingReference {
                    entity: (*id).into(),
                    missing: point.panel.into(),
                });
            }
            if !point.pos.is_finite() {
                return Err(E::NonFinite {
                    entity: (*id).into(),
                });
            }
        }

        for (id, edge) in &self.edges {
            let entity = AnyId::from(*id);
            if !self.panels.contains_key(&edge.panel) {
                return Err(E::DanglingReference {
                    entity,
                    missing: edge.panel.into(),
                });
            }
            for pid in [edge.start, edge.end] {
                let Some(pt) = self.points.get(&pid) else {
                    return Err(E::DanglingReference {
                        entity,
                        missing: pid.into(),
                    });
                };
                if pt.panel != edge.panel {
                    return Err(E::WrongOwner { entity });
                }
            }
            if let EdgeCurve::Cubic { c1, c2 } = &edge.curve
                && (!c1.is_finite() || !c2.is_finite())
            {
                return Err(E::NonFinite { entity });
            }
        }

        let mut usage = std::collections::BTreeMap::new();
        for (id, panel) in &self.panels {
            let pid = AnyId::from(*id);
            if !self.garments.contains_key(&panel.garment) {
                return Err(E::DanglingReference {
                    entity: pid,
                    missing: panel.garment.into(),
                });
            }
            if !panel.grain_angle_deg.is_finite() || !panel.seam_allowance_mm.is_finite() {
                return Err(E::NonFinite { entity: pid });
            }
            if panel.seam_allowance_mm < 0.0 {
                return Err(E::OutOfRange { entity: pid });
            }
            if let Some(link) = &panel.mirror_of {
                let source_ok = link.source != *id
                    && self
                        .panels
                        .get(&link.source)
                        .is_some_and(|s| s.mirror_of.is_none());
                let axis_ok = link.origin.is_finite()
                    && link.offset.is_finite()
                    && link.dir.is_finite()
                    && link.dir.length_squared() > 0.0;
                if !source_ok || !axis_ok {
                    return Err(E::BadMirror { panel: pid });
                }
            }
            if panel.contour.len() < 2 {
                return Err(E::ContourTooShort { panel: pid });
            }
            let mut edges = Vec::with_capacity(panel.contour.len());
            for eid in &panel.contour {
                let Some(edge) = self.edges.get(eid) else {
                    return Err(E::DanglingReference {
                        entity: pid,
                        missing: (*eid).into(),
                    });
                };
                if edge.panel != *id {
                    return Err(E::WrongOwner {
                        entity: (*eid).into(),
                    });
                }
                *usage.entry(*eid).or_insert(0usize) += 1;
                edges.push(edge);
            }
            let n = edges.len();
            for i in 0..n {
                if edges[i].end != edges[(i + 1) % n].start {
                    return Err(E::OpenContour { panel: pid });
                }
            }
        }
        for id in self.edges.keys() {
            if usage.get(id).copied() != Some(1) {
                return Err(E::EdgeUsage { edge: (*id).into() });
            }
        }

        for (id, notch) in &self.notches {
            let entity = AnyId::from(*id);
            if !self.edges.contains_key(&notch.edge) {
                return Err(E::DanglingReference {
                    entity,
                    missing: notch.edge.into(),
                });
            }
            if !notch.offset_mm.is_finite() {
                return Err(E::NonFinite { entity });
            }
            if notch.offset_mm < 0.0 {
                return Err(E::OutOfRange { entity });
            }
        }

        for (id, line) in &self.internal_lines {
            let entity = AnyId::from(*id);
            if !self.panels.contains_key(&line.panel) {
                return Err(E::DanglingReference {
                    entity,
                    missing: line.panel.into(),
                });
            }
            if line.segments.is_empty() {
                return Err(E::OutOfRange { entity });
            }
            let finite = line.start.is_finite()
                && line.segments.iter().all(|s| match s {
                    PathSegment::Line { to } => to.is_finite(),
                    PathSegment::Cubic { c1, c2, to } => {
                        c1.is_finite() && c2.is_finite() && to.is_finite()
                    }
                });
            if !finite {
                return Err(E::NonFinite { entity });
            }
        }
        Ok(())
    }
}
