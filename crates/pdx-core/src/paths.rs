use crate::{Diagnostic, Result};
use std::collections::BTreeMap;

/// 内部pathの可搬な表記はslash区切りのみ。
pub fn relative_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path
            .chars()
            .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
    {
        return Err(Diagnostic::safety(format!(
            "invalid relative path: {path:?}"
        )));
    }
    for part in path.split('/') {
        let stem = part
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end_matches(' ')
            .to_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || matches!(
                stem.as_str(),
                "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$"
            )
            || ["COM", "LPT"].iter().any(|p| {
                stem.strip_prefix(p).is_some_and(|s| {
                    matches!(
                        s,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            })
            || part.contains('~')
        {
            return Err(Diagnostic::safety(format!("non-portable path: {path:?}")));
        }
    }
    Ok(())
}

pub fn identity(path: &str) -> String {
    path.to_uppercase()
}

pub fn overlaps(a: &str, b: &str) -> bool {
    let a = identity(a);
    let b = identity(b);
    a == b || a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
}

pub fn reject_aliases<'a>(paths: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let mut seen = BTreeMap::new();
    for path in paths {
        relative_path(path)?;
        let mut prefix = String::new();
        for part in path.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if let Some(old) = seen.insert(identity(&prefix), prefix.clone())
                && old != prefix
            {
                return Err(Diagnostic::safety(format!("path aliases: {old}, {prefix}")));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_paths() {
        for p in [
            "../a",
            "/a",
            "C:/a",
            "\\\\host\\a",
            "a\\b",
            "a:b",
            "nul.txt",
            "COM1",
            "NUL .txt",
            "CLOCK$",
            "LPT¹.x",
            "a.",
            "a ",
            "a//b",
            "a\n",
            "A~1",
        ] {
            assert!(relative_path(p).is_err(), "{p:?}");
        }
        relative_path("本文/第一章.md").unwrap();
        assert!(reject_aliases(["Content/a", "content/b"]).is_err());
    }
}
