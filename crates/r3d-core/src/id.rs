//! Stable typed identifiers.
//!
//! IDs serialize as self-describing strings (`"panel:3"`), so that an agent
//! or a script cannot pass a point where a panel is expected. All entity
//! kinds share one monotonic counter per document: an ID is unique across
//! kinds and never reused while it can still be referenced.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Error for a string that is not a valid ID.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid id `{text}`: expected `{expected}:<number>`")]
pub struct ParseIdError {
    pub text: String,
    pub expected: &'static str,
}

fn parse_raw(s: &str, prefix: &'static str) -> Result<u64, ParseIdError> {
    s.split_once(':')
        .filter(|(p, _)| *p == prefix)
        .and_then(|(_, n)| n.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| ParseIdError {
            text: s.to_owned(),
            expected: prefix,
        })
}

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $variant:ident, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            pub const PREFIX: &'static str = $prefix;

            pub fn raw(self) -> u64 {
                self.0
            }

            pub(crate) fn from_raw(raw: u64) -> Self {
                Self(raw)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}:{}", $prefix, self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }
        }

        impl FromStr for $name {
            type Err = ParseIdError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                parse_raw(s, $prefix).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                s.parse().map_err(de::Error::custom)
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({
                    "type": "string",
                    "pattern": concat!("^", $prefix, ":[1-9][0-9]*$"),
                    "description": concat!("Identifier of a ", $prefix, ", e.g. `", $prefix, ":1`."),
                })
            }
        }

        impl From<$name> for AnyId {
            fn from(id: $name) -> AnyId {
                AnyId::$variant(id)
            }
        }
    };
}

define_id!(
    /// A garment: a group of panels worn together.
    GarmentId,
    Garment,
    "garment"
);
define_id!(
    /// A flat pattern panel.
    PanelId,
    Panel,
    "panel"
);
define_id!(
    /// A 2D point on a panel contour.
    PointId,
    Point,
    "point"
);
define_id!(
    /// An edge (line or curve) between two points of a panel contour.
    EdgeId,
    Edge,
    "edge"
);
define_id!(
    /// A notch: an alignment mark on an edge.
    NotchId,
    Notch,
    "notch"
);
define_id!(
    /// An internal line drawn inside a panel (fold, placement, dart leg…).
    InternalLineId,
    InternalLine,
    "line"
);
define_id!(
    /// A seam: two edges sewn together.
    SeamId,
    Seam,
    "seam"
);
define_id!(
    /// A pin holding the cloth in place in 3D.
    PinId,
    Pin,
    "pin"
);
define_id!(
    /// A constraint or a dimension on points and edges.
    ConstraintId,
    Constraint,
    "constraint"
);
define_id!(
    /// A construction line on the board: drawn to draft against, never cut
    /// or sewn.
    GuideId,
    Guide,
    "guide"
);
define_id!(
    /// A button and its buttonhole: a closure.
    ButtonId,
    Button,
    "button"
);
define_id!(
    /// A zipper joining two stretches of outline.
    ZipperId,
    Zipper,
    "zipper"
);
define_id!(
    /// A construction detail on a piece: an elastic (spec 31).
    DetailId,
    Detail,
    "detail"
);

/// Any entity ID, for messages and command outputs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnyId {
    Garment(GarmentId),
    Panel(PanelId),
    Point(PointId),
    Edge(EdgeId),
    Notch(NotchId),
    InternalLine(InternalLineId),
    Seam(SeamId),
    Pin(PinId),
    Constraint(ConstraintId),
    Guide(GuideId),
    Button(ButtonId),
    Zipper(ZipperId),
    Detail(DetailId),
}

impl fmt::Display for AnyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyId::Garment(id) => id.fmt(f),
            AnyId::Panel(id) => id.fmt(f),
            AnyId::Point(id) => id.fmt(f),
            AnyId::Edge(id) => id.fmt(f),
            AnyId::Notch(id) => id.fmt(f),
            AnyId::InternalLine(id) => id.fmt(f),
            AnyId::Seam(id) => id.fmt(f),
            AnyId::Pin(id) => id.fmt(f),
            AnyId::Constraint(id) => id.fmt(f),
            AnyId::Guide(id) => id.fmt(f),
            AnyId::Button(id) => id.fmt(f),
            AnyId::Zipper(id) => id.fmt(f),
            AnyId::Detail(id) => id.fmt(f),
        }
    }
}

impl fmt::Debug for AnyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl FromStr for AnyId {
    type Err = ParseIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let prefix = s.split_once(':').map(|(p, _)| p).unwrap_or_default();
        match prefix {
            GarmentId::PREFIX => s.parse().map(AnyId::Garment),
            PanelId::PREFIX => s.parse().map(AnyId::Panel),
            PointId::PREFIX => s.parse().map(AnyId::Point),
            EdgeId::PREFIX => s.parse().map(AnyId::Edge),
            NotchId::PREFIX => s.parse().map(AnyId::Notch),
            InternalLineId::PREFIX => s.parse().map(AnyId::InternalLine),
            SeamId::PREFIX => s.parse().map(AnyId::Seam),
            PinId::PREFIX => s.parse().map(AnyId::Pin),
            ConstraintId::PREFIX => s.parse().map(AnyId::Constraint),
            GuideId::PREFIX => s.parse().map(AnyId::Guide),
            ButtonId::PREFIX => s.parse().map(AnyId::Button),
            ZipperId::PREFIX => s.parse().map(AnyId::Zipper),
            DetailId::PREFIX => s.parse().map(AnyId::Detail),
            _ => Err(ParseIdError {
                text: s.to_owned(),
                expected: "garment|panel|point|edge|notch|line|seam|pin|constraint|guide|button|zipper|detail",
            }),
        }
    }
}

impl Serialize for AnyId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for AnyId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl JsonSchema for AnyId {
    fn schema_name() -> Cow<'static, str> {
        "AnyId".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": "^(garment|panel|point|edge|notch|line|seam|pin|constraint|guide|button|zipper|detail):[1-9][0-9]*$",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_parse_round_trip() {
        let id = PanelId::from_raw(42);
        assert_eq!(id.to_string(), "panel:42");
        assert_eq!("panel:42".parse::<PanelId>(), Ok(id));
        assert_eq!("panel:42".parse::<AnyId>(), Ok(AnyId::Panel(id)));
    }

    #[test]
    fn wrong_kind_is_rejected() {
        assert!("point:42".parse::<PanelId>().is_err());
        assert!("panel:0".parse::<PanelId>().is_err());
        assert!("panel:".parse::<PanelId>().is_err());
        assert!("42".parse::<PanelId>().is_err());
        assert!("cloud:1".parse::<AnyId>().is_err());
    }

    #[test]
    fn serde_as_string() {
        let id = EdgeId::from_raw(7);
        assert_eq!(
            serde_json::to_string(&id).ok().as_deref(),
            Some("\"edge:7\"")
        );
        let back: EdgeId = serde_json::from_str("\"edge:7\"").unwrap();
        assert_eq!(back, id);
        assert!(serde_json::from_str::<EdgeId>("\"panel:7\"").is_err());
    }
}
