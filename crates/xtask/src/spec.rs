//! README split: each file in `docs/spec/` is a verbatim copy of one or more README sections.
//! A copy starts with a marker line naming its sources, and `spec-check` fails when any copy differs.

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

/// (file, README headings it copies). A heading carries its level: `##` or `###`.
const SPECS: &[(&str, &[&str])] = &[
    ("brain", &["## Company brain"]),
    ("sync", &["## Sync engine"]),
    ("harness", &["## Core runtime (harness)"]),
    (
        "model-access-and-relay",
        &["### Model access", "### Model relay"],
    ),
    ("plugins", &["## Plugins and connections"]),
    ("security", &["## Security model"]),
    ("data-model", &["## Data model"]),
    ("api", &["## API"]),
    ("desktop-and-node", &["## Desktop app and local node"]),
    ("gateway", &["## Gateway and request path"]),
    ("ingest", &["## Ingest and event adapter"]),
    ("ambient", &["## Ambient agents"]),
    ("operations", &["## Operations"]),
    ("performance", &["## Performance, latency and cost"]),
    ("evals", &["## Evals and quality"]),
];

fn marker(headings: &[&str]) -> String {
    format!(
        "<!-- Copy of README sections: {}. Change the README and this file together; `cargo xtask spec-check` enforces it. -->\n\n",
        headings
            .iter()
            .map(|h| format!("\"{h}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The text of one section: its heading line up to the next heading of the same or a higher level.
/// Lines inside code fences never count as headings.
fn section(readme: &str, heading: &str) -> Result<String> {
    let level = heading.chars().take_while(|c| *c == '#').count();
    let (mut out, mut inside, mut fence) = (Vec::new(), false, false);
    for line in readme.lines() {
        if line.trim_start().starts_with("```") {
            fence = !fence;
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        let is_heading = !fence && hashes > 0 && line.chars().nth(hashes) == Some(' ');
        if is_heading && inside && hashes <= level {
            break;
        }
        if !inside && !fence && line == heading {
            inside = true;
        }
        if inside {
            out.push(line);
        }
    }
    if out.is_empty() {
        bail!("README has no section {heading:?}");
    }
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    Ok(out.join("\n"))
}

fn expected(readme: &str, headings: &[&str]) -> Result<String> {
    let parts: Result<Vec<String>> = headings.iter().map(|h| section(readme, h)).collect();
    Ok(format!("{}{}\n", marker(headings), parts?.join("\n\n")))
}

pub fn split(root: &Path) -> Result<()> {
    let readme = fs::read_to_string(root.join("README.md")).context("README.md")?;
    let dir = root.join("docs/spec");
    fs::create_dir_all(&dir)?;
    for (file, headings) in SPECS {
        fs::write(dir.join(format!("{file}.md")), expected(&readme, headings)?)?;
    }
    println!("wrote {} spec files", SPECS.len());
    Ok(())
}

pub fn check(root: &Path) -> Result<()> {
    let readme = fs::read_to_string(root.join("README.md")).context("README.md")?;
    let mut stale = Vec::new();
    for (file, headings) in SPECS {
        let path = root.join(format!("docs/spec/{file}.md"));
        let have = fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if have != expected(&readme, headings)? {
            stale.push(*file);
        }
    }
    if !stale.is_empty() {
        bail!(
            "docs/spec is out of date for: {}. Run `cargo xtask spec-split`.",
            stale.join(", ")
        );
    }
    println!("docs/spec matches the README ({} files)", SPECS.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str =
        "# T\n\n## A\n\nbody a\n\n### A1\n\nsub\n\n```\n## not a heading\n```\n\n## B\n\nbody b\n";

    #[test]
    fn a_section_includes_its_subsections_and_stops_at_the_next_peer() {
        let a = section(DOC, "## A").unwrap();
        assert!(a.contains("body a") && a.contains("### A1") && a.contains("## not a heading"));
        assert!(!a.contains("body b"));
    }

    #[test]
    fn a_subsection_stops_at_the_next_heading_of_any_higher_level() {
        let s = section(DOC, "### A1").unwrap();
        assert!(s.contains("sub") && !s.contains("body b"));
    }

    #[test]
    fn missing_sections_are_errors() {
        assert!(section(DOC, "## Nope").is_err());
    }
}
