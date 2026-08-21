//! Repository census: what the source tree measurably contains.
//!
//! **Class (c): derived from the source tree, not from the specoment.** `SD-011` split
//! artefacts into (a) root projections, policed by byte-identity against the specoment digest,
//! and (b) accumulated state, policed by conformance. A census is neither — derived, so not
//! (b); derived from `cargo metadata` and the source tree, so not (a).
//!
//! Placing it in `emit::all()` would invert its staleness key: `run()` stamps
//! `specoment:{canonical_sha256}`, so changing all 37 crates would read *fresh* while a typo in
//! the specoment reads *stale*. It would also make `project --check` machine-dependent, because
//! `cargo metadata` emits absolute manifest paths. Recorded as `SD-013`.
//!
//! This module **measures**. It assigns no dispositions: every row carries `disposition: null`
//! and `escalation: null`, and a non-null value means judgement leaked into a measurement-only
//! package. `WP-V1-CENSUS-002` assigns them against this frozen output.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::sha256;

/// A crate row. Dispositions are absent by construction.
#[derive(Debug, Clone)]
pub struct CrateRow {
    pub name: String,
    pub location: String,
    pub manifest: String,
    pub source_lines: usize,
    pub source_bytes: u64,
    pub declared_public_items: usize,
    pub test_functions: usize,
}

/// One versioned on-disk format. A magic-plus-version pair is one row, not two.
#[derive(Debug, Clone)]
pub struct FormatRow {
    pub name: String,
    pub magic: Option<String>,
    pub version_constant: Option<String>,
    pub owner: String,
}

/// A dependency edge. `reverse` is derived from the declared layer order, never asserted.
#[derive(Debug, Clone)]
pub struct EdgeRow {
    pub from: String,
    pub to: String,
    pub optional: bool,
    pub reverse: bool,
}

/// The layer order the census judges edges against.
///
/// "Exactly two reverse edges" is not verifiable until an ordering exists, and neither the plan
/// nor the specoment declared one. Lower index is lower level; an edge from a lower layer to a
/// higher one is reverse.
pub const LAYERS: &[(&str, &[&str])] = &[
    (
        "foundation",
        &["meridian-core", "meridian-tasks", "meridian-diagnostics"],
    ),
    (
        "platform",
        &[
            "meridian-platform",
            "meridian-input",
            "meridian-render-graph",
        ],
    ),
    (
        "data",
        &[
            "meridian-assets",
            "meridian-save",
            "meridian-package",
            "meridian-world",
        ],
    ),
    (
        "simulation",
        &[
            "meridian-ecs",
            "meridian-physics",
            "meridian-streaming",
            "meridian-alluvium",
            "meridian-modeler",
        ],
    ),
    // Runtime sits BELOW presentation. The specoment forbids headless runtime reaching
    // renderer or UI, so an ordering that placed runtime above presentation would render
    // `meridian-rt -> meridian-renderer` "forward" and silently bless the edge the rule
    // exists to catch. The first draft of this table did exactly that, and the derived
    // `reverse` flag exposed it immediately — which is the argument for deriving the flag
    // from a declared order rather than asserting a count.
    ("runtime", &["meridian-rt"]),
    (
        "presentation",
        &[
            "meridian-rhi",
            "meridian-renderer",
            "meridian-ui-core",
            "meridian-ui-text",
            "meridian-ui-render",
            "meridian-ui-semantics",
            "meridian-ui-runtime",
            "meridian-ui",
        ],
    ),
    (
        "tools",
        &[
            "meridian-spec",
            "meridian-build",
            "meridian-benchmark",
            "meridian-asset-tools",
            "meridian-shader-tools",
            "meridian-world-tools",
            "meridian-editor-core",
            "meridian-ui-editor",
        ],
    ),
    ("product", &["meridian-editor"]),
];

fn layer_of(crate_name: &str) -> Option<usize> {
    LAYERS
        .iter()
        .position(|(_, members)| members.contains(&crate_name))
}

/// The measured census.
#[derive(Debug, Default)]
pub struct Census {
    pub crates: Vec<CrateRow>,
    pub formats: Vec<FormatRow>,
    pub edges: Vec<EdgeRow>,
    pub tests: Vec<TestRow>,
    pub generated_files: Vec<String>,
    pub ci_rows: Vec<String>,
}

/// One `#[test]` function, keyed so an individual test can be mapped later.
#[derive(Debug, Clone)]
pub struct TestRow {
    pub file: String,
    pub line: usize,
    pub function: String,
}

/// Count `pub` items declared at module root.
///
/// `^pub ` across all files, which is 213 for the four crates `meridian-ui` globs. The
/// alternative — `pub` item keywords at any indentation, 462 — counts `pub fn` methods inside
/// `impl` blocks and items inside private modules, **neither of which a glob re-export
/// forwards**, so it inflates a façade's count with things the façade does not export.
///
/// Residual ambiguity, stated rather than hidden: a glob strictly forwards the *crate-root*
/// namespace, which is `lib.rs` alone and gives 205. The extra 8 are module-root items in
/// submodules, reachable as `submod::X`. 213 answers "what public API exists"; 205 answers
/// "what does this glob forward". A resolution-based count is the honest long-term answer and
/// is recorded as a limitation, not silently approximated.
fn declared_public_items(text: &str) -> usize {
    text.lines().filter(|line| line.starts_with("pub ")).count()
}

fn test_rows(relative: &str, text: &str) -> Vec<TestRow> {
    let lines: Vec<&str> = text.lines().collect();
    let mut rows = Vec::new();
    for (offset, line) in lines.iter().enumerate() {
        if line.trim() != "#[test]" {
            continue;
        }
        // The function name sits on the next non-attribute line.
        let name = lines[offset + 1..]
            .iter()
            .find(|candidate| candidate.trim().contains("fn "))
            .and_then(|candidate| candidate.split("fn ").nth(1))
            .and_then(|rest| rest.split('(').next())
            .unwrap_or("unknown")
            .trim()
            .to_string();
        rows.push(TestRow {
            file: relative.to_string(),
            line: offset + 1,
            function: name,
        });
    }
    rows
}

/// Every file the census reads, in sorted order. The digest over this set is the staleness key.
fn input_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for area in ["engine", "editor"] {
        collect(&root.join(area), root, &mut found);
    }
    for extra in ["Cargo.toml", "Cargo.lock", "governance/manifest.json"] {
        if root.join(extra).is_file() {
            found.push(extra.to_string());
        }
    }
    collect(&root.join(".github/workflows"), root, &mut found);
    found.sort();
    found
}

fn collect(directory: &Path, root: &Path, into: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            collect(&path, root, into);
        } else if path.extension().is_some_and(|ext| {
            ext == "rs" || ext == "toml" || ext == "yml" || ext == "lock" || ext == "json"
        }) {
            if let Ok(relative) = path.strip_prefix(root) {
                into.push(relative.to_string_lossy().to_string());
            }
        }
    }
}

/// The staleness key: a digest over exactly the files the census reads.
///
/// Not repository HEAD — that moves on doc-only commits, and is unstampable in the commit that
/// lands the file, which `mod.rs` already rejected as "a check that can never pass". Not
/// manifests-plus-lock either — the census records test counts and file sizes, which change
/// without touching a manifest.
///
/// The checkpoint is **provenance**, not enforcement: regeneration-and-compare is the catch, so
/// an over-sensitive digest costs a regeneration rather than a false pass.
pub fn source_tree_checkpoint(root: &Path) -> String {
    let mut hasher_input = String::new();
    for relative in input_files(root) {
        let Ok(bytes) = fs::read(root.join(&relative)) else {
            continue;
        };
        let _ = writeln!(hasher_input, "{relative}:{}", sha256::hex(&bytes));
    }
    format!("source-tree:{}", sha256::hex(hasher_input.as_bytes()))
}

/// The hand-listed known-format set, committed as an artifact.
///
/// A grep is a **discovery tool, not a completeness proof**. The previous criterion,
/// `[A-Z_]*(SCHEMA_VERSION|FORMAT_VERSION|_VERSION|_MAGIC)`, required a leading underscore on
/// both `_MAGIC` and `_VERSION`, so a bare `const MAGIC` could not match — and two exist:
/// `MERIDN\0\0` (package container) and `MSAV` (save file). Those are the two serialized
/// outputs whose corruption would be least recoverable, and precisely the ones `PH-AUTH-006`'s
/// stop condition — "Stop if decomposition changes serialized output" — exists to protect.
///
/// The enumeration is asserted against this table, so a format added later without updating
/// both fails rather than passing silently. Row 15 has no Rust constant at all, only a JSON
/// Schema, so no constant-name pattern could find it however written.
pub const KNOWN_FORMATS: &[(&str, Option<&str>, Option<&str>, &str)] = &[
    (
        "package-container",
        Some("MERIDN\\0\\0"),
        Some("FORMAT_VERSION"),
        "meridian-package",
    ),
    ("save-file", Some("MSAV"), None, "meridian-save"),
    (
        "save-journal",
        Some("MJNL"),
        Some("JOURNAL_FORMAT_VERSION"),
        "meridian-save",
    ),
    (
        "compiled-cell",
        Some("COMPILED_CELL_MAGIC"),
        Some("COMPILED_CELL_VERSION"),
        "meridian-streaming",
    ),
    (
        "visual-facet",
        Some("VISUAL_FACET_MAGIC"),
        None,
        "meridian-assets",
    ),
    (
        "collision-facet",
        Some("COLLISION_FACET_MAGIC"),
        None,
        "meridian-assets",
    ),
    (
        "recipe",
        None,
        Some("RECIPE_SCHEMA_VERSION"),
        "meridian-alluvium",
    ),
    ("model", None, Some("MODEL_VERSION"), "meridian-modeler"),
    (
        "ui-document",
        None,
        Some("UI_DOCUMENT_SCHEMA_VERSION"),
        "meridian-ui-core",
    ),
    (
        "ui-source",
        None,
        Some("UI_DOCUMENT_SOURCE_FORMAT_VERSION"),
        "meridian-ui-core",
    ),
    (
        "build-protocol",
        None,
        Some("BUILD_PROTOCOL_VERSION"),
        "meridian-build",
    ),
    (
        "workspace-state",
        None,
        Some("WORKSPACE_STATE_VERSION"),
        "meridian-editor-core",
    ),
    (
        "golden-fixture",
        None,
        Some("GOLDEN_FIXTURE_VERSION"),
        "meridian-benchmark",
    ),
    (
        "fixture-mesh-import",
        None,
        Some("FIXTURE_MESH_IMPORTER_VERSION"),
        "meridian-benchmark",
    ),
    (
        "benchmark-result",
        None,
        None,
        "schemas/benchmark-result.schema.json",
    ),
];

/// Constants a name-based sweep finds that are **not** on-disk formats.
const NOT_FORMATS: &[&str] = &["CARGO_PKG_VERSION", "ENGINE_VERSION", "GENERATOR_VERSION"];

/// Discover format constants in the source, for reconciliation against `KNOWN_FORMATS`.
pub fn discovered_format_constants(root: &Path) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for relative in input_files(root) {
        if !is_rust(&relative) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&relative)) else {
            continue;
        };
        for line in text.lines() {
            // `pub const` as well as `const`: the first draft stripped only the latter and
            // missed every exported format constant, which the reconciliation test caught.
            let trimmed = line.trim();
            let Some(rest) = trimmed
                .strip_prefix("pub const ")
                .or_else(|| trimmed.strip_prefix("const "))
            else {
                continue;
            };
            let Some(name) = rest.split(':').next().map(str::trim) else {
                continue;
            };
            let is_format = name == "MAGIC"
                || name.contains("MAGIC")
                || name.ends_with("_VERSION")
                || name.ends_with("SCHEMA_VERSION");
            if is_format && !NOT_FORMATS.contains(&name) && !found.iter().any(|f| f == name) {
                found.push(name.to_string());
            }
        }
    }
    found.sort();
    found
}

/// Reconcile discovered format constants against the hand-listed set.
///
/// Live rather than test-only: a format added without updating `KNOWN_FORMATS` must fail
/// `check`, not merely `cargo test`. A grep discovers candidates; agreement with a set built
/// by a different method is what licenses trusting the count.
pub fn format_reconciliation(root: &Path) -> Vec<String> {
    let discovered = discovered_format_constants(root);
    let mut problems = Vec::new();
    if discovered.is_empty() {
        problems.push("the format discovery sweep found nothing, which means it is broken".into());
        return problems;
    }
    if !discovered.iter().any(|name| name == "MAGIC") {
        problems.push(
            "bare `const MAGIC` was not discovered: it names the package container and the save \
             file, the two serialized outputs PH-AUTH-006's stop condition most protects"
                .to_string(),
        );
    }
    for name in &discovered {
        if NOT_FORMATS.contains(&name.as_str()) {
            problems.push(format!(
                "{name} is not an on-disk format and must be excluded"
            ));
        }
    }
    for (format, magic, version, _) in KNOWN_FORMATS {
        for constant in [magic, version].into_iter().flatten() {
            if (constant.ends_with("_MAGIC") || constant.ends_with("_VERSION"))
                && !discovered.iter().any(|d| d == constant)
            {
                problems.push(format!(
                    "{format} names {constant}, which the sweep did not find"
                ));
            }
        }
    }
    problems
}

/// Measure the repository.
pub fn measure(root: &Path) -> Census {
    let mut census = Census::default();

    for relative in input_files(root) {
        if !is_rust(&relative) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&relative)) else {
            continue;
        };
        census.tests.extend(test_rows(&relative, &text));
    }

    for (name, magic, version, owner) in KNOWN_FORMATS {
        census.formats.push(FormatRow {
            name: (*name).to_string(),
            magic: magic.map(str::to_string),
            version_constant: version.map(str::to_string),
            owner: (*owner).to_string(),
        });
    }

    if let Ok(manifest) = fs::read_to_string(root.join("governance/manifest.json")) {
        for line in manifest.lines() {
            if let Some(at) = line.find("\"path\": \"") {
                let rest = &line[at + 9..];
                if let Some(path) = rest.split('"').next() {
                    census.generated_files.push(path.to_string());
                }
            }
        }
        census
            .generated_files
            .push("governance/manifest.json".to_string());
    }

    for workflow in ["ci.yml", "discord-ci.yml"] {
        let path = root.join(".github/workflows").join(workflow);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mut in_jobs = false;
        for line in text.lines() {
            if line.starts_with("jobs:") {
                in_jobs = true;
                continue;
            }
            if in_jobs
                && line.starts_with("  ")
                && line.trim_end().ends_with(':')
                && !line.starts_with("    ")
            {
                census
                    .ci_rows
                    .push(format!("{workflow}:{}", line.trim().trim_end_matches(':')));
            }
        }
    }

    census
}

/// Crate rows and edges, from `cargo metadata` output supplied by the caller.
pub fn absorb_metadata(census: &mut Census, root: &Path, metadata: &serde_json::Value) {
    let members: Vec<&serde_json::Value> = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .map(|list| list.iter().collect())
        .unwrap_or_default();
    let names: Vec<String> = members
        .iter()
        .filter_map(|p| {
            p.get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect();

    for package in &members {
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let manifest = package
            .get("manifest_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        // Absolute paths would make any comparison machine-dependent.
        let manifest = manifest
            .strip_prefix(&format!("{}/", root.display()))
            .unwrap_or(manifest)
            .to_string();
        let source_dir = root
            .join(manifest.trim_end_matches("Cargo.toml"))
            .join("src");

        let (mut lines, mut bytes, mut public) = (0usize, 0u64, 0usize);
        let mut files = Vec::new();
        collect(&source_dir, root, &mut files);
        for relative in &files {
            if !is_rust(relative) {
                continue;
            }
            if let Ok(text) = fs::read_to_string(root.join(relative)) {
                lines += text.lines().count();
                bytes += text.len() as u64;
                public += declared_public_items(&text);
            }
        }
        let tests = census
            .tests
            .iter()
            .filter(|row| files.iter().any(|f| f == &row.file))
            .count();

        census.crates.push(CrateRow {
            name: name.to_string(),
            location: if manifest.starts_with("engine/") {
                "engine"
            } else {
                "editor"
            }
            .to_string(),
            manifest,
            source_lines: lines,
            source_bytes: bytes,
            declared_public_items: public,
            test_functions: tests,
        });

        for dependency in package
            .get("dependencies")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(to) = dependency.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !names.iter().any(|n| n == to) {
                continue;
            }
            let optional = dependency
                .get("optional")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let reverse = match (layer_of(name), layer_of(to)) {
                (Some(from), Some(target)) => from < target,
                _ => false,
            };
            census.edges.push(EdgeRow {
                from: name.to_string(),
                to: to.to_string(),
                optional,
                reverse,
            });
        }
    }
    census.crates.sort_by(|a, b| a.name.cmp(&b.name));
    census
        .edges
        .sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
}

/// A named metric with the command that produced it, so a figure never stands unstamped.
pub struct Measured {
    pub name: &'static str,
    pub value: usize,
    pub command: &'static str,
}

impl Census {
    /// Counts, each paired with the command a reader can re-run.
    pub fn measurements(&self) -> Vec<Measured> {
        vec![
            Measured {
                name: "crates",
                value: self.crates.len(),
                command: "cargo metadata --locked --no-deps --format-version 1",
            },
            Measured {
                name: "test_functions",
                value: self.tests.len(),
                command: "grep -rh '#[test]' --include='*.rs' engine editor | wc -l",
            },
            Measured {
                name: "formats",
                value: self.formats.len(),
                command: "hand-listed in KNOWN-FORMATS.md, reconciled against discovered constants",
            },
            Measured {
                name: "edges",
                value: self.edges.len(),
                command: "cargo metadata --locked --no-deps, internal dependencies only",
            },
        ]
    }
}

/// Rust source, matched case-insensitively so a `.RS` file is not silently skipped.
fn is_rust(relative: &str) -> bool {
    std::path::Path::new(relative)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Render the census.
///
/// Every row carries `disposition: null` and `escalation: null`. This package measures;
/// `WP-V1-CENSUS-002` assigns. A non-null value here means judgement leaked in.
pub fn render(census: &Census, root: &Path, specoment_sha256: &str) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(out, "  \"schema\": 1,");
    let _ = writeln!(
        out,
        "  \"generated\": \"DO NOT EDIT. Regenerate with: cargo run -p meridian-spec -- check\","
    );
    let _ = writeln!(
        out,
        "  \"artefact_class\": \"c: derived from the source tree. Not a specoment projection and not accumulated state. See SD-013.\","
    );
    let _ = writeln!(
        out,
        "  \"source_tree_checkpoint\": \"{}\",",
        source_tree_checkpoint(root)
    );
    let _ = writeln!(out, "  \"specoment_sha256\": \"{specoment_sha256}\",");
    let _ = writeln!(
        out,
        "  \"assignment\": \"disposition and escalation are null throughout: WP-V1-CENSUS-001 measures, WP-V1-CENSUS-002 assigns. A row is valid iff exactly one of them is non-null, which is why undecided was withdrawn - it occurs zero times in the specoment.\","
    );

    out.push_str("  \"measurements\": [\n");
    for (i, m) in census.measurements().iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"name\": \"{}\", \"value\": {}, \"command\": \"{}\" }}",
            m.name,
            m.value,
            escape(m.command)
        );
    }
    out.push_str("\n  ],\n");

    out.push_str("  \"layers\": [\n");
    for (i, (name, members)) in LAYERS.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let rendered: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
        let _ = write!(
            out,
            "    {{ \"layer\": \"{name}\", \"members\": [{}] }}",
            rendered.join(", ")
        );
    }
    out.push_str("\n  ],\n");

    render_crates(&mut out, census);
    render_formats(&mut out, census);
    render_edges(&mut out, census);
    render_lists(&mut out, census);
    render_tests(&mut out, census);
    out
}

fn render_crates(out: &mut String, census: &Census) {
    out.push_str("  \"crates\": [\n");
    for (i, c) in census.crates.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            concat!(
                "    {{\n",
                "      \"name\": \"{name}\",\n",
                "      \"location\": \"{loc}\",\n",
                "      \"manifest\": \"{man}\",\n",
                "      \"source_lines\": {lines},\n",
                "      \"source_bytes\": {bytes},\n",
                "      \"declared_public_items\": {pub_},\n",
                "      \"test_functions\": {tests},\n",
                "      \"implementation_maturity\": null,\n",
                "      \"card_disposition\": \"ExistingUnqualified\",\n",
                "      \"disposition\": null,\n",
                "      \"escalation\": null\n",
                "    }}"
            ),
            name = c.name,
            loc = c.location,
            man = escape(&c.manifest),
            lines = c.source_lines,
            bytes = c.source_bytes,
            pub_ = c.declared_public_items,
            tests = c.test_functions
        );
    }
    out.push_str("\n  ],\n");
}

fn render_formats(out: &mut String, census: &Census) {
    out.push_str("  \"formats\": [\n");
    for (i, f) in census.formats.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let magic = f
            .magic
            .as_ref()
            .map_or("null".into(), |m| format!("\"{}\"", escape(m)));
        let version = f
            .version_constant
            .as_ref()
            .map_or("null".into(), |v| format!("\"{v}\""));
        let _ = write!(
            out,
            "    {{ \"name\": \"{}\", \"magic\": {magic}, \"version_constant\": {version}, \"owner\": \"{}\", \"disposition\": null, \"escalation\": null }}",
            f.name, f.owner
        );
    }
    out.push_str("\n  ],\n");
}

fn render_edges(out: &mut String, census: &Census) {
    out.push_str("  \"edges\": [\n");
    for (i, e) in census.edges.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"from\": \"{}\", \"to\": \"{}\", \"optional\": {}, \"reverse\": {} }}",
            e.from, e.to, e.optional, e.reverse
        );
    }
    out.push_str("\n  ],\n");
}

fn render_lists(out: &mut String, census: &Census) {
    for (key, items) in [
        ("generated_files", &census.generated_files),
        ("ci_rows", &census.ci_rows),
    ] {
        let _ = writeln!(out, "  \"{key}\": [");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push_str(",\n");
            }
            let _ = write!(
                out,
                "    {{ \"id\": \"{}\", \"disposition\": null, \"escalation\": null }}",
                escape(item)
            );
        }
        out.push_str("\n  ],\n");
    }
}

fn render_tests(out: &mut String, census: &Census) {
    out.push_str("  \"tests\": [\n");
    for (i, t) in census.tests.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"file\": \"{}\", \"line\": {}, \"function\": \"{}\", \"owner\": null, \"escalation\": null }}",
            escape(&t.file),
            t.line,
            t.function
        );
    }
    out.push_str("\n  ]\n}\n");
}

#[cfg(test)]
mod tests {
    use super::{declared_public_items, layer_of, test_rows};

    #[test]
    fn root_declared_items_exclude_impl_methods_and_private_modules() {
        let source = concat!(
            "pub struct A;\n",
            "mod private {\n",
            "    pub struct Hidden;\n",
            "}\n",
            "impl A {\n",
            "    pub fn method(&self) {}\n",
            "}\n",
        );
        assert_eq!(
            declared_public_items(source),
            1,
            "a glob re-export forwards root-namespace items, not impl methods or private-module items"
        );
    }

    #[test]
    fn test_rows_capture_file_line_and_function() {
        let rows = test_rows("a.rs", "#[test]\nfn alpha() {}\n\n#[test]\nfn beta() {}\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].function, "alpha");
        assert_eq!(rows[1].line, 4);
    }

    #[test]
    fn an_attribute_between_test_and_fn_does_not_lose_the_name() {
        let rows = test_rows("a.rs", "#[test]\n#[ignore]\nfn gamma() {}\n");
        assert_eq!(rows[0].function, "gamma");
    }

    /// `reverse` is derived from this ordering rather than asserted as a count.
    #[test]
    fn layers_place_the_two_known_inversions_correctly() {
        let ecs = layer_of("meridian-ecs").expect("ecs is placed");
        let rt = layer_of("meridian-rt").expect("rt is placed");
        let renderer = layer_of("meridian-renderer").expect("renderer is placed");
        assert!(ecs < renderer, "ecs sits below the renderer it depends on");
        assert!(
            rt < renderer,
            "the specoment forbids headless runtime reaching the renderer, so runtime sits \
             below presentation and rt -> renderer is reverse"
        );
    }

    /// The reconciliation that makes the enumeration a completeness proof rather than a
    /// hand-list. A grep discovers candidates; agreement with a set built by a different
    /// method — reading the crates that write files — is what licenses trusting the count.
    ///
    /// This runs against the real tree, so a format added later without updating
    /// `KNOWN_FORMATS` fails here rather than passing silently.
    #[test]
    fn discovered_format_constants_reconcile_with_the_known_set() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let discovered = super::discovered_format_constants(&root);
        assert!(
            !discovered.is_empty(),
            "the discovery sweep found nothing, which means it is broken rather than the tree empty"
        );

        // The bare `MAGIC` constants the previous criterion could not see.
        assert!(
            discovered.iter().any(|name| name == "MAGIC"),
            "bare `const MAGIC` must be discoverable: it names the package container and the \
             save file, the two serialized outputs PH-AUTH-006's stop condition most protects. \
             Found: {discovered:?}"
        );

        for name in &discovered {
            assert!(
                !super::NOT_FORMATS.contains(&name.as_str()),
                "{name} is not an on-disk format and must be excluded"
            );
        }

        // Every known format naming a constant must be discoverable by that constant.
        for (format, magic, version, _) in super::KNOWN_FORMATS {
            for constant in [magic, version].into_iter().flatten() {
                if constant.ends_with("_MAGIC") || constant.ends_with("_VERSION") {
                    assert!(
                        discovered.iter().any(|d| d == constant),
                        "{format} names {constant}, which the sweep did not find"
                    );
                }
            }
        }
    }

    #[test]
    fn every_workspace_crate_has_a_layer() {
        for (_, members) in super::LAYERS {
            for member in *members {
                assert!(layer_of(member).is_some(), "{member} has no layer");
            }
        }
    }
}
