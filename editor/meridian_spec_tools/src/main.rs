use pulldown_cmark::{Event, Options, Parser, Tag};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

mod specoment;

const DOMAINS: &[&str] = &[
    "CORE", "GOV", "RUN", "RHI", "PEN", "UI", "EDT", "DAT", "PHY", "GAM", "PRJ", "AUD", "ISO",
    "BAS", "VEG", "PRC", "TOR", "DCC", "BLD", "VCS", "SYN", "XR", "NET", "MOD", "AGT", "SEC",
    "REL", "ANI", "NAV", "FWK", "TWO", "SHD", "MDL", "COL", "WRL", "INT", "PRM",
];
const VALID_STATUSES: &[&str] = &[
    "Active",
    "Adopted",
    "ArchitectureComplete",
    "Approved",
    "Blocked",
    "Closed",
    "ClosedUntilMS05",
    "ClosedUntilMS07",
    "Decided",
    "Deferred",
    "DeferredUntilXR",
    "Deprecated",
    "DefinitionOnly",
    "Draft",
    "Expired",
    "Fail",
    "Implemented",
    "ImplementedFoundation",
    "ImplementationReady",
    "Inconclusive",
    "Imported",
    "Mitigating",
    "Normative",
    "NotRun",
    "Occluded",
    "Open",
    "Partial",
    "Pass",
    "Planned",
    "Proposed",
    "Rejected",
    "Research",
    "ResearchReady",
    "Redacted",
    "Retired",
    "Scaffold",
    "StructuralSmoke",
    "Superseded",
    "Transitional",
    "Uncalibrated",
    "Unsupported",
    "UnsupportedCapability",
    "UnsupportedPlatform",
    "VerifiedCurrent",
    "Waived",
    "Stale",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Check,
    ValidateDocs,
    ValidateSchemas,
    ValidateMaturity,
    ValidateEvidence,
    ValidateWorkloads,
    ValidateAdrs,
    CheckV05Legacy,
    Census,
    ListUnmapped,
    Explain,
    Project { check: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Output {
    Human,
    Json,
    Github,
}

#[derive(Clone, Debug)]
struct Config {
    root: PathBuf,
    command: Command,
    explain_id: Option<String>,
    output: Output,
}

#[derive(Clone, Debug, Serialize)]
struct Issue {
    id: String,
    check: String,
    severity: String,
    path: String,
    message: String,
}

#[derive(Default)]
struct Context {
    markdown: Vec<Doc>,
    schemas: Vec<JsonDoc>,
    registries: Vec<JsonDoc>,
    records: Vec<Record>,
}

struct Doc {
    path: PathBuf,
    text: String,
}

struct JsonDoc {
    path: PathBuf,
    value: Value,
}

#[derive(Clone, Debug)]
struct Record {
    id: String,
    path: PathBuf,
    value: Value,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let config = parse_args(&args).unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2);
    });
    let issues = run(&config).unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2);
    });
    print_issues(&config, &issues);
    if issues.iter().any(|issue| issue.severity == "error") {
        std::process::exit(1);
    }
}

/// The v1 validator: the canonical specoment and its generated projections.
///
/// The v0.5 sub-validators remain reachable as `check-v05` until the follow-up package removes
/// them once `OD-010` is ruled. They no longer run by default because `PH-AUTH-004` deleted the
/// authority they policed; nine of their rules police concepts that survive into v1 with no
/// successor yet, and leaving the code in place keeps that ruling from being made by deletion.
fn check_v1(config: &Config, issues: &mut Vec<Issue>) -> Result<(), String> {
    let source_path = config.root.join("MERIDIAN_SPECOMENT.md");
    match fs::read_to_string(&source_path) {
        Ok(source) => {
            let digest = specoment::sha256::hex(source.as_bytes());
            let phases = specoment::phases::parse(&source);
            for edge in specoment::phases::unresolved_edges(&phases) {
                push(
                    issues,
                    "unresolved-phase-dependency",
                    "governance",
                    Path::new("MERIDIAN_SPECOMENT.md"),
                    format!(
                        "{} depends on {}, which no phase card declares",
                        edge.from, edge.to
                    ),
                );
            }
            for cycle in specoment::phases::cycles(&phases) {
                push(
                    issues,
                    "phase-cycle",
                    "governance",
                    Path::new("MERIDIAN_SPECOMENT.md"),
                    format!("phase dependency cycle: {}", cycle.join(" -> ")),
                );
            }
            for problem in specoment::accumulated::check_evidence_index(&config.root, &digest) {
                push(
                    issues,
                    "evidence-index",
                    "governance",
                    Path::new(&problem.path),
                    &problem.detail,
                );
            }
            // Class (c): derived from the source tree, so regeneration-and-compare is the
            // enforcement and the checkpoint is provenance. Deliberately not in emit::all():
            // a specoment-keyed stamp would report fresh after 37 crates changed. SD-013.
            for problem in specoment::census::format_reconciliation(&config.root) {
                push(
                    issues,
                    "format-enumeration",
                    "governance",
                    Path::new(".meridian/implementation/plans/PH-AUTH-005/KNOWN-FORMATS.md"),
                    &problem,
                );
            }
            match census_json(&config.root, &digest) {
                Ok(rendered) => {
                    let target = config.root.join(".meridian/implementation/census.json");
                    // Shape, against the checked-in schema. WP-V1-CENSUS-001 promised this
                    // and shipped without it, leaving the XOR invariant asserted only in
                    // prose — which is how a wholly zeroed census passed every gate.
                    for problem in specoment::census::schema_problems(&config.root, &rendered) {
                        push(issues, "census-schema", "governance", &target, &problem);
                    }
                    match fs::read_to_string(&target) {
                        Ok(existing) if existing == rendered => {}
                        Ok(_) => push(
                            issues,
                            "stale-census",
                            "governance",
                            &target,
                            "the census does not match the current source tree; regenerate it",
                        ),
                        Err(_) => push(
                            issues,
                            "missing-census",
                            "governance",
                            &target,
                            "no census; PH-AUTH-005 requires one",
                        ),
                    }
                }
                Err(error) => push(issues, "census-error", "governance", &config.root, &error),
            }
            for problem in
                specoment::accumulated::v05_declarations(&config.root, &tracked_files(&config.root))
            {
                push(
                    issues,
                    "v05-authority-declaration",
                    "governance",
                    Path::new(&problem.path),
                    &problem.detail,
                );
            }
            for problem in specoment::run(&config.root, true)? {
                push(
                    issues,
                    "stale-projection",
                    "governance",
                    &config.root,
                    &problem,
                );
            }
        }
        Err(error) => push(
            issues,
            "missing-authority",
            "governance",
            &source_path,
            format!("the canonical specoment is unreadable: {error}"),
        ),
    }
    Ok(())
}

/// Tracked files, read from Git's index without invoking Git.
///
/// Walking the filesystem would sweep `target/` and untracked local files; the closure row is
/// about what the repository *ships*.
fn tracked_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_excluded_context_path(entry.path()))
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(relative) = entry.path().strip_prefix(root) {
            let text = relative.to_string_lossy().to_string();
            if !text.starts_with('.') || text.starts_with(".codex") || text.starts_with(".github") {
                found.push(text);
            }
        }
    }
    found
}

/// Write the census to `.meridian/implementation/census.json`.
fn write_census(config: &Config) -> Result<(), String> {
    let source = fs::read_to_string(config.root.join("MERIDIAN_SPECOMENT.md"))
        .map_err(|error| format!("failed to read the specoment: {error}"))?;
    let digest = specoment::sha256::hex(source.as_bytes());
    let rendered = census_json(&config.root, &digest)?;
    let target = config.root.join(".meridian/implementation/census.json");
    fs::write(&target, &rendered)
        .map_err(|error| format!("failed to write {}: {error}", target.display()))?;
    println!("wrote {}", target.display());
    Ok(())
}

/// Render the census from `cargo metadata` plus a source walk.
///
/// `--format-version 1` is explicit because `rust-toolchain.toml` pins a floating `stable`,
/// and a routine toolchain bump changing the default format would otherwise break the gate on
/// a day nobody touched the repository.
fn census_json(root: &Path, specoment_sha256: &str) -> Result<String, String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cargo metadata failed: {error}"))?;
    if !output.status.success() {
        return Err("cargo metadata returned an error".to_string());
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata output is not JSON: {error}"))?;

    let mut census = specoment::census::measure(root);
    specoment::census::absorb_metadata(&mut census, root, &metadata);
    Ok(specoment::census::render(&census, root, specoment_sha256))
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    if args.is_empty() {
        return Err(usage());
    }
    let mut root = PathBuf::from(".");
    let mut output = Output::Human;
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                root = PathBuf::from(args.get(index).ok_or_else(usage)?);
            }
            "--output" | "-o" => {
                index += 1;
                output = parse_output(args.get(index).ok_or_else(usage)?)?;
            }
            value if value.starts_with("--output=") => {
                output = parse_output(value.trim_start_matches("--output="))?;
            }
            value => positional.push(value.to_owned()),
        }
        index += 1;
    }
    let command = match positional.first().map(String::as_str) {
        Some("check") => Command::Check,
        Some("check-v05") => Command::CheckV05Legacy,
        Some("census") => Command::Census,
        Some("validate") => match positional.get(1).map(String::as_str) {
            Some("docs") => Command::ValidateDocs,
            Some("schemas") => Command::ValidateSchemas,
            Some("maturity") => Command::ValidateMaturity,
            Some("evidence") => Command::ValidateEvidence,
            Some("workloads") => Command::ValidateWorkloads,
            Some("adrs") => Command::ValidateAdrs,
            _ => return Err(usage()),
        },
        Some("project") => Command::Project {
            check: args.iter().any(|value| value == "--check"),
        },
        Some("list-unmapped") => Command::ListUnmapped,
        Some("explain") => Command::Explain,
        _ => return Err(usage()),
    };
    let explain_id = if command == Command::Explain {
        Some(positional.get(1).ok_or_else(usage)?.to_owned())
    } else {
        None
    };
    Ok(Config {
        root,
        command,
        explain_id,
        output,
    })
}

fn parse_output(value: &str) -> Result<Output, String> {
    match value {
        "human" => Ok(Output::Human),
        "json" => Ok(Output::Json),
        "github" => Ok(Output::Github),
        _ => Err("--output requires human, json, or github".to_owned()),
    }
}

fn usage() -> String {
    "usage: meridian-spec [--root PATH] [--output human|json|github] check | validate docs|schemas|maturity|evidence|workloads|adrs | list-unmapped | explain <ID>".to_owned()
}

fn run(config: &Config) -> Result<Vec<Issue>, String> {
    let context = load_context(&config.root)?;
    let mut issues = Vec::new();
    match config.command {
        // `check` is the v1 validator. The v0.5 sub-validators remain reachable through
        // their own subcommands until the follow-up package removes them, but they no longer
        // run by default: PH-AUTH-004 deleted the authority they policed, so they would walk
        // an empty tree and report nothing. Nine of their rules police concepts that survive
        // into v1 with no successor yet — recorded as OD-010, not silently dropped.
        Command::Check => check_v1(config, &mut issues)?,
        Command::Census => write_census(config)?,
        Command::CheckV05Legacy => {
            // Retained pending OD-010, but not runnable. `SPEC-001` forbids old and new
            // authority competing, and a v0.5 validator judging the v1 tree is that
            // competition running backwards — it reports `unmapped-id` against VISION.md and
            // the creator-alpha example, which are v1 content it has no standing to judge.
            if !config.root.join("specs").is_dir() {
                return Err(
                    "the v0.5 authority was retired at PH-AUTH-004, so `check-v05` has nothing \
                     to check. The command is retained, not runnable, pending the OD-010 ruling \
                     on the nine v0.5 rules whose concept survives into v1 without a successor."
                        .to_string(),
                );
            }
            validate_docs(&config.root, &context, &mut issues);
            validate_schemas(&context, &mut issues);
            validate_maturity(&config.root, &context, &mut issues);
            validate_evidence(&context, &mut issues);
            validate_workloads(&context, &mut issues);
            validate_delivery_plan(&context, &mut issues);
            validate_work_package_graph(&context, &mut issues);
            validate_program_boundaries(&context, &mut issues);
            validate_marquee_policy(&context, &mut issues);
            validate_ui_contracts(&config.root, &context, &mut issues);
            validate_validation_projects(&context, &mut issues);
            validate_dependency_strategy(&context, &mut issues);
            validate_adrs(&config.root, &context, &mut issues);
            validate_cross_references(&context, &mut issues);
            list_unmapped(&context, &mut issues);
        }
        Command::ValidateDocs => validate_docs(&config.root, &context, &mut issues),
        Command::ValidateSchemas => validate_schemas(&context, &mut issues),
        Command::ValidateMaturity => {
            validate_maturity(&config.root, &context, &mut issues);
            validate_cross_references(&context, &mut issues);
        }
        Command::ValidateEvidence => validate_evidence(&context, &mut issues),
        Command::ValidateWorkloads => {
            validate_workloads(&context, &mut issues);
            validate_delivery_plan(&context, &mut issues);
            validate_work_package_graph(&context, &mut issues);
            validate_program_boundaries(&context, &mut issues);
            validate_marquee_policy(&context, &mut issues);
            validate_ui_contracts(&config.root, &context, &mut issues);
            validate_validation_projects(&context, &mut issues);
            validate_dependency_strategy(&context, &mut issues);
            validate_cross_references(&context, &mut issues);
        }
        Command::ValidateAdrs => validate_adrs(&config.root, &context, &mut issues),
        Command::Project { check } => {
            // Class (b) accumulated state is not emitted, so `project --check` cannot police
            // it by regeneration. Conformance runs here instead, which is what keeps the
            // evidence index honest between now and PH-AUTH-004's CI enforcement.
            if let Ok(source) = fs::read_to_string(config.root.join("MERIDIAN_SPECOMENT.md")) {
                let digest = specoment::sha256::hex(source.as_bytes());
                // A phase graph with a cycle cannot be executed in any order, and an edge to
                // a phase that does not exist cannot be satisfied. Both are hard defects.
                let phases = specoment::phases::parse(&source);
                for edge in specoment::phases::unresolved_edges(&phases) {
                    push(
                        &mut issues,
                        "unresolved-phase-dependency",
                        "governance",
                        Path::new("MERIDIAN_SPECOMENT.md"),
                        format!(
                            "{} depends on {}, which no phase card declares",
                            edge.from, edge.to
                        ),
                    );
                }
                for cycle in specoment::phases::cycles(&phases) {
                    push(
                        &mut issues,
                        "phase-cycle",
                        "governance",
                        Path::new("MERIDIAN_SPECOMENT.md"),
                        format!("phase dependency cycle: {}", cycle.join(" -> ")),
                    );
                }
                for problem in specoment::accumulated::check_evidence_index(&config.root, &digest) {
                    push(
                        &mut issues,
                        "evidence-index",
                        "governance",
                        Path::new(&problem.path),
                        &problem.detail,
                    );
                }
            }
            // Staleness must reach the exit code. Printing it and returning success is a
            // fail-open: CI goes green while the projections misrepresent the authority.
            for problem in specoment::run(&config.root, check)? {
                push(
                    &mut issues,
                    "stale-projection",
                    "governance",
                    &config.root,
                    &problem,
                );
            }
        }
        Command::ListUnmapped => list_unmapped(&context, &mut issues),
        Command::Explain => explain(config, &context, &mut issues),
    }
    Ok(issues)
}

fn load_context(root: &Path) -> Result<Context, String> {
    let mut context = Context::default();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_walk_context_entry(root, entry))
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let relative = relative(root, &path);
        if is_excluded_context_path(&relative) {
            continue;
        }
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("md") => {
                let text = fs::read_to_string(&path)
                    .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
                context.markdown.push(Doc { path, text });
            }
            Some("json") if is_under(&relative, "schemas/governance") => {
                context.schemas.push(read_json(&path)?);
            }
            Some("json") if is_under(&relative, "specs/registry") => {
                let document = read_json(&path)?;
                collect_records(&path, &document.value, &mut context.records);
                context.registries.push(document);
            }
            _ => {}
        }
    }
    Ok(context)
}

fn should_walk_context_entry(root: &Path, entry: &DirEntry) -> bool {
    entry.depth() == 0 || !is_excluded_context_path(&relative(root, entry.path()))
}

fn is_excluded_context_path(path: &Path) -> bool {
    is_under(path, "game")
        || is_under(path, "target")
        || is_under(path, ".git")
        || is_under(path, "website")
        || is_under(path, "editor/meridian_spec_tools/tests/fixtures")
        // Staged v1 authority. SPEC-001 requires that old and new authority never
        // compete; a v0.5 validator judging v1 content is that competition running
        // backwards. Retired with this tool at PH-AUTH-004.
        || is_under(path, ".meridian")
        || is_under(path, "MERIDIAN_SPECOMENT.md")
        // Generated v1 projections. These are derived from the root specoment and
        // legitimately cite drafting ledgers such as "spec-rewrite v0.22", which
        // `has_retired_reference` matches on the substring "v0.2". Retired with this
        // tool at PH-AUTH-004. WP-V1-RESET-002.
        || is_under(path, "governance")
}

fn read_json(path: &Path) -> Result<JsonDoc, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(JsonDoc {
        path: path.to_path_buf(),
        value,
    })
}

fn collect_records(path: &Path, value: &Value, records: &mut Vec<Record>) {
    match value {
        Value::Object(map) => {
            if let Some(id) = value.get("id").and_then(Value::as_str) {
                records.push(Record {
                    id: id.to_owned(),
                    path: path.to_path_buf(),
                    value: value.clone(),
                });
            }
            for child in map.values() {
                collect_records(path, child, records);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_records(path, item, records);
            }
        }
        _ => {}
    }
}

fn validate_docs(root: &Path, context: &Context, issues: &mut Vec<Issue>) {
    let mut declarations = BTreeMap::<String, PathBuf>::new();
    for doc in &context.markdown {
        for line in doc.text.lines().filter(|line| line.starts_with('#')) {
            let heading = line.trim_start_matches('#').trim();
            let ids = ids_in(heading);
            if is_declaration_heading(heading) {
                for id in &ids {
                    if let Some(first) = declarations.insert(id.clone(), doc.path.clone()) {
                        push(
                            issues,
                            "duplicate-id",
                            "docs",
                            &doc.path,
                            format!("{id} is also declared in {}", first.display()),
                        );
                    }
                }
            }
            if heading.starts_with("Requirement ")
                && !heading.starts_with("Requirement IDs")
                && ids.is_empty()
            {
                push(
                    issues,
                    "missing-id",
                    "docs",
                    &doc.path,
                    "requirement heading is missing a stable ID",
                );
            }
        }
        if doc.text.matches("```").count() % 2 != 0 || doc.text.matches("~~~").count() % 2 != 0 {
            push(
                issues,
                "bad-fence",
                "docs",
                &doc.path,
                "Markdown code fence is not balanced",
            );
        }
        validate_links(doc, issues);
        let relative = relative(root, &doc.path);
        let migration_record = is_under(&relative, "docs/migrations")
            || relative.file_name().and_then(|value| value.to_str())
                == Some("SPEC_MIGRATION_AND_CONTRADICTIONS.md");
        let historical_record =
            migration_record || is_under(&relative, "docs/architecture/decisions");
        if !historical_record && declares_stale_current_version(&doc.text) {
            push(
                issues,
                "stale-current-version",
                "docs",
                &doc.path,
                "current document header still declares v0.3 or v0.4 instead of v0.5",
            );
        }
        if !migration_record && has_retired_reference(&doc.text) {
            push(
                issues,
                "stale-phase-ref",
                "docs",
                &doc.path,
                "active document contains a retired phase, v0.2 authority, crate name, or deleted file reference",
            );
        }
    }
}

fn declares_stale_current_version(text: &str) -> bool {
    text.lines().take(16).any(|line| {
        let lower = line.trim().to_ascii_lowercase();
        ["0.3", "0.4"].iter().any(|version| {
            lower.starts_with(&format!("version {version}"))
                || lower.starts_with(&format!("version: {version}"))
                || lower.starts_with(&format!("status: version {version}"))
                || (lower.starts_with('#') && lower.ends_with(&format!(" v{version}")))
        })
    })
}

fn is_declaration_heading(heading: &str) -> bool {
    [
        "Requirement ",
        "Work package ",
        "Research gate ",
        "Evidence ",
        "Waiver ",
        "Decision ",
        "ADR-",
    ]
    .iter()
    .any(|prefix| heading.starts_with(prefix))
}

fn validate_links(doc: &Doc, issues: &mut Vec<Issue>) {
    for event in Parser::new_ext(&doc.text, Options::all()) {
        let Event::Start(Tag::Link { dest_url, .. }) = event else {
            continue;
        };
        let url = dest_url.as_ref();
        if url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("mailto:")
            || url.starts_with('#')
        {
            continue;
        }
        let clean = url.split('#').next().unwrap_or(url);
        if clean.is_empty() {
            continue;
        }
        let target = doc
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(clean);
        if !target.exists() {
            push(
                issues,
                "broken-link",
                "docs",
                &doc.path,
                format!("link target does not exist: {url}"),
            );
        }
    }
}

fn has_retired_reference(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("implementation_phases.md")
        || lower.contains("weather_environment_and_simulation_spec.md")
        || lower.contains("docs/adr/")
        || lower.contains("version 0.2")
        || lower.contains("v0.2")
        || lower.contains("meridian-weather")
        || lower.contains("meridian_weather")
        || lower.contains("meridian-terrain")
        || lower.contains("meridian_terrain")
    {
        return true;
    }
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '.')
        .any(|token| {
            token
                .strip_prefix("Phase")
                .is_some_and(|suffix| suffix.chars().next().is_some_and(|c| c.is_ascii_digit()))
                || token.strip_prefix('P').is_some_and(|suffix| {
                    !suffix.is_empty()
                        && suffix.chars().next().is_some_and(|c| c.is_ascii_digit())
                        && suffix.chars().all(|c| c.is_ascii_digit() || c == '.')
                })
        })
        || contains_phase_phrase(text)
}

fn contains_phase_phrase(text: &str) -> bool {
    text.match_indices("Phase ").any(|(index, _)| {
        text[index + 6..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
    })
}

fn validate_schemas(context: &Context, issues: &mut Vec<Issue>) {
    for schema in &context.schemas {
        if let Err(error) = jsonschema::validator_for(&schema.value) {
            push(
                issues,
                "bad-schema",
                "schemas",
                &schema.path,
                format!("schema does not compile: {error}"),
            );
        }
    }
    for registry in &context.registries {
        if !registry.value.is_object() || registry.value.get("records").is_none() {
            continue;
        }
        let Some(schema_path) = registry.value.get("$schema").and_then(Value::as_str) else {
            push(
                issues,
                "missing-schema",
                "schemas",
                &registry.path,
                "registry does not declare $schema",
            );
            continue;
        };
        let path = registry
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(schema_path);
        let Ok(schema_text) = fs::read_to_string(&path) else {
            push(
                issues,
                "missing-schema",
                "schemas",
                &registry.path,
                format!("declared schema does not exist: {}", path.display()),
            );
            continue;
        };
        let Ok(schema_value) = serde_json::from_str::<Value>(&schema_text) else {
            push(
                issues,
                "bad-schema-json",
                "schemas",
                &path,
                "declared schema is not valid JSON",
            );
            continue;
        };
        let Ok(validator) = jsonschema::validator_for(&schema_value) else {
            continue;
        };
        if let Err(error) = validator.validate(&registry.value) {
            push(
                issues,
                "registry-schema",
                "schemas",
                &registry.path,
                error.to_string(),
            );
        }
    }
}

fn validate_maturity(root: &Path, context: &Context, issues: &mut Vec<Issue>) {
    let today = today_utc();
    for record in &context.records {
        if let Some(status) = field_str(&record.value, "status") {
            if !VALID_STATUSES.contains(&status.as_str()) {
                push(
                    issues,
                    "bad-status",
                    "maturity",
                    &record.path,
                    format!("{} has unknown status {status}", record.id),
                );
            }
            if is_promotion_status(&status) && evidence_values(&record.value).is_empty() {
                push(
                    issues,
                    "false-promotion",
                    "maturity",
                    &record.path,
                    format!("{} is promoted to {status} without evidence", record.id),
                );
            }
        }
        for expiry in live_expiries(&record.value) {
            if expiry < today {
                push(
                    issues,
                    "expired-waiver",
                    "maturity",
                    &record.path,
                    format!("{} has expired waiver {expiry}", record.id),
                );
            }
        }
    }
    if let Some(registry) = registry_named(context, "subsystem-maturity.json") {
        let mut domains = BTreeSet::new();
        for value in records_array(&registry.value) {
            let Some(domain) = value.get("domain").and_then(Value::as_str) else {
                continue;
            };
            if !domains.insert(domain.to_owned()) {
                push(
                    issues,
                    "duplicate-maturity",
                    "maturity",
                    &registry.path,
                    format!("domain {domain} has multiple maturity records"),
                );
            }
            if let Some(spec) = value.get("spec").and_then(Value::as_str) {
                if !root.join(spec).exists() {
                    push(
                        issues,
                        "missing-maturity-spec",
                        "maturity",
                        &registry.path,
                        format!("domain {domain} maps missing spec {spec}"),
                    );
                }
            }
        }
        for domain in DOMAINS {
            if !domains.contains(*domain) {
                push(
                    issues,
                    "missing-maturity",
                    "maturity",
                    &registry.path,
                    format!("domain {domain} has no maturity record"),
                );
            }
        }
    }
}

fn today_utc() -> String {
    if let Ok(override_date) = env::var("MERIDIAN_SPEC_TODAY") {
        return override_date;
    }
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400;
    let days = i64::try_from(days).unwrap_or(i64::MAX);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn validate_evidence(context: &Context, issues: &mut Vec<Issue>) {
    let evidence_statuses: BTreeMap<String, String> = context
        .records
        .iter()
        .filter(|record| record.id.starts_with("EV-"))
        .filter_map(|record| {
            field_str(&record.value, "status").map(|status| (record.id.clone(), status))
        })
        .collect();
    for record in &context.records {
        let status = field_str(&record.value, "status").unwrap_or_default();
        let evidence = evidence_values(&record.value);
        if status == "Implemented" && evidence.is_empty() {
            push(
                issues,
                "implemented-without-evidence",
                "evidence",
                &record.path,
                format!("{} is Implemented without evidence", record.id),
            );
        }
        if is_promotion_status(&status) {
            for evidence_id in evidence.iter().filter(|id| id.starts_with("EV-")) {
                if evidence_statuses
                    .get(evidence_id)
                    .is_some_and(|value| value != "Pass")
                {
                    push(
                        issues,
                        "nonpassing-evidence",
                        "evidence",
                        &record.path,
                        format!("{} relies on {evidence_id}, which is not Pass", record.id),
                    );
                }
            }
        }
        let has_occluded = evidence.iter().any(|value| {
            let lower = value.to_ascii_lowercase();
            lower.contains("occluded") || lower.contains("structural")
        });
        let claims_visual = record
            .value
            .to_string()
            .to_ascii_lowercase()
            .contains("visible");
        if has_occluded && claims_visual {
            push(
                issues,
                "occluded-visual-evidence",
                "evidence",
                &record.path,
                format!(
                    "{} uses occluded/structural evidence for a visible claim",
                    record.id
                ),
            );
        }
    }
}

fn validate_workloads(context: &Context, issues: &mut Vec<Issue>) {
    for record in &context.records {
        if field_str(&record.value, "phase")
            .is_some_and(|phase| contains_phase_phrase(&phase) || phase.starts_with('P'))
        {
            push(
                issues,
                "stale-phase-ref",
                "workloads",
                &record.path,
                format!("{} contains a retired phase field", record.id),
            );
        }
    }
    let Some(registry) = registry_named(context, "workloads.json") else {
        return;
    };
    let required_alluvium_fields = [
        "alluvium_recipe_hashes",
        "alluvium_version",
        "determinism_level",
        "evaluation_mode",
        "provenance_manifest_hash",
    ];
    let report_fields: BTreeSet<_> = registry
        .value
        .get("required_report_fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for field in required_alluvium_fields {
        if !report_fields.contains(field) {
            push(
                issues,
                "missing-recipe-provenance-field",
                "workloads",
                &registry.path,
                format!("benchmark report contract is missing {field}"),
            );
        }
    }
    let ids: BTreeSet<_> = records_array(&registry.value)
        .filter_map(|record| record.get("id").and_then(Value::as_str))
        .collect();
    for number in 1..=16 {
        let id = format!("PEN-B{number:02}");
        if !ids.contains(id.as_str()) {
            push(
                issues,
                "missing-workload",
                "workloads",
                &registry.path,
                format!("{id} is missing"),
            );
        }
    }
    if let Some(ami) = records_array(&registry.value)
        .find(|record| record.get("id") == Some(&Value::String("PEN-B04".to_owned())))
    {
        for forbidden in [
            "logo_assets",
            "narrative_text",
            "proprietary_assets",
            "private_documents",
        ] {
            if ami.get(forbidden).is_some() {
                push(
                    issues,
                    "private-content-field",
                    "workloads",
                    &registry.path,
                    format!("PEN-B04 contains forbidden field {forbidden}"),
                );
            }
        }
    }
}

fn validate_delivery_plan(context: &Context, issues: &mut Vec<Issue>) {
    let Some(registry) = registry_named(context, "delivery-plan.json") else {
        return;
    };
    let expected: BTreeSet<_> = (0..=10).map(|number| format!("MS-{number:02}")).collect();
    let actual: BTreeSet<_> = records_array(&registry.value)
        .filter_map(|record| record.get("milestone").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect();

    for id in expected.difference(&actual) {
        push(
            issues,
            "missing-delivery-plan-milestone",
            "workloads",
            &registry.path,
            format!("{id} has no delivery-plan record"),
        );
    }
    for id in actual.difference(&expected) {
        push(
            issues,
            "unexpected-delivery-plan-milestone",
            "workloads",
            &registry.path,
            format!("{id} is outside MS-00 through MS-10"),
        );
    }

    let package_dependencies: BTreeMap<_, BTreeSet<_>> = context
        .records
        .iter()
        .filter(|record| record.id.starts_with("WP-"))
        .map(|record| {
            let dependencies = strings_for_keys(&record.value, &["depends_on"])
                .into_iter()
                .collect();
            (record.id.clone(), dependencies)
        })
        .collect();

    for milestone in records_array(&registry.value) {
        let milestone_id = milestone
            .get("milestone")
            .and_then(Value::as_str)
            .unwrap_or("unknown milestone");
        let path: Vec<_> = milestone
            .get("critical_path")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let referenced_packages = strings_for_keys(milestone, &["critical_path", "packages"]);
        for package in referenced_packages {
            if package.starts_with("WP-") && !package_dependencies.contains_key(&package) {
                push(
                    issues,
                    "orphan-work-package",
                    "workloads",
                    &registry.path,
                    format!("{milestone_id} references unmapped {package}"),
                );
            }
        }
        for pair in path.windows(2) {
            let previous = pair[0];
            let current = pair[1];
            if !package_dependencies
                .get(current)
                .is_some_and(|dependencies| dependencies.contains(previous))
            {
                push(
                    issues,
                    "broken-critical-path",
                    "workloads",
                    &registry.path,
                    format!(
                        "{milestone_id} orders {previous} before {current}, but {current} does not depend on {previous}"
                    ),
                );
            }
        }
    }
}

fn validate_work_package_graph(context: &Context, issues: &mut Vec<Issue>) {
    let mut unresolved: BTreeMap<String, BTreeSet<String>> = context
        .records
        .iter()
        .filter(|record| record.id.starts_with("WP-"))
        .map(|record| {
            let dependencies = strings_for_keys(&record.value, &["depends_on"])
                .into_iter()
                .filter(|value| value.starts_with("WP-"))
                .collect();
            (record.id.clone(), dependencies)
        })
        .collect();

    for (package, dependencies) in &unresolved {
        for dependency in dependencies {
            if !unresolved.contains_key(dependency) {
                push(
                    issues,
                    "invalid-package-dependency",
                    "workloads",
                    Path::new("specs/registry/work-packages.json"),
                    format!("{package} depends on unknown {dependency}"),
                );
            }
        }
    }

    loop {
        let ready: Vec<_> = unresolved
            .iter()
            .filter(|(_, dependencies)| {
                dependencies
                    .iter()
                    .all(|dependency| !unresolved.contains_key(dependency))
            })
            .map(|(id, _)| id.clone())
            .collect();
        if ready.is_empty() {
            break;
        }
        for id in ready {
            unresolved.remove(&id);
        }
    }

    if !unresolved.is_empty() {
        push(
            issues,
            "work-package-cycle",
            "workloads",
            Path::new("specs/registry/work-packages.json"),
            format!(
                "work-package dependencies contain a cycle involving {}",
                unresolved.keys().cloned().collect::<Vec<_>>().join(", ")
            ),
        );
    }
}

fn validate_program_boundaries(context: &Context, issues: &mut Vec<Issue>) {
    let Some(registry) = registry_named(context, "programs.json") else {
        return;
    };
    for program in records_array(&registry.value) {
        let id = program
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown program");
        if program.get("milestone").is_some()
            || program.get("milestones").is_some()
            || program.get("opens_after").is_some()
            || program.get("blocked_milestone").is_some()
            || program
                .get("may_satisfy_milestones")
                .and_then(Value::as_bool)
                .unwrap_or(true)
        {
            push(
                issues,
                "program-milestone-leak",
                "workloads",
                &registry.path,
                format!("{id} may not satisfy, block, or masquerade as MS-00 through MS-10"),
            );
        }
        if program
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(is_promotion_status)
        {
            push(
                issues,
                "program-implementation-promotion",
                "workloads",
                &registry.path,
                format!("{id} cannot promote implementation maturity"),
            );
        }
    }
    if let Some(delivery) = registry_named(context, "delivery-plan.json") {
        for reference in strings_for_keys(&delivery.value, &["critical_path", "packages"]) {
            if reference.starts_with("PRG-") {
                push(
                    issues,
                    "program-milestone-leak",
                    "workloads",
                    &delivery.path,
                    format!("delivery milestone references post-1.0 program {reference}"),
                );
            }
        }
    }
}

fn validate_marquee_policy(context: &Context, issues: &mut Vec<Issue>) {
    if !has_marquee_records(context) {
        return;
    }

    reject_premature_marquee_packages(context, issues);

    let Some(registry) = registry_named(context, "marquee-policy.json") else {
        push(
            issues,
            "missing-marquee-policy",
            "workloads",
            Path::new("specs/registry/marquee-policy.json"),
            "registered PRM authority requires the Marquee policy registry",
        );
        return;
    };
    let Some(policy) = records_array(&registry.value).next() else {
        push(
            issues,
            "missing-marquee-policy",
            "workloads",
            &registry.path,
            "Marquee policy registry has no record",
        );
        return;
    };

    validate_marquee_operational_boundary(policy, &registry.path, issues);
    validate_marquee_source_classes(policy, &registry.path, issues);
    validate_marquee_approval_invalidation(policy, &registry.path, issues);
    validate_marquee_ai_boundary(policy, &registry.path, issues);
}

fn has_marquee_records(context: &Context) -> bool {
    context.records.iter().any(|record| {
        record.id == "ADR-0025" || record.id == "PRG-PRM-001" || record.id.starts_with("REQ-PRM-")
    })
}

fn validate_marquee_operational_boundary(policy: &Value, path: &Path, issues: &mut Vec<Issue>) {
    if policy.get("capture_policy").and_then(Value::as_str) != Some("ManualImportOnly") {
        push(
            issues,
            "invalid-marquee-capture",
            "workloads",
            path,
            "Marquee must import manually supplied captures and cannot navigate the game",
        );
    }
    if policy.get("publishing_policy").and_then(Value::as_str) != Some("ExportOnly")
        || policy.get("website_generation").and_then(Value::as_bool) != Some(false)
        || policy.get("service_integrations").and_then(Value::as_bool) != Some(false)
    {
        push(
            issues,
            "invalid-marquee-publishing",
            "workloads",
            path,
            "Marquee is export-only and cannot publish, integrate service credentials, or generate websites",
        );
    }
    if policy.get("release_ready_approval").and_then(Value::as_str) != Some("ExplicitHuman") {
        push(
            issues,
            "invalid-marquee-approval",
            "workloads",
            path,
            "ReleaseReady export requires explicit human approval",
        );
    }
}

fn validate_marquee_source_classes(policy: &Value, path: &Path, issues: &mut Vec<Issue>) {
    let required_sources: BTreeSet<_> = [
        "GameplayCapture",
        "StagedEngineRender",
        "Composite",
        "ExternalArt",
        "LicensedStock",
        "LogoBrandAsset",
        "Audio",
        "AuthoredCopy",
    ]
    .into_iter()
    .collect();
    let actual_sources: BTreeSet<_> = policy
        .get("source_classes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if actual_sources != required_sources {
        push(
            issues,
            "incomplete-marquee-source-classes",
            "workloads",
            path,
            "Marquee must classify every supported manual source class",
        );
    }
}

fn validate_marquee_approval_invalidation(policy: &Value, path: &Path, issues: &mut Vec<Issue>) {
    let required_invalidators: BTreeSet<_> = [
        "Source",
        "SourceHash",
        "Claim",
        "BuildId",
        "Template",
        "Locale",
        "ExportProfile",
        "AiSuggestion",
        "ToolVersion",
        "Approval",
        "Font",
        "TransformRecipe",
    ]
    .into_iter()
    .collect();
    let actual_invalidators: BTreeSet<_> = policy
        .get("approval_invalidation")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if actual_invalidators != required_invalidators {
        push(
            issues,
            "incomplete-marquee-approval-invalidation",
            "workloads",
            path,
            "Marquee must invalidate ReleaseReady approvals for every normative input",
        );
    }
}

fn validate_marquee_ai_boundary(policy: &Value, path: &Path, issues: &mut Vec<Issue>) {
    let ai_boundary_is_valid = policy.get("ai").is_some_and(|ai| {
        ai.get("optional").and_then(Value::as_bool) == Some(true)
            && ai.get("cloud_opt_in").and_then(Value::as_bool) == Some(true)
            && ai.get("exact_data_disclosure").and_then(Value::as_bool) == Some(true)
            && ai.get("may_generate_text").and_then(Value::as_bool) == Some(true)
            && ai.get("may_analyze_media").and_then(Value::as_bool) == Some(true)
            && ai.get("may_generate_image").and_then(Value::as_bool) == Some(false)
            && ai.get("may_generate_video").and_then(Value::as_bool) == Some(false)
            && ai.get("may_generate_voice").and_then(Value::as_bool) == Some(false)
            && ai.get("may_generate_music").and_then(Value::as_bool) == Some(false)
            && ai.get("may_generate_sound").and_then(Value::as_bool) == Some(false)
            && ai.get("may_modify_media").and_then(Value::as_bool) == Some(false)
            && ai.get("suggestions_authoritative").and_then(Value::as_bool) == Some(false)
            && ai.get("human_review_required").and_then(Value::as_bool) == Some(true)
            && ai.get("zero_cost_when_disabled").and_then(Value::as_bool) == Some(true)
    });
    if !ai_boundary_is_valid {
        push(
            issues,
            "invalid-marquee-ai-boundary",
            "workloads",
            path,
            "Marquee AI is optional text/analysis suggestion only and cannot generate or modify media",
        );
    }
}

fn validate_ui_contracts(root: &Path, context: &Context, issues: &mut Vec<Issue>) {
    if !context
        .records
        .iter()
        .any(|record| record.id == "REQ-UI-002")
    {
        return;
    }

    validate_ui_token_contract(context, issues);
    validate_ui_component_contract(context, issues);
    validate_ui_workspace_contract(context, issues);
    validate_ui_package_sequence(context, issues);
    validate_ui_mockup_contract(root, issues);
}

fn validate_ui_token_contract(context: &Context, issues: &mut Vec<Issue>) {
    let Some(registry) = registry_named(context, "ui-design-tokens.json") else {
        push(
            issues,
            "missing-ui-design-tokens",
            "workloads",
            Path::new("specs/registry/ui-design-tokens.json"),
            "REQ-UI-002 requires the versioned Meridian UI design-token registry",
        );
        return;
    };

    let actual: BTreeMap<_, _> = records_array(&registry.value)
        .filter_map(|record| {
            Some((
                record.get("id")?.as_str()?.to_owned(),
                record.get("value")?.clone(),
            ))
        })
        .collect();
    let expected_strings = [
        ("token.color.background", "#090b0b"),
        ("token.color.surface", "#121515"),
        ("token.color.border", "#292d2c"),
        ("token.color.text-primary", "#e3e1d8"),
        ("token.color.text-secondary", "#929790"),
        ("token.color.muted", "#686e68"),
        ("token.color.destructive", "#a73732"),
        ("token.color.destructive-hover", "#c04b44"),
        ("token.color.positive", "#8d8961"),
        ("token.color.warning", "#c0964e"),
        ("token.font.interface", "Mona Sans"),
        ("token.font.display", "Hubot Sans"),
        ("token.font.mono", "JetBrains Mono"),
    ];
    for (id, value) in expected_strings {
        if actual.get(id).and_then(Value::as_str) != Some(value) {
            push(
                issues,
                "invalid-ui-token",
                "workloads",
                &registry.path,
                format!("{id} must be exactly {value}"),
            );
        }
    }

    let expected_numbers = [
        ("token.spacing.base", 4),
        ("token.spacing.dock-gutter", 8),
        ("token.border.standard", 1),
        ("token.radius.compact", 4),
        ("token.radius.control", 6),
        ("token.radius.panel", 10),
        ("token.radius.floating", 14),
        ("token.size.application-row", 44),
        ("token.size.workspace-row", 36),
        ("token.size.status-row", 24),
        ("token.size.activity-rail-collapsed", 44),
        ("token.size.activity-rail-expanded", 160),
        ("token.size.browser", 264),
        ("token.size.world-inspector", 344),
        ("token.size.bottom-shelf-peek", 32),
        ("token.size.bottom-shelf-expanded", 240),
        ("token.motion.state-min", 100),
        ("token.motion.state-max", 160),
    ];
    for (id, value) in expected_numbers {
        if actual.get(id).and_then(Value::as_u64) != Some(value) {
            push(
                issues,
                "invalid-ui-token",
                "workloads",
                &registry.path,
                format!("{id} must be exactly {value}"),
            );
        }
    }

    let policies = registry.value.get("policies");
    let policy_is_valid = policies.is_some_and(|policy| {
        policy.get("dense_content").and_then(Value::as_str) == Some("opaque")
            && policy.get("focus_geometry").and_then(Value::as_str)
                == Some("rectangular-edge-contrast-shape")
            && policy.get("decorative_ring").and_then(Value::as_bool) == Some(false)
            && policy.get("scene_tinted_chrome").and_then(Value::as_bool) == Some(false)
            && policy.get("blur").and_then(Value::as_str)
                == Some("floating-overlays-and-title-chrome-with-opaque-fallback")
    });
    if !policy_is_valid {
        push(
            issues,
            "invalid-ui-visual-policy",
            "workloads",
            &registry.path,
            "Meridian UI must keep dense content opaque, reject rings and scene-tinted chrome, and provide an opaque blur fallback",
        );
    }
}

fn validate_ui_component_contract(context: &Context, issues: &mut Vec<Issue>) {
    let Some(registry) = registry_named(context, "ui-components.json") else {
        push(
            issues,
            "missing-ui-components",
            "workloads",
            Path::new("specs/registry/ui-components.json"),
            "REQ-UI-002 requires the versioned component behavior registry",
        );
        return;
    };
    let required: BTreeSet<_> = [
        "component.button",
        "component.icon-button",
        "component.toggle",
        "component.text-field",
        "component.search-field",
        "component.combo-box",
        "component.menu",
        "component.menu-bar",
        "component.context-menu",
        "component.tooltip",
        "component.toast",
        "component.tabs",
        "component.tree",
        "component.table",
        "component.property-grid",
        "component.virtual-list",
        "component.timeline",
        "component.splitter",
        "component.progress",
        "component.command-palette",
        "component.graph-canvas",
        "component.viewport-canvas",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<_> = records_array(&registry.value)
        .filter_map(|record| record.get("id").and_then(Value::as_str))
        .collect();
    let missing: Vec<_> = required.difference(&actual).copied().collect();
    if !missing.is_empty() {
        push(
            issues,
            "incomplete-ui-components",
            "workloads",
            &registry.path,
            format!("component registry is missing {}", missing.join(", ")),
        );
    }
}

fn validate_ui_workspace_contract(context: &Context, issues: &mut Vec<Issue>) {
    let Some(registry) = registry_named(context, "ui-workspaces.json") else {
        push(
            issues,
            "missing-ui-workspaces",
            "workloads",
            Path::new("specs/registry/ui-workspaces.json"),
            "REQ-UI-002 requires the versioned workspace composition registry",
        );
        return;
    };
    let shell = registry.value.get("shell");
    let shell_is_valid = shell.is_some_and(|value| {
        value.get("application_row").and_then(Value::as_u64) == Some(44)
            && value.get("workspace_row").and_then(Value::as_u64) == Some(36)
            && value.get("status_row").and_then(Value::as_u64) == Some(24)
            && value.get("rows_merge").and_then(Value::as_bool) == Some(false)
            && value.get("narrow_strategy").and_then(Value::as_str)
                == Some("compress-utilities-and-controlled-workspace-overflow")
    });
    if !shell_is_valid {
        push(
            issues,
            "invalid-ui-shell",
            "workloads",
            &registry.path,
            "the permanent shell must keep exact 44, 36, and 24 pixel rows separate at every supported width",
        );
    }

    let required: BTreeSet<_> = [
        "workspace.hub",
        "workspace.world",
        "workspace.code",
        "workspace.modeler",
        "workspace.ui-authoring",
        "workspace.materials",
        "workspace.alluvium",
        "workspace.build",
        "workspace.profile",
        "workspace.settings",
        "workspace.recovery",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<_> = records_array(&registry.value)
        .filter_map(|record| record.get("id").and_then(Value::as_str))
        .collect();
    let missing: Vec<_> = required.difference(&actual).copied().collect();
    if !missing.is_empty() {
        push(
            issues,
            "incomplete-ui-workspaces",
            "workloads",
            &registry.path,
            format!("workspace registry is missing {}", missing.join(", ")),
        );
    }
}

fn validate_ui_package_sequence(context: &Context, issues: &mut Vec<Issue>) {
    let expected: [(&str, &[&str], &[&str]); 6] = [
        ("WP-UI-002", &["WP-UI-001"], &[]),
        ("WP-UI-003", &["WP-UI-002"], &[]),
        ("WP-UI-004", &["WP-UI-003"], &[]),
        ("WP-UI-005", &["WP-UI-004"], &["RG-UI-001"]),
        ("WP-EDT-002", &["WP-EDT-001", "WP-UI-005"], &[]),
        (
            "WP-EDT-003",
            &["WP-EDT-002", "WP-BLD-001", "WP-PRC-001", "WP-MDL-001"],
            &[],
        ),
    ];
    for (id, required_dependencies, required_gates) in expected {
        let Some(record) = context.records.iter().find(|record| record.id == id) else {
            push(
                issues,
                "missing-ui-package",
                "workloads",
                Path::new("specs/registry/work-packages.json"),
                format!("Meridian UI 1.0 requires {id}"),
            );
            continue;
        };
        let dependencies: BTreeSet<_> = strings_for_keys(&record.value, &["depends_on"])
            .into_iter()
            .collect();
        let gates: BTreeSet<_> = strings_for_keys(&record.value, &["gates"])
            .into_iter()
            .collect();
        if required_dependencies
            .iter()
            .any(|dependency| !dependencies.contains(*dependency))
            || required_gates.iter().any(|gate| !gates.contains(*gate))
        {
            push(
                issues,
                "invalid-ui-package-sequence",
                "workloads",
                &record.path,
                format!("{id} does not preserve the adopted dependency and research-gate sequence"),
            );
        }
    }
}

fn validate_ui_mockup_contract(root: &Path, issues: &mut Vec<Issue>) {
    let manifest_path = root.join("docs/production/ui-mockups/mockups.json");
    let Some(manifest) = load_ui_mockup_manifest(&manifest_path, issues) else {
        return;
    };
    validate_ui_mockup_manifest_identity(root, &manifest_path, &manifest, issues);

    let outputs: BTreeSet<_> = manifest
        .get("outputs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("file").and_then(Value::as_str))
        .collect();
    validate_ui_mockup_output_set(&manifest_path, &outputs, issues);
    validate_ui_mockup_files(root, &manifest_path, outputs, issues);
}

fn load_ui_mockup_manifest(path: &Path, issues: &mut Vec<Issue>) -> Option<Value> {
    let Ok(text) = fs::read_to_string(path) else {
        push(
            issues,
            "missing-ui-mockup-manifest",
            "workloads",
            path,
            "REQ-UI-002 requires the versioned 17-state mockup manifest",
        );
        return None;
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&text) else {
        push(
            issues,
            "invalid-ui-mockup-manifest",
            "workloads",
            path,
            "UI mockup manifest is not valid JSON",
        );
        return None;
    };
    Some(manifest)
}

fn validate_ui_mockup_manifest_identity(
    root: &Path,
    path: &Path,
    manifest: &Value,
    issues: &mut Vec<Issue>,
) {
    let generator_path = root.join("scripts/generate_ui_mockups.py");
    if manifest.get("schema").and_then(Value::as_str) != Some("meridian.ui-mockup-set/v1")
        || manifest.get("generator_version").and_then(Value::as_str) != Some("1.0.0")
        || !generator_path.is_file()
    {
        push(
            issues,
            "invalid-ui-mockup-manifest",
            "workloads",
            path,
            "mockup manifest must use v1, generator 1.0.0, and the checked-in generator",
        );
    }
}

fn validate_ui_mockup_output_set(path: &Path, outputs: &BTreeSet<&str>, issues: &mut Vec<Issue>) {
    let required: BTreeSet<_> = [
        "hub.svg",
        "world.svg",
        "code-contextual.svg",
        "code-full.svg",
        "modeler.svg",
        "ui-authoring.svg",
        "materials.svg",
        "alluvium.svg",
        "build.svg",
        "profile.svg",
        "settings.svg",
        "recovery.svg",
        "high-contrast.svg",
        "narrow.svg",
        "retina.svg",
        "1440p.svg",
        "ultrawide.svg",
    ]
    .into_iter()
    .collect();
    if *outputs != required {
        push(
            issues,
            "incomplete-ui-mockup-corpus",
            "workloads",
            path,
            "mockup manifest must contain the exact 17 required review states",
        );
    }
}

fn validate_ui_mockup_files(
    root: &Path,
    manifest_path: &Path,
    outputs: BTreeSet<&str>,
    issues: &mut Vec<Issue>,
) {
    for output in outputs {
        let path = manifest_path.parent().unwrap_or(root).join(output);
        let Ok(svg) = fs::read_to_string(&path) else {
            push(
                issues,
                "missing-ui-mockup",
                "workloads",
                &path,
                format!("mockup manifest output {output} does not exist"),
            );
            continue;
        };
        let generated = svg.contains("Generated by scripts/generate_ui_mockups.py")
            && svg.contains("meridian.ui-mockup-set/v1")
            && svg.contains("regenerate: python3 scripts/generate_ui_mockups.py");
        let shell = svg.contains("y=\"44\"")
            && svg.contains("height=\"36\"")
            && svg.contains("World")
            && svg.contains("Modeler")
            && svg.contains("Alluvium")
            && svg.contains("Profile");
        let prohibited = svg.to_ascii_lowercase();
        if !generated
            || !shell
            || prohibited.contains("#00ffff")
            || prohibited.contains("cyan")
            || prohibited.contains("meridian ring")
            || prohibited.contains("project meridian")
        {
            push(
                issues,
                "inconsistent-ui-mockup",
                "workloads",
                &path,
                "generated mockup must retain metadata, the two-row shell, public content, and the no-cyan/no-ring policy",
            );
        }
    }
}

fn reject_premature_marquee_packages(context: &Context, issues: &mut Vec<Issue>) {
    for record in &context.records {
        if record.id.starts_with("WP-PRM-") {
            push(
                issues,
                "premature-marquee-package",
                "workloads",
                &record.path,
                format!(
                    "{} cannot exist before a post-1.0 planning review",
                    record.id
                ),
            );
        }
    }
}

fn validate_validation_projects(context: &Context, issues: &mut Vec<Issue>) {
    let Some(registry) = registry_named(context, "validation-projects.json") else {
        return;
    };
    let expected: BTreeSet<_> = [
        "VAL-PRJ-001",
        "VAL-FWK-001",
        "VAL-FWK-002",
        "VAL-TWO-001",
        "VAL-UI-001",
        "VAL-RUN-001",
        "VAL-COL-001",
        "VAL-PRM-001",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<_> = records_array(&registry.value)
        .filter_map(|record| record.get("id").and_then(Value::as_str))
        .collect();
    for id in expected.difference(&actual) {
        push(
            issues,
            "missing-validation-project",
            "workloads",
            &registry.path,
            format!("{id} is missing"),
        );
    }
    for project in records_array(&registry.value) {
        let id = project
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown validation project");
        if project.get("status").and_then(Value::as_str) != Some("DefinitionOnly")
            || project.get("calibration").and_then(Value::as_str) != Some("Uncalibrated")
        {
            push(
                issues,
                "false-validation-project-promotion",
                "workloads",
                &registry.path,
                format!(
                    "{id} must remain DefinitionOnly and Uncalibrated without execution evidence"
                ),
            );
        }
    }
}

fn validate_dependency_strategy(context: &Context, issues: &mut Vec<Issue>) {
    let Some(registry) = registry_named(context, "dependency-strategy.json") else {
        return;
    };
    let expected: BTreeSet<_> = [
        "DEP-RHI-001",
        "DEP-SHD-001",
        "DEP-RUN-001",
        "DEP-RUN-002",
        "DEP-PHY-001",
        "DEP-BLD-001",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<_> = records_array(&registry.value)
        .filter_map(|record| record.get("id").and_then(Value::as_str))
        .collect();
    for id in expected.difference(&actual) {
        push(
            issues,
            "missing-dependency-strategy",
            "workloads",
            &registry.path,
            format!("{id} is missing"),
        );
    }

    for dependency in records_array(&registry.value) {
        let id = dependency
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown dependency");
        let category = dependency.get("category").and_then(Value::as_str);
        if !matches!(
            category,
            Some("InternalizeEarly" | "InternalizeEventually" | "WrapIndefinitelyUnlessNecessary")
        ) {
            push(
                issues,
                "invalid-dependency-strategy",
                "workloads",
                &registry.path,
                format!("{id} lacks an adopted dependency category"),
            );
        }
    }
}

fn validate_adrs(root: &Path, context: &Context, issues: &mut Vec<Issue>) {
    for record in &context.records {
        let required = field_bool(&record.value, "adr_required")
            || field_bool(&record.value, "requires_adr")
            || field_str(&record.value, "decision").is_some();
        if required {
            let adr = field_str(&record.value, "adr").unwrap_or_default();
            if adr.is_empty() || !root.join(&adr).exists() {
                push(
                    issues,
                    "missing-adr",
                    "adrs",
                    &record.path,
                    format!("{} requires an existing ADR mapping", record.id),
                );
            }
        }
    }
    if let Some(index) = registry_named(context, "adr-index.json") {
        for record in records_array(&index.value) {
            let Some(path) = record.get("path").and_then(Value::as_str) else {
                continue;
            };
            if !root.join(path).exists() {
                push(
                    issues,
                    "missing-adr",
                    "adrs",
                    &index.path,
                    format!("ADR index maps missing file {path}"),
                );
            }
        }
    }
}

fn validate_cross_references(context: &Context, issues: &mut Vec<Issue>) {
    let mut paths = BTreeMap::<String, PathBuf>::new();
    for record in &context.records {
        if let Some(first) = paths.insert(record.id.clone(), record.path.clone()) {
            push(
                issues,
                "duplicate-id",
                "registry",
                &record.path,
                format!("{} is also declared in {}", record.id, first.display()),
            );
        }
    }
    let ids: BTreeSet<_> = paths.keys().cloned().collect();
    for record in &context.records {
        for reference in references(&record.value) {
            if is_stable_id(&reference) && !ids.contains(&reference) {
                push(
                    issues,
                    "orphan-requirement",
                    "registry",
                    &record.path,
                    format!("{} references unmapped {reference}", record.id),
                );
            }
        }
    }
}

fn list_unmapped(context: &Context, issues: &mut Vec<Issue>) {
    let ids: BTreeSet<_> = context
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect();
    for doc in &context.markdown {
        for id in ids_in(&doc.text) {
            if id != "ADR-NNNN" && !ids.contains(id.as_str()) {
                push(
                    issues,
                    "unmapped-id",
                    "list-unmapped",
                    &doc.path,
                    format!("{id} appears in docs but not specs/registry"),
                );
            }
        }
    }
    let alluvium_is_registered = context.records.iter().any(|record| {
        record.id.starts_with("REQ-PRC-")
            || record.id.starts_with("WP-PRC-")
            || record.id.starts_with("RG-PRC-")
            || record.id.starts_with("RISK-PRC-")
            || record.id == "ADR-0017"
    });
    if alluvium_is_registered {
        let amendment = context.markdown.iter().find(|doc| {
            doc.path.file_name().and_then(|value| value.to_str())
                == Some("V0_4_ALLUVIUM_AMENDMENT.md")
        });
        match amendment {
            Some(doc) if doc.text.contains("Total unmapped rows: 0.") => {}
            Some(doc) => push(
                issues,
                "incomplete-migration-ledger",
                "list-unmapped",
                &doc.path,
                "v0.4 Alluvium migration ledger does not report zero total unmapped rows",
            ),
            None => push(
                issues,
                "missing-migration-ledger",
                "list-unmapped",
                Path::new("docs/migrations/V0_4_ALLUVIUM_AMENDMENT.md"),
                "v0.4 Alluvium migration ledger is missing",
            ),
        }
    }
    let v05_is_registered = context.records.iter().any(|record| {
        record.id == "ADR-0018"
            || record.id == "WP-GOV-004"
            || record.id.starts_with("REQ-ANI-")
            || record.id.starts_with("PRG-")
            || record.id.starts_with("VAL-")
    });
    if v05_is_registered {
        let amendment = context.markdown.iter().find(|doc| {
            doc.path.file_name().and_then(|value| value.to_str())
                == Some("V0_5_GENERAL_PURPOSE_PLATFORM_AMENDMENT.md")
        });
        match amendment {
            Some(doc) if doc.text.contains("Total unmapped rows: 0.") => {}
            Some(doc) => push(
                issues,
                "incomplete-migration-ledger",
                "list-unmapped",
                &doc.path,
                "v0.5 general-purpose migration ledger does not report zero total unmapped rows",
            ),
            None => push(
                issues,
                "missing-migration-ledger",
                "list-unmapped",
                Path::new("docs/migrations/V0_5_GENERAL_PURPOSE_PLATFORM_AMENDMENT.md"),
                "v0.5 general-purpose migration ledger is missing",
            ),
        }
    }
    validate_marquee_migration_ledger(context, issues);
}

fn validate_marquee_migration_ledger(context: &Context, issues: &mut Vec<Issue>) {
    if !has_marquee_records(context) {
        return;
    }
    let amendment = context.markdown.iter().find(|doc| {
        doc.path.file_name().and_then(|value| value.to_str()) == Some("V0_5_MARQUEE_AMENDMENT.md")
    });
    match amendment {
        Some(doc) if doc.text.contains("Total unmapped rows: 0.") => {}
        Some(doc) => push(
            issues,
            "incomplete-migration-ledger",
            "list-unmapped",
            &doc.path,
            "v0.5 Marquee amendment ledger does not report zero total unmapped rows",
        ),
        None => push(
            issues,
            "missing-migration-ledger",
            "list-unmapped",
            Path::new("docs/migrations/V0_5_MARQUEE_AMENDMENT.md"),
            "v0.5 Marquee amendment ledger is missing",
        ),
    }
}

fn explain(config: &Config, context: &Context, issues: &mut Vec<Issue>) {
    let id = config.explain_id.as_deref().unwrap_or_default();
    if let Some(record) = context.records.iter().find(|record| record.id == id) {
        push_with_severity(
            issues,
            "record",
            "explain",
            "info",
            &record.path,
            serde_json::to_string_pretty(&record.value).unwrap_or_else(|_| record.id.clone()),
        );
    } else {
        push(
            issues,
            "unknown-id",
            "explain",
            Path::new("specs/registry"),
            format!("{id} is not mapped in specs/registry"),
        );
    }
}

fn registry_named<'a>(context: &'a Context, name: &str) -> Option<&'a JsonDoc> {
    context
        .registries
        .iter()
        .find(|registry| registry.path.file_name().and_then(|value| value.to_str()) == Some(name))
}

fn records_array(value: &Value) -> impl Iterator<Item = &Value> {
    value
        .get("records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn ids_in(text: &str) -> Vec<String> {
    text.split(|character: char| {
        !(character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-')
    })
    .filter(|candidate| is_stable_id(candidate))
    .map(ToOwned::to_owned)
    .collect()
}

fn is_stable_id(candidate: &str) -> bool {
    if candidate == "ADR-NNNN" {
        return true;
    }
    let parts: Vec<_> = candidate.split('-').collect();
    match parts.as_slice() {
        ["MS", number] => number.parse::<u8>().is_ok_and(|value| value <= 10),
        ["PEN", workload] => workload
            .strip_prefix('B')
            .and_then(|value| value.parse::<u8>().ok())
            .is_some_and(|value| (1..=16).contains(&value)),
        ["ADR", number] => number.len() == 4 && number.chars().all(|c| c.is_ascii_digit()),
        ["REQ" | "WP" | "RG" | "WVR" | "RISK" | "SRC" | "PRG" | "VAL" | "DEP", domain, number] => {
            !domain.is_empty()
                && domain.chars().all(|c| c.is_ascii_uppercase())
                && number.len() == 3
                && number.chars().all(|c| c.is_ascii_digit())
        }
        ["EV" | "REV", domain, date, number] => {
            !domain.is_empty()
                && domain.chars().all(|c| c.is_ascii_uppercase())
                && date.len() == 8
                && date.chars().all(|c| c.is_ascii_digit())
                && number.len() == 3
                && number.chars().all(|c| c.is_ascii_digit())
        }
        ["GOV", number] => number.len() >= 3 && number.chars().all(|c| c.is_ascii_digit()),
        ["REQ", number] => number.len() >= 3 && number.chars().all(|c| c.is_ascii_digit()),
        _ => false,
    }
}

fn is_promotion_status(status: &str) -> bool {
    matches!(
        status,
        "Implemented" | "ImplementedFoundation" | "StructuralSmoke"
    )
}

fn evidence_values(value: &Value) -> Vec<String> {
    strings_for_keys(
        value,
        &[
            "evidence",
            "evidence_id",
            "evidence_ids",
            "captures",
            "tests",
        ],
    )
}

fn references(value: &Value) -> Vec<String> {
    strings_for_keys(
        value,
        &[
            "requirements",
            "requirement_ids",
            "work_packages",
            "programs",
            "program",
            "depends_on",
            "gates",
            "critical_path",
            "packages",
            "requires",
            "blocks",
            "evidence",
            "required_workloads",
            "milestones",
            "opens_after",
            "blocked_milestone",
            "remediation_package",
            "proving_requirements",
        ],
    )
}

/// Statuses that retire a waiver. A retired waiver grants nothing, so its
/// expiry date is historical fact rather than a live governance risk.
const RETIRED_WAIVER_STATUSES: &[&str] = &["Closed", "Retired", "Superseded", "Rejected"];

/// Collect `expiry`/`expires` values that still govern something.
///
/// Any object carrying a retired status is skipped whole: neither its own
/// expiry nor any nested one is reported. Waivers that remain open are
/// reported exactly as before.
fn live_expiries(value: &Value) -> Vec<String> {
    let mut values = Vec::new();
    collect_live_expiries(value, &mut values);
    values
}

fn collect_live_expiries(value: &Value, values: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(status) = map.get("status").and_then(Value::as_str) {
                if RETIRED_WAIVER_STATUSES.contains(&status) {
                    return;
                }
            }
            for (key, child) in map {
                if key == "expiry" || key == "expires" {
                    collect_strings(child, values);
                } else {
                    collect_live_expiries(child, values);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_live_expiries(item, values);
            }
        }
        _ => {}
    }
}

fn strings_for_keys(value: &Value, keys: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    collect_strings_for_keys(value, keys, &mut values);
    values
}

fn collect_strings_for_keys(value: &Value, keys: &[&str], values: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if keys.contains(&key.as_str()) {
                    collect_strings(child, values);
                } else {
                    collect_strings_for_keys(child, keys, values);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_strings_for_keys(item, keys, values);
            }
        }
        _ => {}
    }
}

fn collect_strings(value: &Value, values: &mut Vec<String>) {
    match value {
        Value::String(text) => values.push(text.clone()),
        Value::Array(items) => {
            for item in items {
                collect_strings(item, values);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_strings(item, values);
            }
        }
        _ => {}
    }
}

fn field_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn field_bool(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn is_under(path: &Path, prefix: &str) -> bool {
    let expected: Vec<_> = prefix.split('/').collect();
    let actual: Vec<_> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    actual.starts_with(&expected)
}

fn push(issues: &mut Vec<Issue>, id: &str, check: &str, path: &Path, message: impl Into<String>) {
    push_with_severity(issues, id, check, "error", path, message);
}

fn push_with_severity(
    issues: &mut Vec<Issue>,
    id: &str,
    check: &str,
    severity: &str,
    path: &Path,
    message: impl Into<String>,
) {
    issues.push(Issue {
        id: id.to_owned(),
        check: check.to_owned(),
        severity: severity.to_owned(),
        path: path.display().to_string(),
        message: message.into(),
    });
}

fn print_issues(config: &Config, issues: &[Issue]) {
    match config.output {
        Output::Human => {
            if issues.is_empty() {
                println!("ok");
            } else {
                for issue in issues {
                    println!(
                        "{} [{}:{}] {}: {}",
                        issue.severity, issue.check, issue.id, issue.path, issue.message
                    );
                }
            }
        }
        Output::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "issues": issues })).expect("serialize issues")
        ),
        Output::Github => {
            for issue in issues {
                let level = if issue.severity == "error" {
                    "error"
                } else {
                    "notice"
                };
                println!(
                    "::{level} file={}::{}:{} {}",
                    issue.path, issue.check, issue.id, issue.message
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn civil_dates_match_unix_epoch_and_a_leap_day() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }
}
