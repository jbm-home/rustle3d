//! Scripts: a JSON array of steps, run in order.
//!
//! A step can refer to the result of an earlier step with a string such as
//! `"$3.created[0]"`: the result of step 3 (0-based), then the path
//! `.created[0]`. The whole string must be the reference (no interpolation
//! inside longer text); the value found replaces it, whatever its JSON type.
//! A string that really starts with `$` is written `$$…`.
//!
//! References make scripts independent of ID allocation: a script never has
//! to guess that a panel will be `panel:12`.

use r3d_core::{Localizable, Message};
use serde_json::Value;

use crate::error::CommandError;
use crate::step::Step;

/// A parsed script, not yet run.
#[derive(Clone, Debug)]
pub struct Script {
    steps: Vec<Value>,
}

/// A step that failed, with its index.
#[derive(Debug)]
pub struct ScriptError<E> {
    pub index: usize,
    pub error: E,
}

impl<E: Localizable> Localizable for ScriptError<E> {
    /// The step's own message, with its index added as the `step` argument.
    fn message(&self) -> Message {
        let mut m = self.error.message();
        m.args
            .entry("step")
            .or_insert_with(|| self.index.to_string());
        m
    }
}

fn bad_reference(reference: &str, reason: &'static str) -> CommandError {
    CommandError::BadReference {
        reference: reference.to_owned(),
        reason,
    }
}

/// Follow `path` (e.g. `.created[0].id`) into `value`.
fn follow<'a>(value: &'a Value, path: &str, reference: &str) -> Result<&'a Value, CommandError> {
    let mut current = value;
    let mut rest = path;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let end = after.find(['.', '[']).unwrap_or(after.len());
            let key = &after[..end];
            if key.is_empty() {
                return Err(bad_reference(reference, "empty field name"));
            }
            current = current
                .get(key)
                .ok_or_else(|| bad_reference(reference, "no such field"))?;
            rest = &after[end..];
        } else if let Some(after) = rest.strip_prefix('[') {
            let end = after
                .find(']')
                .ok_or_else(|| bad_reference(reference, "missing `]`"))?;
            let index: usize = after[..end]
                .parse()
                .map_err(|_| bad_reference(reference, "index is not a number"))?;
            current = current
                .get(index)
                .ok_or_else(|| bad_reference(reference, "index out of range"))?;
            rest = &after[end + 1..];
        } else {
            return Err(bad_reference(reference, "expected `.field` or `[index]`"));
        }
    }
    Ok(current)
}

fn resolve_string(s: &str, results: &[Value]) -> Result<Value, CommandError> {
    let Some(reference) = s.strip_prefix('$') else {
        return Ok(Value::from(s));
    };
    if reference.starts_with('$') {
        return Ok(Value::from(reference)); // `$$x` is the literal `$x`
    }
    let digits = reference.chars().take_while(char::is_ascii_digit).count();
    let index: usize = reference[..digits]
        .parse()
        .map_err(|_| bad_reference(s, "expected a step number after `$`"))?;
    let result = results
        .get(index)
        .ok_or_else(|| bad_reference(s, "refers to a step that has not run yet"))?;
    follow(result, &reference[digits..], s).cloned()
}

/// Replace every reference in `value` by what it points to.
pub fn resolve(value: &Value, results: &[Value]) -> Result<Value, CommandError> {
    Ok(match value {
        Value::String(s) => resolve_string(s, results)?,
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| resolve(v, results))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| Ok((k.clone(), resolve(v, results)?)))
                .collect::<Result<_, CommandError>>()?,
        ),
        other => other.clone(),
    })
}

impl Script {
    /// Parse a JSON array of steps. Steps are validated when they run,
    /// because references are only known then.
    pub fn parse(text: &str) -> Result<Self, CommandError> {
        let malformed = |detail: String| CommandError::Malformed {
            name: "script".into(),
            detail,
        };
        match serde_json::from_str(text).map_err(|e| malformed(e.to_string()))? {
            Value::Array(steps) => Ok(Self { steps }),
            _ => Err(malformed("expected a JSON array of steps".into())),
        }
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Run every step with `exec`, stopping at the first failure. Returns the
    /// result of each step.
    pub fn run<E: From<CommandError>>(
        &self,
        mut exec: impl FnMut(&Step) -> Result<Value, E>,
    ) -> Result<Vec<Value>, ScriptError<E>> {
        let mut results = Vec::with_capacity(self.steps.len());
        for (index, raw) in self.steps.iter().enumerate() {
            let fail = |error: E| ScriptError { index, error };
            let step = resolve(raw, &results)
                .and_then(Step::from_value)
                .map_err(|e| fail(E::from(e)))?;
            results.push(exec(&step).map_err(fail)?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_paths_and_escapes() {
        let results = vec![
            json!({"created": ["panel:2", "point:3"]}),
            json!([{"id": "edge:9"}]),
        ];
        let v = json!({
            "panel": "$0.created[0]",
            "edge": "$1[0].id",
            "all": "$0.created",
            "price": "$$5",
            "plain": "Front",
        });
        let r = resolve(&v, &results).unwrap();
        assert_eq!(r["panel"], "panel:2");
        assert_eq!(r["edge"], "edge:9");
        assert_eq!(r["all"], json!(["panel:2", "point:3"]));
        assert_eq!(r["price"], "$5");
        assert_eq!(r["plain"], "Front");
    }

    #[test]
    fn bad_references_are_explained() {
        let results = vec![json!({"created": ["panel:2"]})];
        for (s, reason) in [
            ("$1.created[0]", "refers to a step that has not run yet"),
            ("$0.made[0]", "no such field"),
            ("$0.created[3]", "index out of range"),
            ("$0.created[x]", "index is not a number"),
            ("$x", "expected a step number after `$`"),
            ("$0created", "expected `.field` or `[index]`"),
        ] {
            match resolve(&json!(s), &results) {
                Err(CommandError::BadReference { reason: r, .. }) => assert_eq!(r, reason, "{s}"),
                other => panic!("{s}: {other:?}"),
            }
        }
    }
}
