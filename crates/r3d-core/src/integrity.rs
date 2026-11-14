//! Structural invariants of a [`Project`].
//!
//! Checked after loading a file (a corrupted or hand-edited file is refused
//! instead of half-loaded) and after every command in tests.

use crate::id::AnyId;
use crate::message::{Localizable, Message};
use crate::model::{EdgeCurve, Project};

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
        let ids = self
            .garments
            .keys()
            .map(|k| (AnyId::from(*k), k.raw()))
            .chain(self.panels.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.points.keys().map(|k| (AnyId::from(*k), k.raw())))
            .chain(self.edges.keys().map(|k| (AnyId::from(*k), k.raw())));
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
            if let EdgeCurve::Cubic { c1, c2 } = &edge.curve {
                if !c1.is_finite() || !c2.is_finite() {
                    return Err(E::NonFinite { entity });
                }
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
            if !panel.grain_angle_deg.is_finite() {
                return Err(E::NonFinite { entity: pid });
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
        Ok(())
    }
}
