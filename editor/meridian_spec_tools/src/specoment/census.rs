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
    /// Items this crate re-exports rather than declares.
    ///
    /// `WP-V1-CENSUS-001` argued for this field explicitly — "a document whose declared test is
    /// 'every public API has one owner and disposition' cannot report 0 for a crate exporting
    /// hundreds" — and then shipped without it, so `meridian-ui` reported 5 while forwarding
    /// roughly 213. Counted as the items reachable through this crate's glob re-exports.
    pub reexported_public_items: usize,
    /// Tests in this crate's `src/` only. The `tests` section counts every `#[test]` in the
    /// workspace including `tests/` and `examples/`, so the two totals differ by design —
    /// stated here so the gap reads as a definition rather than a contradiction.
    pub test_functions: usize,
}

/// One versioned on-disk format. A magic-plus-version pair is one row, not two.
#[derive(Debug, Clone)]
pub struct FormatRow {
    pub name: String,
    pub magic: Option<String>,
    pub version_constant: Option<String>,
    /// The crate that owns this format. Named `owning_crate`, not `owner`: the card requires
    /// "one disposition **and next phase**", and tests additionally need a requirement id, so
    /// a single `owner` field would have meant three different things across three sections
    /// and writing a phase id here would have overwritten `meridian-package`.
    pub owning_crate: String,
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

/// One public item, as a row rather than a per-crate scalar.
///
/// The card's declared test is that every **public API** has one owner and disposition. That is
/// unsatisfiable against a scalar, which is why `WP-V1-CENSUS-001` could report 920 public
/// items and still leave every one of them unaddressable.
#[derive(Debug, Clone)]
pub struct PublicTypeRow {
    pub krate: String,
    pub item: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
}

/// One third-party dependency. `licence` is deliberately null: `OD-006` records that
/// `LEGAL-005` provenance is unmet for exactly these, and this package inventories them
/// without pretending to resolve it.
#[derive(Debug, Clone)]
pub struct DependencyRow {
    pub name: String,
    pub direct: bool,
}

/// One cargo feature definition.
#[derive(Debug, Clone)]
pub struct FeatureRow {
    pub krate: String,
    pub feature: String,
    pub enables: Vec<String>,
}

/// One example target.
#[derive(Debug, Clone)]
pub struct ExampleRow {
    pub krate: String,
    pub name: String,
}

/// One evidence runner: a CI invocation that writes to an `--evidence` path.
#[derive(Debug, Clone)]
pub struct EvidenceRunnerRow {
    pub krate: String,
    /// The source target, or the workflow line for a CI step with no matching target.
    pub target: String,
    /// Where CI writes this runner's evidence, or `None` when nothing invokes it.
    pub evidence_path: Option<String>,
    pub wired_in_ci: bool,
    /// Whether a failure gates the build — from `continue-on-error`, not from the step name.
    pub promoting: bool,
}

/// One row's judgement, resolved from `dispositions.json`.
///
/// Four fields, four legal shapes. See `WP-V1-CENSUS-003`'s row-validity table: exactly one of
/// `disposition`/`escalation`, and `next_phase` null iff exactly one of `phase_pending`/
/// `escalation` names an unresolved owner decision.
#[derive(Debug, Clone, Default)]
pub struct Judgement {
    pub disposition: Option<String>,
    pub escalation: Option<String>,
    pub next_phase: Option<String>,
    pub phase_pending: Option<String>,
    /// Test rows only: the requirement id a retained test serves. The card requires each
    /// retained test to have an owner AND every code area to have a next phase, so these are
    /// two fields, not one.
    pub owner: Option<String>,
}

impl Judgement {
    fn render(&self) -> String {
        let q = |v: &Option<String>| {
            v.as_ref()
                .map_or("null".to_string(), |s| format!("\"{}\"", escape(s)))
        };
        format!(
            "\"disposition\": {}, \"next_phase\": {}, \"phase_pending\": {}, \"escalation\": {}",
            q(&self.disposition),
            q(&self.next_phase),
            q(&self.phase_pending),
            q(&self.escalation)
        )
    }
}

/// The checked-in assignment input.
///
/// Named rules plus an exception list, not ~1,800 opaque rows: a reviewer reads the rules and
/// the exceptions, and bulk assignment is a statement the file makes about itself rather than a
/// suspicion. Keyed by `(section, key)` where key is stable — `file::function` for tests,
/// `crate::item` for public types, never a line number, because any unrelated source edit shifts
/// lines and would orphan every disposition.
///
/// **Deviation from the accepted plan**, recorded rather than silent: the plan named
/// `dispositions.toml`. This is `dispositions.json`. Adding a `toml` crate would add a direct
/// third-party dependency to the very dependency census this package is dispositioning, and
/// `serde_json` is already a direct dependency used throughout this module.
#[derive(Debug, Default)]
pub struct Dispositions {
    /// section -> key -> judgement
    rows: std::collections::BTreeMap<String, std::collections::BTreeMap<String, Judgement>>,
    /// Named rules, evaluated in order against a row key.
    matchers: Vec<Matcher>,
    /// (id, rationale) for reporting.
    pub rules: Vec<(String, String)>,
}

/// One named rule. Matching is substring containment on the row key, deliberately: the keys are
/// `crate`, `crate::item` and `file::function`, and a rule that has to be read by a reviewer is
/// better as "contains `meridian_ui_runtime` and contains `accessib`" than as a regex.
#[derive(Debug, Clone)]
struct Matcher {
    section: String,
    /// Exact key match. Required for crate rules: substring containment let a rule keyed on
    /// `meridian-ui` swallow `meridian-ui-core`, `-editor`, `-render`, `-runtime`, `-semantics`
    /// and `-text`, escalating six crates that had dispositions of their own.
    key_equals: Option<String>,
    key_contains: Vec<String>,
    key_matches_any: Option<Vec<String>>,
    key_excludes: Vec<String>,
    judgement: Judgement,
}

impl Dispositions {
    pub fn load(root: &Path) -> Self {
        let path = root.join(".meridian/implementation/dispositions.json");
        let Ok(text) = fs::read_to_string(&path) else {
            return Self::default();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return Self::default();
        };
        let mut out = Self::default();
        if let Some(rules) = value.get("rules").and_then(serde_json::Value::as_array) {
            for rule in rules {
                let id = rule
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let why = rule
                    .get("rationale")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                out.rules.push((id, why));
            }
        }
        if let Some(items) = value.get("rules").and_then(serde_json::Value::as_array) {
            for item in items {
                let Some(section) = item.get("section").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let list = |name: &str| -> Vec<String> {
                    item.get(name)
                        .and_then(serde_json::Value::as_array)
                        .map(|l| {
                            l.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default()
                };
                let any = item
                    .get("key_matches_any")
                    .and_then(serde_json::Value::as_array)
                    .map(|l| {
                        l.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    });
                out.matchers.push(Matcher {
                    section: section.to_string(),
                    key_equals: field(item, "key_equals"),
                    key_contains: list("key_contains"),
                    key_matches_any: any,
                    key_excludes: list("key_excludes"),
                    judgement: Judgement {
                        disposition: field(item, "disposition"),
                        escalation: field(item, "escalation"),
                        next_phase: field(item, "next_phase"),
                        phase_pending: field(item, "phase_pending"),
                        owner: field(item, "owner"),
                    },
                });
            }
        }
        for group in ["exceptions"] {
            let Some(items) = value.get(group).and_then(serde_json::Value::as_array) else {
                continue;
            };
            for item in items {
                let Some(section) = item.get("section").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let judgement = Judgement {
                    disposition: field(item, "disposition"),
                    escalation: field(item, "escalation"),
                    next_phase: field(item, "next_phase"),
                    phase_pending: field(item, "phase_pending"),
                    owner: field(item, "owner"),
                };
                let keys: Vec<String> = item
                    .get("keys")
                    .and_then(serde_json::Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(|k| k.as_str().map(str::to_string))
                            .collect()
                    })
                    .or_else(|| {
                        item.get("key")
                            .and_then(serde_json::Value::as_str)
                            .map(|k| vec![k.to_string()])
                    })
                    .unwrap_or_default();
                let entry = out.rows.entry(section.to_string()).or_default();
                for key in keys {
                    // Exceptions are loaded after rules and overwrite them by design.
                    entry.insert(key, judgement.clone());
                }
            }
        }
        out
    }

    /// Resolve one row: exact exception first, then the first matching rule.
    ///
    /// Rules are evaluated rather than expanded into rows. Materialising all ~1,800 judgements
    /// produced a file that no one reads and that goes stale the moment the workspace gains a
    /// public item or a test — including the ones this very module adds, which is how the
    /// activated schema first failed on rows the census had just created about itself.
    pub fn get(&self, section: &str, key: &str) -> Judgement {
        if let Some(exact) = self.rows.get(section).and_then(|e| e.get(key)) {
            return exact.clone();
        }
        for rule in &self.matchers {
            if rule.section != section {
                continue;
            }
            if let Some(exact) = &rule.key_equals {
                if exact != key {
                    continue;
                }
            }
            if !rule.key_contains.iter().all(|needle| key.contains(needle)) {
                continue;
            }
            if let Some(pattern) = &rule.key_matches_any {
                if !pattern.iter().any(|needle| key.contains(needle)) {
                    continue;
                }
            }
            if rule.key_excludes.iter().any(|needle| key.contains(needle)) {
                continue;
            }
            return rule.judgement.clone();
        }
        Judgement::default()
    }
}

fn field(item: &serde_json::Value, name: &str) -> Option<String> {
    item.get(name)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// The measured census.
#[derive(Debug, Default)]
pub struct Census {
    pub crates: Vec<CrateRow>,
    pub public_types: Vec<PublicTypeRow>,
    pub dependencies: Vec<DependencyRow>,
    pub features: Vec<FeatureRow>,
    pub examples: Vec<ExampleRow>,
    pub evidence_runners: Vec<EvidenceRunnerRow>,
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
    /// The `mod` path containing this test, or `<root>`. Promised by `WP-V1-CENSUS-001` and
    /// absent from its output, which silently reduced "module granularity" to file
    /// granularity — and one file holds 122 tests.
    pub module: String,
    pub function: String,
}

/// Count `pub` items **declared** at module root.
///
/// Shares [`public_item`]'s predicate so the scalar and the row count cannot disagree. The
/// naive `^pub ` count is 20 higher because it counts `pub use` re-export statements as
/// declarations — which is how `meridian-ui` came to report "5 declared" for a facade that
/// declares nothing at all and forwards 213 items. A re-export is counted in
/// `reexported_public_items`, where it belongs.
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
    text.lines()
        .filter(|line| public_item(line).is_some())
        .count()
}

/// Count items named in `pub use path::{A, B, C};` that come from **another** crate.
///
/// Two corrections live here, both found by review rather than by the tests.
///
/// The input must be stripped of comments and string literals first. Counting raw text made
/// this function count its own documentation: `meridian-spec` declares no `pub use` at all and
/// reported 7 re-exports, every one of them from the doc comments in this file — the same
/// self-measurement class as a `#[test]` grep that reads a source file's own prose.
///
/// And a re-export of the crate's **own** submodule surfaces an item the crate already
/// declares. `meridian-renderer`'s `pub use camera::{Camera, ...}` re-surfaces items already
/// counted in its declared 93, so summing them double-counts. This field exists for exactly
/// one purpose, argued at length in `WP-V1-CENSUS-001`: a facade that declares nothing and
/// exposes hundreds. Only cross-crate re-exports serve that purpose, so `local_modules` — the
/// crate's own module names — are excluded.
fn named_reexport_count(text: &str, local_modules: &[String]) -> usize {
    let code = strip_literals(text);
    let mut total = 0;
    let mut rest = code.as_str();
    while let Some(at) = rest.find("pub use ") {
        rest = &rest[at + 8..];
        let Some(end) = rest.find(';') else { break };
        let statement = &rest[..end];
        // Exact segment match. `trim_start_matches` strips every leading repetition and would
        // mangle a segment that merely begins with those letters (`selfish`, `crateful`).
        let mut segments = statement.split("::");
        let mut first = segments.next().unwrap_or_default().trim();
        if first == "crate" || first == "self" || first == "super" {
            first = segments.next().unwrap_or_default().trim();
        }
        let intra_crate = local_modules.iter().any(|m| m == first);
        if !intra_crate {
            if let (Some(open), Some(close)) = (statement.find('{'), statement.rfind('}')) {
                if close > open {
                    total += statement[open + 1..close]
                        .split(',')
                        .filter(|part| !part.trim().is_empty())
                        .count();
                }
            }
        }
        rest = &rest[end..];
    }
    total
}

/// Classify one module-root `pub` line into (kind, item name).
///
/// Deliberately the same `^pub ` criterion as [`declared_public_items`], so the row count and
/// the scalar cannot disagree — a mismatch between two counts of the same thing was how the
/// format sweep hid a bug in `WP-V1-CENSUS-001`.
fn public_item(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("pub ")?;
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("unsafe ").unwrap_or(rest);
    let (kind, tail) = rest.split_once(' ')?;
    if !matches!(
        kind,
        "fn" | "struct" | "enum" | "trait" | "type" | "const" | "static" | "mod" | "union"
    ) {
        return None;
    }
    let item = tail
        .split(['(', '<', ':', ' ', ';', '{'])
        .next()?
        .trim()
        .to_string();
    if item.is_empty() {
        None
    } else {
        Some((kind.to_string(), item))
    }
}

/// `pub use some_crate::*;` — the statement that makes a façade's declared count a lie.
fn glob_reexport(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("pub use ")?;
    let target = rest.strip_suffix("::*;")?;
    Some(target.replace('_', "-"))
}

/// Blank every literal and comment in a file, preserving line structure exactly.
///
/// Whole-file rather than per-line, because Rust literals span lines: a raw string
/// `r#"{{ ... }}"#` holding JSON braces defeats any per-line stripper, which resets its state
/// at each newline and then counts the fixture's braces as code.
///
/// Line-for-line correspondence is a hard requirement — the caller indexes this output by
/// source line number — so every newline is emitted unconditionally and the state machine
/// never advances past one. An earlier draft skipped two characters at each escape, which
/// could step over a newline: it lost four lines out of 1,784, shifted the output by four, and
/// attributed `mod tests {`'s brace to the `#[cfg(test)]` above it.
fn strip_literals(text: &str) -> String {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Code,
        LineComment,
        BlockComment(usize),
        Str,
        Char,
        Raw(usize),
    }

    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut state = State::Code;
    let mut escaped = false;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            out.push('\n');
            if state == State::LineComment {
                state = State::Code;
            }
            escaped = false;
            i += 1;
            continue;
        }
        match state {
            State::LineComment => {}
            State::BlockComment(depth) => {
                if c == '*' && chars.get(i + 1) == Some(&'/') {
                    state = if depth == 1 {
                        State::Code
                    } else {
                        State::BlockComment(depth - 1)
                    };
                    i += 1;
                } else if c == '/' && chars.get(i + 1) == Some(&'*') {
                    state = State::BlockComment(depth + 1);
                    i += 1;
                }
            }
            State::Str | State::Char => {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if (state == State::Str && c == '"') || (state == State::Char && c == '\'') {
                    state = State::Code;
                }
            }
            State::Raw(hashes) => {
                if c == '"' {
                    let closed = (0..hashes).all(|k| chars.get(i + 1 + k) == Some(&'#'));
                    if closed {
                        i += hashes;
                        state = State::Code;
                    }
                }
            }
            State::Code => {
                if c == '/' && chars.get(i + 1) == Some(&'/') {
                    state = State::LineComment;
                } else if c == '/' && chars.get(i + 1) == Some(&'*') {
                    state = State::BlockComment(1);
                    i += 1;
                } else if c == '"' {
                    state = State::Str;
                } else if c == '\'' && is_char_literal(&chars, i) {
                    state = State::Char;
                } else if let Some(hashes) = raw_string_at(&chars, i) {
                    // Skip `r`/`br` and its hashes; the opening quote lands us in Raw.
                    let lead = usize::from(c == 'b');
                    i += lead + 1 + hashes;
                    state = State::Raw(hashes);
                } else {
                    out.push(c);
                }
            }
        }
        i += 1;
    }
    out
}

/// Strip comments but keep string literals, for searches whose target lives inside a string.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//") {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// A char literal closes within a few characters; a lifetime (`&'a T`) never does.
fn is_char_literal(chars: &[char], i: usize) -> bool {
    if chars.get(i + 1) == Some(&'\\') {
        return (2..=5).any(|k| chars.get(i + k) == Some(&'\''));
    }
    chars.get(i + 2) == Some(&'\'')
}

/// `r"`, `r#"`, `br"`, `br##"` … returns the hash count if this position opens a raw string.
fn raw_string_at(chars: &[char], i: usize) -> Option<usize> {
    let start = match chars.get(i) {
        Some('r') => i,
        Some('b') if chars.get(i + 1) == Some(&'r') => i + 1,
        _ => return None,
    };
    let mut h = start + 1;
    while chars.get(h) == Some(&'#') {
        h += 1;
    }
    if chars.get(h) == Some(&'"') {
        Some(h - start - 1)
    } else {
        None
    }
}

fn test_rows(relative: &str, text: &str) -> Vec<TestRow> {
    let lines: Vec<&str> = text.lines().collect();
    let stripped = strip_literals(text);
    let code_lines: Vec<&str> = stripped.lines().collect();
    let mut rows = Vec::new();
    // Track `mod` nesting by brace depth so each test carries the module path the predecessor
    // promised. Depth is counted on the raw text, which is why `module_path` is a stack rather
    // than a single name: nested `mod tests { mod inner { ... } }` occurs in this workspace.
    let mut stack: Vec<(String, isize)> = Vec::new();
    let mut depth = 0isize;
    for (offset, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("mod ")
            .or_else(|| trimmed.strip_prefix("pub mod "))
        {
            if let Some(name) = rest.split(['{', ';', ' ']).next() {
                if trimmed.ends_with('{') {
                    stack.push((name.to_string(), depth));
                }
            }
        }
        if trimmed == "#[test]" {
            if let Some(name) = lines[offset + 1..].iter().find_map(|l| {
                let t = l.trim();
                t.strip_prefix("fn ")
                    .or_else(|| t.strip_prefix("async fn "))
                    .and_then(|r| r.split(['(', '<']).next())
            }) {
                rows.push(TestRow {
                    file: relative.to_string(),
                    line: offset + 1,
                    module: if stack.is_empty() {
                        "<root>".to_string()
                    } else {
                        stack
                            .iter()
                            .map(|(n, _)| n.as_str())
                            .collect::<Vec<_>>()
                            .join("::")
                    },
                    function: name.to_string(),
                });
            }
        }
        // Count braces on code only. On raw text, `assert!(json.ends_with("}}"))` and a
        // multi-line `r#"{{...}}"#` JSON fixture both corrupt the depth and pop `mod tests`
        // early. CENSUS-003 keys its test map on this field.
        let code = code_lines.get(offset).copied().unwrap_or("");
        let opens = isize::try_from(code.matches('{').count()).unwrap_or(0);
        let closes = isize::try_from(code.matches('}').count()).unwrap_or(0);
        depth += opens - closes;
        while stack.last().is_some_and(|(_, d)| depth <= *d) {
            stack.pop();
        }
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
/// Enumerate evidence runners from source, then mark which CI wires up and how.
///
/// `promoting` is derived from `continue-on-error`, which is what actually decides whether a
/// failure gates the build. An earlier draft keyed off the literal string "non-promoting" in
/// the step name: correct only because the label and the flag happen to coincide today, and
/// silently wrong the moment someone edits a step title.
fn evidence_runners(root: &Path) -> Vec<EvidenceRunnerRow> {
    let mut rows: Vec<EvidenceRunnerRow> = Vec::new();
    for relative in input_files(root) {
        if !is_rust(&relative) {
            continue;
        }
        // A runner is a runnable target: an example, or a binary. A library that parses the
        // flag is the implementation, not the runner, and a comment mentioning the flag — this
        // file mentions it twice — is neither.
        let runnable = relative.contains("/examples/")
            || relative.ends_with("/src/main.rs")
            || relative.contains("/src/bin/");
        if !runnable {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&relative)) else {
            continue;
        };
        let mentions = strip_comments(&text).contains("--evidence");
        if !mentions {
            continue;
        }
        let krate = relative
            .split('/')
            .nth(1)
            .unwrap_or("unknown")
            .replace('_', "-");
        rows.push(EvidenceRunnerRow {
            krate,
            target: relative.clone(),
            evidence_path: None,
            wired_in_ci: false,
            promoting: false,
        });
    }

    for workflow in ["ci.yml", "discord-ci.yml"] {
        let Ok(text) = fs::read_to_string(root.join(".github/workflows").join(workflow)) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        for (offset, line) in lines.iter().enumerate() {
            let Some(at) = line.find("--evidence ") else {
                continue;
            };
            let Some(path) = line[at + 11..].split_whitespace().next() else {
                continue;
            };
            let step_start = lines[..=offset]
                .iter()
                .rposition(|l| {
                    let t = l.trim_start();
                    t.starts_with("- name:") || t.starts_with("- run:")
                })
                .unwrap_or(0);
            let step_end = lines
                .iter()
                .enumerate()
                .skip(offset + 1)
                .find(|(_, l)| {
                    let t = l.trim_start();
                    t.starts_with("- name:") || t.starts_with("- run:")
                })
                .map_or(lines.len(), |(i, _)| i);
            let promoting = !lines[step_start..step_end]
                .iter()
                .any(|l| l.contains("continue-on-error: true"));
            let example = lines[step_start..step_end]
                .iter()
                .find_map(|l| {
                    l.find("--example ")
                        .map(|i| l[i + 10..].split_whitespace().next())
                })
                .flatten();
            let target = rows
                .iter()
                .find(|r| example.is_some_and(|e| r.target.contains(e)))
                .map(|r| r.target.clone());
            if let Some(target) = target {
                if let Some(row) = rows.iter_mut().find(|r| r.target == target) {
                    row.evidence_path = Some(path.to_string());
                    row.wired_in_ci = true;
                    row.promoting = promoting;
                    continue;
                }
            }
            let krate = lines[step_start..step_end]
                .iter()
                .find_map(|l| {
                    l.find(" -p ")
                        .and_then(|i| l[i + 4..].split_whitespace().next())
                })
                .unwrap_or("unknown")
                .to_string();
            rows.push(EvidenceRunnerRow {
                krate,
                target: format!("{workflow}:{}", offset + 1),
                evidence_path: Some(path.to_string()),
                wired_in_ci: true,
                promoting,
            });
        }
    }
    rows.sort_by(|a, b| (&a.krate, &a.target).cmp(&(&b.krate, &b.target)));
    rows
}

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
            owning_crate: (*owner).to_string(),
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

    // Evidence runners: every target that accepts `--evidence`, unioned with the CI steps
    // that invoke one. Enumerating only CI call sites missed three runners that exist in the
    // repository but are not wired into a workflow — precisely what a requalification census
    // must surface, since an unwired runner has no owner and no disposition otherwise.
    census.evidence_runners = evidence_runners(root);

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
/// Dependencies, features and examples, all read straight off one package's manifest.
///
/// Split out of [`absorb_metadata`] purely for length; it carries no logic of its own beyond
/// deduplicating third-party dependencies by name.
fn absorb_manifest_sections(
    census: &mut Census,
    package: &serde_json::Value,
    name: &str,
    workspace: &[String],
) {
    let empty = Vec::new();
    // Dependencies, features and examples all come straight off the manifest.
    for dep in package
        .get("dependencies")
        .and_then(serde_json::Value::as_array)
        .unwrap_or(&empty)
    {
        if let Some(dep_name) = dep.get("name").and_then(serde_json::Value::as_str) {
            if !workspace.iter().any(|n| n == dep_name)
                && !census.dependencies.iter().any(|d| d.name == dep_name)
            {
                census.dependencies.push(DependencyRow {
                    name: dep_name.to_string(),
                    direct: true,
                });
            }
        }
    }
    if let Some(features) = package
        .get("features")
        .and_then(serde_json::Value::as_object)
    {
        for (feature, enables) in features {
            census.features.push(FeatureRow {
                krate: name.to_string(),
                feature: feature.clone(),
                enables: enables
                    .as_array()
                    .map(|list| {
                        list.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }
    }
    for target in package
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .unwrap_or(&empty)
    {
        let is_example = target
            .get("kind")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|k| k.iter().any(|v| v.as_str() == Some("example")));
        if is_example {
            if let Some(example) = target.get("name").and_then(serde_json::Value::as_str) {
                census.examples.push(ExampleRow {
                    krate: name.to_string(),
                    name: example.to_string(),
                });
            }
        }
    }
}

/// What one crate's `src/` tree yields. Split out of [`absorb_metadata`] for length.
struct SourceScan {
    lines: usize,
    bytes: u64,
    public: usize,
    /// Declarations in `lib.rs` alone. A glob forwards the crate-root namespace, so this — not
    /// the whole-tree count — is what a facade re-exporting this crate actually gains.
    root_public: usize,
    files: Vec<String>,
    globs: Vec<String>,
    /// Items named individually in `pub use path::{A, B}`. A glob-only count misses these
    /// entirely: `meridian-ui` names 12 from `meridian_ui_text` and they were counted neither
    /// as declared nor re-exported.
    named_reexports: usize,
}

/// Walk one crate's sources, accumulating the scalars and pushing its public-item rows.
///
/// Pushes into `census.public_types` as it goes so the rows and the scalar come from a single
/// pass over the same text — two passes could disagree, and a disagreement between two counts
/// of the same thing is what hid a bug in the predecessor's format sweep.
fn scan_crate_sources(
    census: &mut Census,
    root: &Path,
    source_dir: &Path,
    name: &str,
) -> SourceScan {
    let (mut lines, mut bytes, mut public) = (0usize, 0u64, 0usize);
    let (mut root_public, mut named_reexports) = (0usize, 0usize);
    let mut files = Vec::new();
    let mut globs: Vec<String> = Vec::new();
    collect(source_dir, root, &mut files);
    for relative in &files {
        if !is_rust(relative) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            continue;
        };
        lines += text.lines().count();
        bytes += text.len() as u64;
        public += declared_public_items(&text);
        if relative.ends_with("/src/lib.rs") {
            root_public += declared_public_items(&text);
        }

        for (offset, line) in text.lines().enumerate() {
            if let Some((kind, item)) = public_item(line) {
                census.public_types.push(PublicTypeRow {
                    krate: name.to_string(),
                    item,
                    kind,
                    file: relative.clone(),
                    line: offset + 1,
                });
            }
            if let Some(target) = glob_reexport(line) {
                globs.push(target);
            }
        }
    }
    // Module names of this crate, so a re-export of its own submodule is not counted as
    // exposing something it does not declare.
    let mut local_modules: Vec<String> = files
        .iter()
        .filter_map(|f| {
            let stem = f.rsplit('/').next()?.strip_suffix(".rs")?;
            if stem == "lib" || stem == "main" || stem == "mod" {
                f.rsplit('/').nth(1).map(str::to_string)
            } else {
                Some(stem.to_string())
            }
        })
        .collect();
    // File stems alone miss a module declared inline (`mod foo { ... }` with no `foo.rs`), and
    // a `pub use foo::{A, B}` of one would then be counted as cross-crate. No crate in the
    // workspace does this today, so this closes a latent gap rather than fixing a live error.
    for relative in &files {
        if !is_rust(relative) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            continue;
        };
        for line in strip_literals(&text).lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed
                .strip_prefix("mod ")
                .or_else(|| trimmed.strip_prefix("pub mod "))
                .or_else(|| trimmed.strip_prefix("pub(crate) mod "))
                .or_else(|| trimmed.strip_prefix("pub(super) mod "))
            else {
                continue;
            };
            if let Some(name) = rest.split(['{', ';', ' ']).next() {
                if !name.is_empty() {
                    local_modules.push(name.to_string());
                }
            }
        }
    }
    local_modules.sort();
    local_modules.dedup();
    for relative in &files {
        if !is_rust(relative) {
            continue;
        }
        if let Ok(text) = fs::read_to_string(root.join(relative)) {
            named_reexports += named_reexport_count(&text, &local_modules);
        }
    }

    SourceScan {
        lines,
        bytes,
        public,
        root_public,
        files,
        globs,
        named_reexports,
    }
}

/// Resolve re-exports once every crate's crate-root declaration count exists.
///
/// A glob forwards the **crate-root** namespace, so the sum is over `lib.rs` declarations
/// alone, not the whole `src/` tree. An earlier draft summed whole-tree counts while its own
/// doc comment said crate-root, producing 210 where the glob actually forwards 202 — the same
/// 213-vs-205 ambiguity `WP-V1-CENSUS-001` wrote three paragraphs about and then did not
/// resolve. Named re-exports (`pub use path::{A, B}`) are added on top, because a glob-only
/// count misses them entirely.
///
/// This cannot run inside the loop that produces the counts, which is why it is a second pass.
fn resolve_glob_reexports(
    census: &mut Census,
    glob_targets: &[(String, Vec<String>, usize)],
    root_declared: &[(String, usize)],
) {
    // the crate-root namespace of each crate it globs, so its re-exported count is the sum of
    // those crates' declared counts.
    let declared: Vec<(String, usize)> = root_declared.to_vec();
    for (krate, globs, named) in glob_targets {
        let total: usize = globs
            .iter()
            .filter_map(|target| {
                declared
                    .iter()
                    .find(|(n, _)| n == target)
                    .map(|(_, count)| *count)
            })
            .sum();
        if let Some(row) = census.crates.iter_mut().find(|c| &c.name == krate) {
            row.reexported_public_items = total + named;
        }
    }
}

pub fn absorb_metadata(census: &mut Census, root: &Path, metadata: &serde_json::Value) {
    // Canonicalise once. `--root` defaults to `.`, while `cargo metadata` emits absolute
    // manifest paths, so a strip against the uncanonicalised root never matched: absolute
    // paths leaked into the output and every per-crate measurement came back zero because
    // `strip_prefix` failed on every source file. Both bugs shipped, and no test caught them
    // because every assertion was about structure rather than about the values being sane.
    let root = &root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
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

    let mut glob_targets: Vec<(String, Vec<String>, usize)> = Vec::new();
    let mut root_declared: Vec<(String, usize)> = Vec::new();

    for package in &members {
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        absorb_manifest_sections(census, package, name, &names);
        let manifest = package
            .get("manifest_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        // Absolute paths would make any comparison machine-dependent.
        let manifest = Path::new(manifest).strip_prefix(root).map_or_else(
            |_| manifest.to_string(),
            |relative| relative.to_string_lossy().to_string(),
        );
        let source_dir = root
            .join(manifest.trim_end_matches("Cargo.toml"))
            .join("src");

        let scan = scan_crate_sources(census, root, &source_dir, name);
        let (lines, bytes, public) = (scan.lines, scan.bytes, scan.public);
        let (files, globs) = (scan.files.clone(), scan.globs.clone());
        glob_targets.push((name.to_string(), globs, scan.named_reexports));
        root_declared.push((name.to_string(), scan.root_public));
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
            // Filled in below: resolving a glob needs every crate's declared count, so it
            // cannot be computed inside the loop that produces those counts.
            reexported_public_items: 0,
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

    resolve_glob_reexports(census, &glob_targets, &root_declared);
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
                name: "test_functions_total",
                value: self.tests.len(),
                command:
                    "grep -rhE '^[[:space:]]*#\\[test\\]$' --include='*.rs' engine editor | wc -l",
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
/// Validate a rendered census against the checked-in schema, and check that every
/// `escalation` names an `OD-*` that actually exists in `state.json`.
///
/// The schema constrains shape; only this function can constrain existence, and without it
/// `escalation` is a free string wearing an `OD-*` costume — which would make "escalation
/// count equals open owner decisions" decoration rather than a control.
pub fn schema_problems(root: &Path, rendered: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(rendered) else {
        return vec!["census is not valid JSON".to_string()];
    };
    let schema_path = root.join("governance/schemas/census.schema.json");
    let Ok(schema_text) = fs::read_to_string(&schema_path) else {
        return vec!["governance/schemas/census.schema.json is missing".to_string()];
    };
    let Ok(schema) = serde_json::from_str::<serde_json::Value>(&schema_text) else {
        return vec!["census schema is not valid JSON".to_string()];
    };
    match jsonschema::validator_for(&schema) {
        Ok(validator) => {
            for error in validator.iter_errors(&value) {
                problems.push(format!("schema: {error}"));
            }
        }
        Err(error) => problems.push(format!("census schema does not compile: {error}")),
    }

    // UNRESOLVED records only, scoped to open_owner_decisions. An escalation naming a settled
    // decision would otherwise pass a check this package calls machine-checkable: OD-007 and
    // OD-011 both carry `resolved`, and the earlier harvester accepted any 6-character OD- string
    // anywhere in the file.
    let known: Vec<String> = fs::read_to_string(root.join(".meridian/implementation/state.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .map(|state| {
            state
                .get("open_owner_decisions")
                .and_then(serde_json::Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter(|entry| entry.get("resolved").is_none())
                        .filter_map(|entry| {
                            entry
                                .get("id")
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_string)
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();
    for (section, rows) in value.as_object().into_iter().flatten() {
        let Some(rows) = rows.as_array() else {
            continue;
        };
        for row in rows {
            for name in ["escalation", "phase_pending"] {
                let Some(id) = row.get(name).and_then(serde_json::Value::as_str) else {
                    continue;
                };
                if !known.iter().any(|k| k == id) {
                    problems.push(format!(
                        "{section}: {name} {id} names no unresolved OD-* record in state.json"
                    ));
                }
            }
            let Some(id) = row.get("escalation").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !known.iter().any(|k| k == id) {
                problems.push(format!(
                    "{section}: escalation {id} names no unresolved OD-* record in state.json"
                ));
            }
        }
    }
    problems
}

/// The assignment constraints `WP-V1-CENSUS-003` declares, as live rules rather than prose.
///
/// A budget with no mechanism is decoration, and a ceiling with no floor punishes only honesty:
/// bulk-assigning every test to a plausible id produces zero escalations and passes a ceiling
/// comfortably. These are the constraints that fail the build.
pub fn assignment_problems(rendered: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(rendered) else {
        return problems;
    };
    let tests = value
        .get("tests")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tests.is_empty() {
        return problems;
    }

    let owner_of = |row: &serde_json::Value| {
        row.get("owner")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    };
    let mapped: Vec<String> = tests.iter().filter_map(owner_of).collect();
    if mapped.is_empty() {
        return problems;
    }

    // Per-family cap, derived in step 1 from the measured meridian-spec and meridian-ui-editor
    // mappings rather than pre-set. The UI family is the binding one at 35.3%; 45% is the
    // measured maximum plus headroom, and a pre-set 25% would have failed before any judgement.
    let mut families: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for id in &mapped {
        let family = id.rsplit_once('-').map_or(id.as_str(), |(head, _)| head);
        *families.entry(family.to_string()).or_default() += 1;
    }
    for (family, count) in &families {
        // Integer arithmetic: a percentage of a row count needs no float, and the cast lint is
        // a real signal on a value derived from `usize`.
        if count * 100 > mapped.len() * 45 {
            problems.push(format!(
                "requirement family {family} owns {count} of {} mapped tests ({}%), over the derived 45% cap",
                mapped.len(),
                count * 100 / mapped.len()
            ));
        }
    }

    // No crate with five or more mapped tests may map them all to one id AND have no escalated
    // test. The rule exists to catch bulk assignment, and its premise is that a crate that large
    // serves more than one requirement. An escalated test satisfies that premise with better
    // evidence than a second id would: it records a second subject whose owner does not exist,
    // rather than inventing one that plausibly might. Four crates reached this state when the
    // review escalated 124 mappings to scope-level requirements no unit test can serve.
    let mut by_crate: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut escalated: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for row in &tests {
        if let (Some(file), Some(_)) = (
            row.get("file").and_then(serde_json::Value::as_str),
            row.get("escalation").and_then(serde_json::Value::as_str),
        ) {
            escalated.insert(file.split('/').nth(1).unwrap_or("?").to_string());
        }
    }
    for row in &tests {
        let (Some(file), Some(owner)) = (
            row.get("file").and_then(serde_json::Value::as_str),
            owner_of(row),
        ) else {
            continue;
        };
        // `engine/meridian_foo/src/...` -> `meridian_foo`
        let krate = file.split('/').nth(1).unwrap_or("?").to_string();
        by_crate.entry(krate.clone()).or_default().insert(owner);
        *counts.entry(krate).or_default() += 1;
    }
    for (krate, ids) in &by_crate {
        let n = counts.get(krate).copied().unwrap_or(0);
        if n >= 5 && ids.len() == 1 && !escalated.contains(krate) {
            problems.push(format!(
                "{krate} maps all {n} of its mapped tests to one id ({}) and escalates none; a crate that large serves more than one requirement",
                ids.iter().next().map_or("?", String::as_str)
            ));
        }
    }
    problems
}

pub fn render(census: &Census, root: &Path, specoment_sha256: &str) -> String {
    let judged = Dispositions::load(root);
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
        "  \"assignment\": \"WP-V1-CENSUS-003 assigned every judgement-bearing row. A row carries exactly one of disposition and escalation, and next_phase is null only when phase_pending or escalation names an unresolved owner decision - four legal shapes, verified by exhaustive cross-product over all sixteen combinations of the four judgement fields. The edges and layers sections carry no judgement fields at all and are exempt by construction, stated here because CENSUS-001 claimed every row and was wrong for 104 of them.\","
    );

    // Limitations, recorded in the artefact itself. The recurring failure in this package's
    // lineage is a limitation documented where the next reader will not look.
    let _ = writeln!(
        out,
        "  \"limitations\": [\n    \"dependencies lists the 18 declared direct third-party crates, not the 494 packages Cargo.lock resolves; OD-006's LEGAL-005 question covers all 494 and is not answered here\",\n    \"public_types uses a column-0 `pub <kind>` predicate: 3 known macro-generated public types (including UiNodeId) have no row, and named re-exports are counted in reexported_public_items rather than as declarations\",\n    \"reexported_public_items counts CROSS-CRATE re-exports only: crate-root declarations of globbed crates plus items named in pub use path::{{A, B}} from another crate. The whole-tree reading is rejected because a glob forwards only the crate root, and intra-crate re-exports are excluded because they re-surface items the crate already declares - see the entry naming meridian-renderer, meridian-platform and meridian-ui-render\",\n    \"ci_rows counts workflow jobs, not matrix expansions: the rust job runs a 3-OS matrix and is one row\",\n    \"format_migrations and forbidden-edge reasons are not yet present; carried to WP-V1-CENSUS-003\",\n    \"tests and generated_files and ci_rows carry no next_phase field; carried to WP-V1-CENSUS-003\",\n    \"collect_od_ids harvests any 6-character OD- string and any id key beginning OD-, unscoped to open_owner_decisions and unfiltered by status; it gates nothing today because every escalation is null\",\n    \"test_rows drops a #[test] whose fn line it cannot parse, where earlier code emitted function: unknown; nothing is dropped today, but a future drop would be silent\",\n    \"intra-crate re-exports excluded by the rule above, with the counts they would otherwise have contributed: meridian-renderer 84, meridian-platform 3, meridian-ui-render 3. Local module names are derived from file stems plus inline mod declarations\",\n    \"the schema validates the census the generator produces, not the file on disk; a hand-edit is caught as stale-census by byte comparison, not as a schema violation\"\n  ],"
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

    render_crates(&mut out, census, &judged);
    render_public_types(&mut out, census, &judged);
    render_dependencies(&mut out, census, &judged);
    render_features(&mut out, census, &judged);
    render_examples(&mut out, census, &judged);
    render_evidence_runners(&mut out, census, &judged);
    render_formats(&mut out, census, &judged);
    render_edges(&mut out, census);
    render_lists(&mut out, census, &judged);
    render_tests(&mut out, census, &judged);
    out
}

fn render_crates(out: &mut String, census: &Census, judged: &Dispositions) {
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
                "      \"reexported_public_items\": {reexp},\n",
                "      \"test_functions\": {tests},\n",
                "      \"implementation_maturity\": null,\n",
                "      \"card_disposition\": \"ExistingUnqualified\",\n",
                "      {judgement}\n",
                "    }}"
            ),
            name = c.name,
            loc = c.location,
            man = escape(&c.manifest),
            lines = c.source_lines,
            bytes = c.source_bytes,
            pub_ = c.declared_public_items,
            reexp = c.reexported_public_items,
            tests = c.test_functions,
            judgement = judged.get("crates", &c.name).render()
        );
    }
    out.push_str("\n  ],\n");
}

fn render_public_types(out: &mut String, census: &Census, judged: &Dispositions) {
    out.push_str("  \"public_types\": [\n");
    for (i, t) in census.public_types.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"crate\": \"{}\", \"item\": \"{}\", \"kind\": \"{}\", \"file\": \"{}\", \"line\": {}, {} }}",
            t.krate,
            escape(&t.item),
            t.kind,
            escape(&t.file),
            t.line,
            judged.get("public_types", &format!("{}::{}", t.krate, t.item)).render()
        );
    }
    out.push_str("\n  ],\n");
}

fn render_dependencies(out: &mut String, census: &Census, judged: &Dispositions) {
    out.push_str("  \"dependencies\": [\n");
    for (i, d) in census.dependencies.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"name\": \"{}\", \"direct\": {}, \"licence\": null, {} }}",
            escape(&d.name),
            d.direct,
            judged.get("dependencies", &d.name).render()
        );
    }
    out.push_str("\n  ],\n");
}

fn render_features(out: &mut String, census: &Census, judged: &Dispositions) {
    out.push_str("  \"features\": [\n");
    for (i, f) in census.features.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let enables: Vec<String> = f
            .enables
            .iter()
            .map(|e| format!("\"{}\"", escape(e)))
            .collect();
        let _ = write!(
            out,
            "    {{ \"crate\": \"{}\", \"feature\": \"{}\", \"enables\": [{}], {} }}",
            f.krate,
            escape(&f.feature),
            enables.join(", "),
            judged
                .get("features", &format!("{}::{}", f.krate, f.feature))
                .render()
        );
    }
    out.push_str("\n  ],\n");
}

fn render_examples(out: &mut String, census: &Census, judged: &Dispositions) {
    out.push_str("  \"examples\": [\n");
    for (i, e) in census.examples.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"crate\": \"{}\", \"name\": \"{}\", {} }}",
            e.krate,
            escape(&e.name),
            judged
                .get("examples", &format!("{}::{}", e.krate, e.name))
                .render()
        );
    }
    out.push_str("\n  ],\n");
}

fn render_evidence_runners(out: &mut String, census: &Census, judged: &Dispositions) {
    out.push_str("  \"evidence_runners\": [\n");
    for (i, r) in census.evidence_runners.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"crate\": \"{}\", \"target\": \"{}\", \"evidence_path\": {}, \"wired_in_ci\": {}, \"promoting\": {}, {} }}",
            escape(&r.krate),
            escape(&r.target),
            r.evidence_path
                .as_ref()
                .map_or("null".to_string(), |p| format!("\"{}\"", escape(p))),
            r.wired_in_ci,
            r.promoting,
            judged.get("evidence_runners", &r.target).render()
        );
    }
    out.push_str("\n  ],\n");
}

fn render_formats(out: &mut String, census: &Census, judged: &Dispositions) {
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
            "    {{ \"name\": \"{}\", \"magic\": {magic}, \"version_constant\": {version}, \"owning_crate\": \"{}\", {} }}",
            f.name, f.owning_crate, judged.get("formats", &f.name).render()
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

fn render_lists(out: &mut String, census: &Census, judged: &Dispositions) {
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
                "    {{ \"id\": \"{}\", {} }}",
                escape(item),
                judged.get(key, item).render()
            );
        }
        out.push_str("\n  ],\n");
    }
}

fn render_tests(out: &mut String, census: &Census, judged: &Dispositions) {
    out.push_str("  \"tests\": [\n");
    for (i, t) in census.tests.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let key = format!("{}::{}", t.file, t.function);
        let j = judged.get("tests", &key);
        let _ = write!(
            out,
            "    {{ \"file\": \"{}\", \"line\": {}, \"module\": \"{}\", \"function\": \"{}\", \"owner\": {}, {} }}",
            escape(&t.file),
            t.line,
            escape(&t.module),
            t.function,
            // `owner` on a test row means the requirement id it serves - the third meaning the
            // field carries, disambiguated in WP-V1-CENSUS-002 and kept here.
            j.owner
                .as_ref()
                .map_or("null".to_string(), |o| format!("\"{o}\"")),
            j.render()
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

    /// Compose the same way `census_json` does. `measure` alone leaves every
    /// metadata-derived section empty, so a test calling it would assert against a census the
    /// product never produces.
    fn full_census(root: &std::path::Path) -> super::Census {
        let output = std::process::Command::new("cargo")
            .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
            .current_dir(root)
            .output()
            .expect("cargo metadata runs");
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("metadata is json");
        let mut census = super::measure(root);
        super::absorb_metadata(&mut census, root, &metadata);
        census
    }

    /// The card names ten inventory axes. `WP-V1-CENSUS-001` delivered five and the phase was
    /// nearly closed on it. This enumerates them from the card's own wording.
    #[test]
    fn every_card_axis_has_a_section() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        let rendered = super::render(&census, &root, "x");
        let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid json");
        // "Inventory every crate, public type, source format, backend dependency, feature,
        // example, test, evidence runner, generated file, and CI row."
        for axis in [
            "crates",
            "public_types",
            "formats",
            "dependencies",
            "features",
            "examples",
            "tests",
            "evidence_runners",
            "generated_files",
            "ci_rows",
        ] {
            let rows = value.get(axis).and_then(serde_json::Value::as_array);
            assert!(
                rows.is_some_and(|r| !r.is_empty()),
                "card axis {axis} has no populated section"
            );
        }
    }

    /// The evidence-runner section was rebuilt from scratch, so its shape is pinned rather
    /// than merely floored: five example targets exist, two are wired into CI, and exactly one
    /// invocation gates the build.
    #[test]
    fn evidence_runner_shape_is_pinned() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        let examples = census
            .evidence_runners
            .iter()
            .filter(|r| r.target.contains("/examples/"))
            .count();
        assert_eq!(examples, 5, "example runners changed");
        assert_eq!(
            census
                .evidence_runners
                .iter()
                .filter(|r| r.wired_in_ci)
                .count(),
            3,
            "CI wiring changed"
        );
        assert_eq!(
            census
                .evidence_runners
                .iter()
                .filter(|r| r.promoting)
                .count(),
            1,
            "the set of build-gating evidence runners changed"
        );
    }

    /// Floors on every axis, so a collapse fails rather than passing "not all zero".
    ///
    /// `no_section_is_uniformly_zero` only ever examined the `crates` section, and every other
    /// new assertion was shape-or-nonzero: a regression dropping 800 of the 901 public types
    /// passed all of them. These are floors, not equalities, so adding a crate or a test does
    /// not break the build; a collapse does.
    #[test]
    fn every_axis_meets_its_floor() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        for (axis, actual, floor) in [
            ("crates", census.crates.len(), 37),
            ("public_types", census.public_types.len(), 880),
            ("dependencies", census.dependencies.len(), 18),
            ("features", census.features.len(), 7),
            ("examples", census.examples.len(), 14),
            ("evidence_runners", census.evidence_runners.len(), 6),
            ("formats", census.formats.len(), 15),
            ("tests", census.tests.len(), 770),
            ("generated_files", census.generated_files.len(), 9),
            ("ci_rows", census.ci_rows.len(), 3),
            ("edges", census.edges.len(), 90),
        ] {
            assert!(
                actual >= floor,
                "{axis} collapsed to {actual}, below its floor of {floor}"
            );
        }
    }

    /// Scalar sanity for the `crates` section specifically. Section *lengths* across all
    /// eleven sections are covered by `every_axis_meets_its_floor`; this name says `crate_`
    /// because its body only ever examined `crates`, and the previous name overclaimed.
    #[test]
    fn crate_scalars_are_not_uniformly_zero() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        assert!(census.crates.iter().any(|c| c.source_bytes > 0));
        assert!(census.crates.iter().any(|c| c.declared_public_items > 0));
        assert!(census.crates.iter().any(|c| c.test_functions > 0));
        assert!(census.crates.iter().any(|c| c.reexported_public_items > 0));
        for row in &census.crates {
            let prefix = if row.location == "engine" {
                "engine/"
            } else {
                "editor/"
            };
            assert!(
                row.manifest.starts_with(prefix),
                "{} claims location {} but its manifest is {}",
                row.name,
                row.location,
                row.manifest
            );
            assert!(
                !row.manifest.starts_with('/'),
                "{} carries an absolute path",
                row.name
            );
        }
    }

    /// The scalar and the rows count the same thing, so they must agree by construction.
    #[test]
    fn public_item_scalar_matches_row_count() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        let scalar: usize = census.crates.iter().map(|c| c.declared_public_items).sum();
        assert_eq!(
            scalar,
            census.public_types.len(),
            "declared_public_items and the public_types rows disagree"
        );
    }

    /// A crate with no `pub use` at module root cannot re-export anything.
    ///
    /// This fails on the unfixed code: `meridian-spec` has zero column-0 `pub use` lines and
    /// reported 7, counted out of this file's own doc comments. The fix without the guard
    /// would leave the next regression exactly as invisible as this one was.
    #[test]
    fn a_crate_without_pub_use_reexports_nothing() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        for row in &census.crates {
            let dir = root
                .join(row.manifest.trim_end_matches("Cargo.toml"))
                .join("src");
            let mut files = Vec::new();
            super::collect(&dir, &root, &mut files);
            let has = files.iter().any(|f| {
                std::fs::read_to_string(root.join(f))
                    .is_ok_and(|t| t.lines().any(|l| l.starts_with("pub use ")))
            });
            assert!(
                has || row.reexported_public_items == 0,
                "{} has no module-root `pub use` but reports {} re-exports",
                row.name,
                row.reexported_public_items
            );
        }
    }

    /// Only cross-crate re-exports count: a crate re-surfacing its own submodule is exposing
    /// something it already declares, and summing both double-counts it.
    #[test]
    fn intra_crate_reexports_are_not_counted() {
        let locals = vec!["camera".to_string()];
        assert_eq!(
            super::named_reexport_count("pub use camera::{Camera, CameraError};", &locals),
            0
        );
        assert_eq!(
            super::named_reexport_count("pub use bevy_ecs::prelude::{A, B};", &locals),
            2
        );
        // Documentation is not public API.
        assert_eq!(
            super::named_reexport_count("/// like `pub use path::{A, B, C};`\nfn f() {}", &locals),
            0
        );
    }

    /// `pub use` is a re-export, not a declaration. Counting it as one is how a facade that
    /// declares nothing reported five declared items.
    #[test]
    fn reexports_are_not_declarations() {
        assert!(super::public_item("pub use meridian_ui_runtime::*;").is_none());
        assert!(super::public_item("pub struct Frame {").is_some());
        assert_eq!(
            super::glob_reexport("pub use meridian_ui_runtime::*;").as_deref(),
            Some("meridian-ui-runtime")
        );
    }

    /// Non-empty is too weak: `<root>` is non-empty and was what a mis-parsed file produced.
    /// A `src/` file that opens `mod tests {` must attribute every test below that line.
    #[test]
    fn tests_inside_a_test_module_are_not_attributed_to_root() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        for file in census
            .tests
            .iter()
            .map(|t| t.file.clone())
            .collect::<std::collections::BTreeSet<_>>()
        {
            if !file.contains("/src/") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(root.join(&file)) else {
                continue;
            };
            let Some(at) = text
                .lines()
                .position(|l| l.trim().ends_with("mod tests {"))
                .map(|p| p + 1)
            else {
                continue;
            };
            for row in census.tests.iter().filter(|t| t.file == file) {
                assert!(
                    !(row.line > at && row.module == "<root>"),
                    "{}:{} {} sits below `mod tests {{` but is attributed to <root>",
                    file,
                    row.line,
                    row.function
                );
            }
        }
    }

    /// Literals and comments must not be counted as code, which is how the depth went wrong.
    #[test]
    fn strip_literals_preserves_lines_and_blanks_literals() {
        let text = "fn a() {\n    let s = r#\"{{\"x\":1}}\"#;\n    // }\n}\n";
        let stripped = super::strip_literals(text);
        assert_eq!(text.lines().count(), stripped.lines().count());
        assert_eq!(
            stripped.matches('{').count(),
            1,
            "raw-string and comment braces leaked into the code count"
        );
        assert_eq!(stripped.matches('}').count(), 1);
    }

    /// Every test row must carry a module, or `CENSUS-003`'s "module granularity" is file
    /// granularity wearing a different word.
    #[test]
    fn every_test_row_has_a_module() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        for row in &census.tests {
            assert!(
                !row.module.is_empty(),
                "{}:{} has no module",
                row.file,
                row.line
            );
        }
    }

    /// Row validity, by exhaustive cross-product rather than enumerated cases.
    ///
    /// Four judgement fields give sixteen states — fewer than the number of cases anyone would
    /// think to write, and it does not depend on thinking of the right ones. Two blocking
    /// review findings were combinations the plan declared invalid and the schema accepted;
    /// both were found by generating the space. This subsumes the individual both-set,
    /// neither-set, malformed and double-naming cases rather than listing them beside it.
    #[test]
    fn exactly_four_of_sixteen_row_shapes_are_legal() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let sections = [
            "crates",
            "public_types",
            "dependencies",
            "features",
            "examples",
            "evidence_runners",
            "formats",
            "tests",
            "generated_files",
            "ci_rows",
        ];
        let template = r#"{"schema":1,"source_tree_checkpoint":"x","specoment_sha256":"x","limitations":[],"measurements":[],"layers":[],"crates":[],"public_types":[],"dependencies":[],"features":[],"examples":[],"evidence_runners":[],"formats":[],"edges":[],"tests":[],"generated_files":[],"ci_rows":[]}"#;
        // The four legal shapes, as WP-V1-CENSUS-003's row-validity table declares them.
        let legal = [
            (true, false, true, false),
            (false, true, true, false),
            (true, false, false, true),
            (false, true, false, false),
        ];
        for section in sections {
            let mut accepted = Vec::new();
            for bits in 0..16u8 {
                let (d, e, n, p) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0);
                let row = format!(
                    r#"{{"disposition":{},"escalation":{},"next_phase":{},"phase_pending":{}}}"#,
                    if d { "\"retain\"" } else { "null" },
                    if e { "\"OD-005\"" } else { "null" },
                    if n { "\"PH-AUTH-006\"" } else { "null" },
                    if p { "\"OD-013\"" } else { "null" }
                );
                let rendered = template.replace(
                    &format!("\"{section}\":[]"),
                    &format!("\"{section}\":[{row}]"),
                );
                if super::schema_problems(&root, &rendered).is_empty() {
                    accepted.push((d, e, n, p));
                }
            }
            assert_eq!(
                accepted.len(),
                4,
                "{section}: {} of 16 combinations accepted, expected exactly 4",
                accepted.len()
            );
            for shape in legal {
                assert!(
                    accepted.contains(&shape),
                    "{section}: legal shape {shape:?} was rejected"
                );
            }
        }
        // A malformed decision id must not pass the shape-3 branch.
        let bad = template.replace(
            "\"crates\":[]",
            r#""crates":[{"disposition":"retain","escalation":null,"next_phase":null,"phase_pending":"OD-13"}]"#,
        );
        assert!(
            !super::schema_problems(&root, &bad).is_empty(),
            "a malformed OD id was accepted"
        );
    }

    /// The assertion whose absence let a fully zeroed census ship.
    ///
    /// Every prior test checked structure — that rows exist, that fields are null, that the
    /// vocabulary is closed. None checked that a measurement measured anything. A census in
    /// which every crate reports 0 bytes, 0 public items and 0 tests satisfied all of them.
    #[test]
    fn measurements_are_not_all_zero() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = super::measure(&root);
        assert!(!census.tests.is_empty(), "the test sweep found nothing");
        assert!(
            census.formats.len() > 10,
            "the format table is suspiciously small: {}",
            census.formats.len()
        );
    }

    #[test]
    fn every_workspace_crate_has_a_layer() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let census = full_census(&root);
        // Iterating LAYERS' own members and asserting they have a layer is true by
        // construction. The question is whether the table covers the workspace, and it does
        // not: four marker crates are in no layer, so any edge touching one would be reported
        // forward regardless of direction. They have no edges today; the assertion below is
        // what makes that a checked fact rather than a lucky one.
        let unlayered: Vec<&str> = census
            .crates
            .iter()
            .filter(|c| super::layer_of(&c.name).is_none())
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(
            unlayered,
            vec![
                "meridian-audio",
                "meridian-basalt",
                "meridian-isobar",
                "meridian-vegetation"
            ],
            "the set of unlayered crates changed; LAYERS must cover any crate that has edges"
        );
        for edge in &census.edges {
            assert!(
                !unlayered.contains(&edge.from.as_str()) && !unlayered.contains(&edge.to.as_str()),
                "edge {} -> {} touches an unlayered crate, so its direction is unverifiable",
                edge.from,
                edge.to
            );
        }
    }
}
