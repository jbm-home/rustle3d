//! `cargo xtask version [YY.M.D]`: set Rustle3D's version, the date of the
//! release (calendar versioning: `26.11.19` for 19 November 2026), in the
//! workspace's `Cargo.toml` and the app's `pubspec.yaml`. Without a date,
//! today's (UTC). Months and days have no leading zero: Cargo's versions
//! are semver, which forbids them (`26.1.5`, not `26.01.05`).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// The civil date (year, month, day) of a day count since 1970-01-01
/// (H. Hinnant, "chrono-Compatible Low-Level Date Algorithms", 2013).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Today's version.
fn today() -> Result<String, String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let (year, month, day) = civil(i64::try_from(secs / 86_400).map_err(|e| e.to_string())?);
    Ok(format!("{}.{month}.{day}", year % 100))
}

/// A `YY.M.D` version, checked.
fn parse(version: &str) -> Result<String, String> {
    let bad = || format!("{version}: expected YY.M.D, e.g. 26.11.19 (no leading zeros)");
    let parts: Vec<u32> = version
        .split('.')
        .map(|p| {
            if p.len() > 1 && p.starts_with('0') {
                return Err(bad());
            }
            p.parse().map_err(|_| bad())
        })
        .collect::<Result<_, _>>()?;
    match parts.as_slice() {
        [y, m, d] if *y < 100 && (1..=12).contains(m) && (1..=31).contains(d) => {
            Ok(version.to_owned())
        }
        _ => Err(bad()),
    }
}

/// Replace the first line starting with `key` in `path` by `line`.
fn set_line(path: &Path, key: &str, line: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let at = text
        .lines()
        .position(|l| l.starts_with(key))
        .ok_or_else(|| format!("{}: no `{key}` line", path.display()))?;
    let out: Vec<&str> = text
        .lines()
        .enumerate()
        .map(|(i, l)| if i == at { line } else { l })
        .collect();
    let mut out = out.join(if text.contains("\r\n") { "\r\n" } else { "\n" });
    if text.ends_with('\n') {
        out.push_str(if text.ends_with("\r\n") { "\r\n" } else { "\n" });
    }
    std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let version = match args {
        [] => today()?,
        [v] => parse(v)?,
        _ => return Err("usage: cargo xtask version [YY.M.D]".to_owned()),
    };
    // The workspace's version is the first `version = ` line of the root
    // manifest, under [workspace.package].
    set_line(
        &root.join("Cargo.toml"),
        "version = ",
        &format!("version = \"{version}\""),
    )?;
    set_line(
        &root.join("app/pubspec.yaml"),
        "version: ",
        &format!("version: {version}"),
    )?;
    let mut lock = std::process::Command::new(env!("CARGO"));
    lock.current_dir(root)
        .args(["update", "--workspace", "--quiet"]);
    let status = lock.status().map_err(|e| format!("cargo update: {e}"))?;
    if !status.success() {
        return Err(format!("cargo update --workspace failed ({status})"));
    }
    eprintln!("version {version}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_776), (2026, 11, 19));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(civil(-1), (1969, 12, 31));
    }

    #[test]
    fn versions() {
        assert_eq!(parse("26.11.19").as_deref(), Ok("26.11.19"));
        assert_eq!(parse("26.1.5").as_deref(), Ok("26.1.5"));
        for bad in [
            "26.01.05",
            "2026.11.19",
            "26.13.1",
            "26.11",
            "26.11.19.1",
            "a.b.c",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
}
