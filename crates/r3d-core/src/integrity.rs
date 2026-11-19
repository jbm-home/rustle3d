//! Structural invariants of a [`Project`].
//!
//! Checked after loading a file (a corrupted or hand-edited file is refused
//! instead of half-loaded) and after every command in tests.

use crate::id::{AnyId, EdgeId, PointId};
use crate::message::{Localizable, Message};
use crate::model::{Constraint, EdgeCurve, PathSegment, Project};

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
    #[error("{seam} is invalid (missing or unjoined edges, a bad range, or an edge sewn twice)")]
    BadSeam { seam: AnyId },
    #[error("{entity} has an invalid value")]
    BadValue { entity: AnyId },
    #[error(
        "the size range is invalid (fewer than two sizes, empty or repeated names, base or shown size out of range, or a base size that differs from the avatar)"
    )]
    BadGrading,
    #[error("{entity} has a grade that does not match the size range")]
    BadGrade { entity: AnyId },
    #[error("a variable has an invalid name or an empty formula")]
    BadVariables,
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
            Self::BadGrading => Message::new("integrity.bad_grading", text),
            Self::BadVariables => Message::new("integrity.bad_variables", text),
            Self::BadGrade { entity } => {
                Message::new("integrity.bad_grade", text).arg("entity", entity)
            }
            Self::BadMirror { panel } => {
                Message::new("integrity.bad_mirror", text).arg("panel", panel)
            }
        }
    }
}

impl Project {
    /// A seam side names existing geometry: consecutive contour (or hole)
    /// edges of one piece, or an internal line; a range within it.
    fn side_ok(&self, side: &crate::model::SeamSide) -> bool {
        let range_ok = [side.from_mm, side.to_mm]
            .iter()
            .flatten()
            .all(|v| v.is_finite() && *v >= 0.0)
            && match (side.from_mm, side.to_mm) {
                (Some(a), Some(b)) => a < b,
                _ => true,
            };
        let path_ok = match (side.edges.as_slice(), side.line) {
            ([], Some(line)) => self.internal_lines.contains_key(&line),
            ([first, ..], None) => {
                let edges: Vec<_> = side.edges.iter().map(|e| self.edges.get(e)).collect();
                let panel = self.edges.get(first).map(|e| e.panel);
                let mut seen = std::collections::BTreeSet::new();
                edges
                    .iter()
                    .all(|e| e.is_some_and(|e| Some(e.panel) == panel))
                    && side.edges.iter().all(|e| seen.insert(*e))
                    && edges.windows(2).all(|w| match w {
                        [Some(a), Some(b)] => a.end == b.start,
                        _ => false,
                    })
            }
            _ => false,
        };
        range_ok && path_ok
    }

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
            let print_ok = g.print.as_ref().is_none_or(|p| {
                let repeat = |v: f64| {
                    (crate::print::MIN_REPEAT_MM..=crate::print::MAX_REPEAT_MM).contains(&v)
                };
                repeat(p.width_mm)
                    && p.height_mm.is_none_or(repeat)
                    && p.angle_deg.is_finite()
                    && match &p.source {
                        crate::print::PrintSource::Builtin(_) => true,
                        crate::print::PrintSource::Image(path) => crate::print::image_path_ok(path),
                    }
            });
            if !hex_ok || !print_ok {
                return Err(E::BadValue {
                    entity: (*id).into(),
                });
            }
        }
        // Whole edges sewn twice; partial sides may share an edge (a pleat
        // sews one edge to itself), seam.add checks they do not overlap.
        let mut sewn = std::collections::BTreeSet::new();
        for (id, seam) in &self.seams {
            let ok = self.side_ok(&seam.a)
                && self.side_ok(&seam.b)
                && seam.a != seam.b
                && seam.ease_mm.is_finite()
                && [&seam.a, &seam.b]
                    .iter()
                    .filter_map(|s| s.whole_edge())
                    .all(|e| sewn.insert(e));
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
        for (id, b) in &self.buttons {
            let ok = [b.at, b.hole.at].iter().all(|p| p.is_finite())
                && b.hole.angle_deg.is_finite()
                && b.diameter_mm.is_finite()
                && b.diameter_mm > 0.0
                && self.panels.contains_key(&b.panel)
                && self.panels.contains_key(&b.hole.panel);
            if !ok {
                return Err(E::BadValue {
                    entity: (*id).into(),
                });
            }
        }
        for (id, z) in &self.zippers {
            let ok = self.side_ok(&z.a)
                && self.side_ok(&z.b)
                && z.a != z.b
                && z.open_mm.is_finite()
                && z.open_mm >= 0.0;
            if !ok {
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
                || a.mesh.as_ref().is_some_and(|m| !m.valid())
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
            .chain(self.pins.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.constraints.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.guides.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.buttons.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.zippers.keys().map(|k| (AnyId::from(*k), k.raw())));
        for (entity, raw) in ids {
            if raw >= self.next_id {
                return Err(E::IdCounter { entity });
            }
        }

        let sizes = match &self.grading {
            Some(g) => {
                let mut names = std::collections::BTreeSet::new();
                let named = g
                    .sizes
                    .iter()
                    .all(|s| !s.name.trim().is_empty() && names.insert(s.name.as_str()));
                let finite = g.sizes.iter().all(|s| {
                    [s.height_cm, s.bust_cm, s.waist_cm, s.hips_cm]
                        .iter()
                        .all(|v| v.is_finite())
                });
                let base_ok = g.sizes.get(g.base).is_some_and(|s| {
                    [s.height_cm, s.bust_cm, s.waist_cm, s.hips_cm]
                        .iter()
                        .all(|v| *v == 0.0)
                });
                if g.sizes.len() < 2 || !named || !finite || !base_ok || g.shown >= g.sizes.len() {
                    return Err(E::BadGrading);
                }
                Some((g.sizes.len(), g.base))
            }
            None => None,
        };
        for (id, point) in &self.points {
            let grade_ok = point.grade.is_empty()
                || sizes.is_some_and(|(n, base)| {
                    point.grade.len() == n
                        && point.grade.iter().all(|g| g.is_finite())
                        && point.grade.get(base) == Some(&glam::DVec2::ZERO)
                });
            if !grade_ok {
                return Err(E::BadGrade {
                    entity: (*id).into(),
                });
            }
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
            let grade_ok = edge.grade.is_empty()
                || (matches!(edge.curve, EdgeCurve::Cubic { .. })
                    && sizes.is_some_and(|(n, base)| {
                        edge.grade.len() == n
                            && edge.grade.iter().flatten().all(|g| g.is_finite())
                            && edge.grade.get(base) == Some(&[glam::DVec2::ZERO; 2])
                    }));
            if !grade_ok {
                return Err(E::BadGrade { entity });
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
            if !panel.grain_angle_deg.is_finite()
                || !panel.seam_allowance_mm.is_finite()
                || !panel.print.offset_mm.is_finite()
                || !panel.print.angle_deg.is_finite()
            {
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
            if let Some(sym) = &panel.symmetry {
                let own_point = |p: &PointId| self.points.get(p).is_some_and(|p| p.panel == *id);
                let own_edge = |e: &EdgeId| panel.contour.contains(e);
                let ok = panel.mirror_of.is_none()
                    && sym.origin.is_finite()
                    && sym.dir.is_finite()
                    && sym.dir.length_squared() > 0.0
                    && own_point(&sym.start)
                    && own_point(&sym.end)
                    && sym.points.iter().flatten().all(own_point)
                    && sym.edges.iter().flatten().all(own_edge);
                if !ok {
                    return Err(E::BadMirror { panel: pid });
                }
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
            for hole in &panel.holes {
                let mut loop_edges = Vec::with_capacity(hole.len());
                for eid in hole {
                    let Some(edge) = self.edges.get(eid).filter(|e| e.panel == *id) else {
                        return Err(E::OpenContour { panel: pid });
                    };
                    *usage.entry(*eid).or_insert(0usize) += 1;
                    loop_edges.push(edge);
                }
                let m = loop_edges.len();
                let closed = m >= 2
                    && (0..m).all(|i| {
                        loop_edges
                            .get(i)
                            .zip(loop_edges.get((i + 1) % m))
                            .is_some_and(|(a, b)| a.end == b.start)
                    });
                if !closed {
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

        for (id, c) in &self.constraints {
            let entity = AnyId::from(*id);
            let missing = c
                .points()
                .into_iter()
                .find(|p| !self.points.contains_key(p))
                .map(AnyId::from)
                .or_else(|| {
                    c.edges()
                        .into_iter()
                        .find(|e| !self.edges.contains_key(e))
                        .map(AnyId::from)
                });
            if let Some(missing) = missing {
                return Err(E::DanglingReference { entity, missing });
            }
            let derived = c.points().into_iter().any(|p| self.is_derived_point(p))
                || c.edges().into_iter().any(|e| self.is_derived_edge(e));
            let value_ok = match c {
                Constraint::Distance { mm, .. } | Constraint::Length { mm, .. } => {
                    mm.is_finite() && *mm > 0.0
                }
                Constraint::Angle { deg, .. } => (0.0..=180.0).contains(deg),
                _ => true,
            };
            if derived || !value_ok {
                return Err(E::BadValue { entity });
            }
        }

        if self
            .variables
            .keys()
            .any(|n| !crate::formula::valid_name(n))
            || self.variables.values().any(|f| f.trim().is_empty())
        {
            return Err(E::BadVariables);
        }
        for (id, g) in &self.guides {
            if !(g.a.is_finite() && g.b.is_finite()) || g.a == g.b {
                return Err(E::BadValue {
                    entity: (*id).into(),
                });
            }
        }
        Ok(())
    }
}
