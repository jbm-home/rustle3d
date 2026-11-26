//! PLY (Stanford polygon file format), ASCII or binary (little or big
//! endian): the `x`, `y`, `z` of the `vertex` element and the
//! `vertex_indices` (or `vertex_index`) list of the `face` element,
//! polygons split into fans. Other elements and properties (colours,
//! normals, confidence of a scan) are read past and ignored.

use super::{Model, ModelError, ModelFormat, bad, too_large};

/// A scalar type of the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl Scalar {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "char" | "int8" => Self::I8,
            "uchar" | "uint8" => Self::U8,
            "short" | "int16" => Self::I16,
            "ushort" | "uint16" => Self::U16,
            "int" | "int32" => Self::I32,
            "uint" | "uint32" => Self::U32,
            "float" | "float32" => Self::F32,
            "double" | "float64" => Self::F64,
            _ => return None,
        })
    }

    fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Property {
    One(Scalar, String),
    /// Count type, item type, name.
    List(Scalar, Scalar, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Element {
    name: String,
    count: usize,
    properties: Vec<Property>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Ascii,
    Little,
    Big,
}

/// Where the values of the body come from.
enum Body<'a> {
    Ascii(std::str::SplitAsciiWhitespace<'a>),
    Binary {
        bytes: &'a [u8],
        at: usize,
        big: bool,
    },
}

impl Body<'_> {
    fn next(&mut self, ty: Scalar) -> Option<f64> {
        match self {
            Self::Ascii(words) => words.next()?.parse().ok(),
            Self::Binary { bytes, at, big } => {
                let end = at.checked_add(ty.size())?;
                let b = bytes.get(*at..end)?;
                *at = end;
                macro_rules! num {
                    ($t:ty) => {{
                        let a = b.try_into().ok()?;
                        f64::from(if *big {
                            <$t>::from_be_bytes(a)
                        } else {
                            <$t>::from_le_bytes(a)
                        })
                    }};
                }
                Some(match ty {
                    Scalar::I8 => num!(i8),
                    Scalar::U8 => num!(u8),
                    Scalar::I16 => num!(i16),
                    Scalar::U16 => num!(u16),
                    Scalar::I32 => num!(i32),
                    Scalar::U32 => num!(u32),
                    Scalar::F32 => num!(f32),
                    Scalar::F64 => num!(f64),
                })
            }
        }
    }
}

fn ply(detail: impl ToString) -> ModelError {
    bad(ModelFormat::Ply, detail)
}

/// The header's elements, its encoding and where the body starts.
fn header(bytes: &[u8]) -> Result<(Vec<Element>, Encoding, usize), ModelError> {
    const END: &[u8] = b"end_header";
    let end = bytes
        .windows(END.len())
        .position(|w| w == END)
        .ok_or_else(|| ply("the header has no end_header"))?;
    // The body starts after the end of that line (`\n` or `\r\n`).
    let body = bytes
        .get(end..)
        .and_then(|rest| rest.iter().position(|b| *b == b'\n'))
        .map_or(bytes.len(), |nl| end + nl + 1);
    let text = String::from_utf8_lossy(bytes.get(..end).unwrap_or_default());
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("ply") {
        return Err(ply("not a PLY file"));
    }
    let mut encoding = None;
    let mut elements: Vec<Element> = Vec::new();
    for line in lines {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words[..] {
            ["format", format, _] => {
                encoding = Some(match format {
                    "ascii" => Encoding::Ascii,
                    "binary_little_endian" => Encoding::Little,
                    "binary_big_endian" => Encoding::Big,
                    _ => return Err(ply(format!("unknown format {format}"))),
                });
            }
            ["element", name, count] => elements.push(Element {
                name: name.into(),
                count: count
                    .parse()
                    .map_err(|_| ply(format!("bad count for element {name}")))?,
                properties: Vec::new(),
            }),
            ["property", "list", count, item, name] => {
                let (Some(c), Some(i)) = (Scalar::parse(count), Scalar::parse(item)) else {
                    return Err(ply(format!("unknown type in property {name}")));
                };
                elements
                    .last_mut()
                    .ok_or_else(|| ply("a property before any element"))?
                    .properties
                    .push(Property::List(c, i, name.into()));
            }
            ["property", ty, name] => {
                let ty = Scalar::parse(ty).ok_or_else(|| ply(format!("unknown type {ty}")))?;
                elements
                    .last_mut()
                    .ok_or_else(|| ply("a property before any element"))?
                    .properties
                    .push(Property::One(ty, name.into()));
            }
            _ => {} // comment, obj_info, blank lines
        }
    }
    let encoding = encoding.ok_or_else(|| ply("the header has no format line"))?;
    Ok((elements, encoding, body))
}

/// Read a PLY file.
pub(super) fn read(bytes: &[u8]) -> Result<Model, ModelError> {
    let (elements, encoding, start) = header(bytes)?;
    let rest = bytes.get(start..).unwrap_or_default();
    let text;
    let mut body = match encoding {
        Encoding::Ascii => {
            text = String::from_utf8_lossy(rest);
            Body::Ascii(text.split_ascii_whitespace())
        }
        Encoding::Little | Encoding::Big => Body::Binary {
            bytes: rest,
            at: 0,
            big: encoding == Encoding::Big,
        },
    };
    let short = |name: &str| ply(format!("the file ends inside its {name} element"));
    let mut model = Model::default();
    for element in &elements {
        let is_vertex = element.name == "vertex";
        let is_face = element.name == "face";
        let axis = |p: &Property| match p {
            Property::One(_, n) => ["x", "y", "z"].iter().position(|a| a == n),
            Property::List(..) => None,
        };
        let indices = element.properties.iter().position(|p| {
            matches!(p, Property::List(_, _, n) if n == "vertex_indices" || n == "vertex_index")
        });
        if is_vertex && (0..3).any(|a| !element.properties.iter().any(|p| axis(p) == Some(a))) {
            return Err(ply("the vertices have no x, y and z"));
        }
        if is_face && indices.is_none() {
            return Err(ply("the faces have no vertex_indices"));
        }
        // An element without properties takes no room: nothing to read.
        let count = if element.properties.is_empty() {
            0
        } else {
            element.count
        };
        for _ in 0..count {
            let mut xyz = [0.0f32; 3];
            for (k, property) in element.properties.iter().enumerate() {
                match property {
                    Property::One(ty, _) => {
                        let v = body.next(*ty).ok_or_else(|| short(&element.name))?;
                        if let (true, Some(a)) = (is_vertex, axis(property)) {
                            let v = v as f32;
                            if !v.is_finite() {
                                return Err(ply("a vertex is not a number"));
                            }
                            if let Some(slot) = xyz.get_mut(a) {
                                *slot = v;
                            }
                        }
                    }
                    Property::List(count, item, _) => {
                        let n = body.next(*count).ok_or_else(|| short(&element.name))?;
                        let mut polygon = Vec::new();
                        for _ in 0..(n.max(0.0) as usize) {
                            let i = body.next(*item).ok_or_else(|| short(&element.name))?;
                            if is_face && indices == Some(k) {
                                polygon.push(i);
                            }
                        }
                        if let Some((first, rest)) = polygon.split_first() {
                            for pair in rest.windows(2) {
                                if let [b, c] = pair {
                                    model.triangles.push([
                                        vertex(*first)?,
                                        vertex(*b)?,
                                        vertex(*c)?,
                                    ]);
                                }
                            }
                            too_large(model.triangles.len())?;
                        }
                    }
                }
            }
            if is_vertex {
                model.positions.push(xyz);
            }
        }
    }
    let count = model.positions.len();
    if let Some(i) = model
        .triangles
        .iter()
        .flatten()
        .find(|i| **i as usize >= count)
    {
        return Err(ply(format!("face index {i} out of range")));
    }
    Ok(model)
}

fn vertex(index: f64) -> Result<u32, ModelError> {
    if index >= 0.0 && index <= f64::from(u32::MAX) && index.fract() == 0.0 {
        Ok(index as u32)
    } else {
        Err(ply(format!("bad face index {index}")))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::super::{ModelError, ModelFormat, read};

    const ASCII: &str = "ply
format ascii 1.0
comment a square
element vertex 4
property float x
property float y
property float z
property uchar red
element face 1
property list uchar int vertex_indices
element edge 1
property int vertex1
property int vertex2
end_header
0 0 0 255
1 0 0 255
1 1 0 255
0 1 0 255
4 0 1 2 3
0 1
";

    fn binary(big: bool) -> Vec<u8> {
        let mut b = format!(
            "ply\r\nformat binary_{}_endian 1.0\r\nelement vertex 3\r\nproperty double z\r\nproperty float nx\r\nproperty double x\r\nproperty double y\r\nelement face 1\r\nproperty uchar flags\r\nproperty list uchar uint vertex_index\r\nend_header\r\n",
            if big { "big" } else { "little" }
        )
        .into_bytes();
        let f64b = |v: f64| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let f32b = |v: f32| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let u32b = |v: u32| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        for [x, y, z] in [[0.0, 0.0, 3.0], [1.0, 0.0, 3.0], [0.0, 1.0, 3.0]] {
            b.extend(f64b(z));
            b.extend(f32b(0.5));
            b.extend(f64b(x));
            b.extend(f64b(y));
        }
        b.push(7);
        b.push(3);
        for i in [0, 1, 2] {
            b.extend(u32b(i));
        }
        b
    }

    #[test]
    fn ascii_polygons_and_other_elements() {
        let m = read(ASCII.as_bytes(), ModelFormat::Ply).unwrap();
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.positions[2], [1.0, 1.0, 0.0]);
        assert_eq!(m.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn binary_both_endians_any_property_order() {
        for big in [false, true] {
            let m = read(&binary(big), ModelFormat::Ply).unwrap();
            assert_eq!(
                m.positions,
                vec![[0.0, 0.0, 3.0], [1.0, 0.0, 3.0], [0.0, 1.0, 3.0]]
            );
            assert_eq!(m.triangles, vec![[0, 1, 2]]);
        }
    }

    #[test]
    fn broken_files_are_refused() {
        let malformed =
            |b: &[u8]| matches!(read(b, ModelFormat::Ply), Err(ModelError::Malformed { .. }));
        assert!(malformed(b"not a ply"));
        assert!(malformed(b"ply\nelement vertex 1\nend_header\n"));
        assert!(malformed(
            ASCII.replace("4 0 1 2 3", "4 0 1 2 9").as_bytes()
        ));
        assert!(malformed(
            ASCII
                .replace("0 1\n", "")
                .replace("4 0 1 2 3\n", "4 0 1")
                .as_bytes()
        ));
        assert!(malformed(
            ASCII.replace("property float z\n", "").as_bytes()
        ));
        let mut short = binary(false);
        short.truncate(short.len() - 3);
        assert!(malformed(&short));
        // A point cloud: no triangles.
        let cloud = "ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n0 0 0\n";
        assert_eq!(
            read(cloud.as_bytes(), ModelFormat::Ply),
            Err(ModelError::Empty)
        );
    }
}
