//! Entities carried over to other IDs: what a copy keeps (the IDs it
//! holds) and what a paste gives (fresh IDs). Each `remapped` returns
//! `None` when an entity names something the mapping has not.

use crate::detail::{Detail, InterfacingArea};
use crate::id::{EdgeId, GuideId, InternalLineId, NotchId, PanelId, PointId};
use crate::model::{Constraint, SeamSide};
use crate::trim::{Button, Buttonhole, Zipper};

/// Each entity's replacement; `None`: not carried over.
pub trait Remap {
    fn panel(&self, id: PanelId) -> Option<PanelId>;
    fn point(&self, id: PointId) -> Option<PointId>;
    fn edge(&self, id: EdgeId) -> Option<EdgeId>;
    fn line(&self, id: InternalLineId) -> Option<InternalLineId>;
    fn notch(&self, id: NotchId) -> Option<NotchId>;
    fn guide(&self, id: GuideId) -> Option<GuideId>;
}

impl SeamSide {
    pub fn remapped(&self, m: &impl Remap) -> Option<SeamSide> {
        Some(SeamSide {
            edges: self
                .edges
                .iter()
                .map(|e| m.edge(*e))
                .collect::<Option<_>>()?,
            line: match self.line {
                Some(l) => Some(m.line(l)?),
                None => None,
            },
            ..self.clone()
        })
    }
}

impl Detail {
    pub fn remapped(&self, m: &impl Remap) -> Option<Detail> {
        let mut d = self.clone();
        match &mut d {
            Detail::Elastic(e) => e.side = e.side.remapped(m)?,
            Detail::Topstitch(t) => t.side = t.side.remapped(m)?,
            Detail::Interfacing(i) => {
                i.panel = m.panel(i.panel)?;
                match &mut i.area {
                    InterfacingArea::Whole => {}
                    InterfacingArea::Band { edges, .. } => {
                        *edges = edges.iter().map(|e| m.edge(*e)).collect::<Option<_>>()?;
                    }
                    InterfacingArea::Inside { line } => *line = m.line(*line)?,
                }
            }
            Detail::Quilting(q) => {
                q.panel = m.panel(q.panel)?;
                if let Some(line) = &mut q.line {
                    *line = m.line(*line)?;
                }
            }
        }
        Some(d)
    }
}

impl Button {
    pub fn remapped(&self, m: &impl Remap) -> Option<Button> {
        Some(Button {
            panel: m.panel(self.panel)?,
            hole: Buttonhole {
                panel: m.panel(self.hole.panel)?,
                ..self.hole.clone()
            },
            ..self.clone()
        })
    }
}

impl Zipper {
    pub fn remapped(&self, m: &impl Remap) -> Option<Zipper> {
        Some(Zipper {
            a: self.a.remapped(m)?,
            b: self.b.remapped(m)?,
            ..self.clone()
        })
    }
}

impl Constraint {
    /// The same constraint on other points, edges, notches and
    /// construction lines, its name kept.
    pub fn remapped(&self, m: &impl Remap) -> Option<Constraint> {
        let mut c = self.clone();
        match &mut c {
            Constraint::Fixed { point } | Constraint::Tangent { point } => {
                *point = m.point(*point)?
            }
            Constraint::Coincident { a, b } | Constraint::Distance { a, b, .. } => {
                *a = m.point(*a)?;
                *b = m.point(*b)?;
            }
            Constraint::Horizontal { edge }
            | Constraint::Vertical { edge }
            | Constraint::Length { edge, .. } => *edge = m.edge(*edge)?,
            Constraint::OnEdge { point, edge } => {
                *point = m.point(*point)?;
                *edge = m.edge(*edge)?;
            }
            Constraint::Parallel { a, b }
            | Constraint::Perpendicular { a, b }
            | Constraint::EqualLength { a, b }
            | Constraint::Angle { a, b, .. } => {
                *a = m.edge(*a)?;
                *b = m.edge(*b)?;
            }
            Constraint::NotchDistance { notch, .. } => *notch = m.notch(*notch)?,
            Constraint::OnGuide { point, guide }
            | Constraint::GuideDistance { point, guide, .. } => {
                *point = m.point(*point)?;
                *guide = m.guide(*guide)?;
            }
        }
        Some(c)
    }
}
