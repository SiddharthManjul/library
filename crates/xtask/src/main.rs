//! `cargo xtask <task>`: contract generation and the README spec split.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

mod spec;

fn root() -> PathBuf {
    // crates/xtask -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .components()
        .collect()
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    match args.first().map(String::as_str) {
        Some("contracts") => contracts(check),
        Some("spec-split") => spec::split(&root()),
        Some("spec-check") => spec::check(&root()),
        Some("schema-dump") => schema_dump(),
        _ => {
            bail!("usage: cargo xtask contracts [--check] | spec-split | spec-check | schema-dump")
        }
    }
}

fn contracts(check: bool) -> Result<()> {
    let files = [
        (
            "contracts/protocol.md",
            library_protocol::render_protocol_md(),
        ),
        (
            "apps/desktop/src/generated/app-methods.ts",
            library_protocol::render_app_methods_ts(),
        ),
    ];
    for (rel, want) in files {
        let path = root().join(rel);
        if check {
            let have = fs::read_to_string(&path)
                .unwrap_or_default()
                .replace("\r\n", "\n");
            if have != want {
                bail!("{rel} is out of date; run `cargo xtask contracts`");
            }
            println!("{rel} is current");
        } else {
            fs::create_dir_all(path.parent().context("parent")?)?;
            fs::write(&path, want)?;
            println!("wrote {rel}");
        }
    }
    Ok(())
}

/// Dumps the schema of a freshly migrated dev database, so agents can read it without a database.
fn schema_dump() -> Result<()> {
    let url = std::env::var("DATABASE_OWNER_URL")
        .unwrap_or_else(|_| "postgres://library_owner:owner_dev_pw@127.0.0.1:5433/library".into());
    let out = Command::new("pg_dump")
        .args([
            "--schema-only",
            "--no-owner",
            "--no-privileges",
            "--no-comments",
            "--exclude-table=_sqlx_migrations",
            &url,
        ])
        .output()
        .context("pg_dump is not installed or not on PATH")?;
    if !out.status.success() {
        bail!("pg_dump failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    // Drop dump noise that changes between runs and servers.
    let text = String::from_utf8_lossy(&out.stdout);
    let body: String = text
        .lines()
        .filter(|l| {
            !l.starts_with("--")
                && !l.starts_with("SET ")
                && !l.starts_with("SELECT pg_catalog.set_config")
                && !l.starts_with("\\restrict")
                && !l.starts_with("\\unrestrict")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let header = "-- Schema of the Library IDE database, dumped from the migrations by `cargo xtask schema-dump`.\n-- Read-only reference for agents. The migrations in migrations/ are the source.\n";
    let squeezed = body.split("\n\n\n").collect::<Vec<_>>().join("\n\n");
    fs::write(
        root().join("contracts/schema.sql"),
        format!("{header}\n{}\n", squeezed.trim()),
    )?;
    println!("wrote contracts/schema.sql");
    Ok(())
}
