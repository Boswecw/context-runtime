//! Read-only repo discovery.
//!
//! For a target file, gather the four governed source roles the code-fix context
//! bundle needs, plus the structural `RepoFacts` the code-native PCC payloads
//! (RepoNavigationMap / ValidationCommandPacket) are built from. Nothing here
//! mutates the repo — it is the safe-by-design intake stage.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::{ContextError, Result};

/// The role a gathered source plays in the fix. Each role maps deterministically
/// to a PCC governance `SourceClass` (envelope) and a code-native contract class
/// (payload) — see `crate::assemble`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRole {
    /// The file under repair.
    Target,
    /// A neighboring file for surrounding context.
    Adjacent,
    /// Canonical repo truth (doc/system, README) — repo navigation map.
    RepoTruth,
    /// How a fix is validated — commands the verifier (pact) will run.
    Validation,
}

impl SourceRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceRole::Target => "target",
            SourceRole::Adjacent => "adjacent",
            SourceRole::RepoTruth => "repo_truth",
            SourceRole::Validation => "validation",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GatheredSource {
    pub role: SourceRole,
    /// Stable ref used as the bundle's payload_ref / forgeHQ context_item_ref.
    pub payload_ref: String,
    /// Repo-relative path, when the source is a real file.
    pub rel_path: Option<String>,
    /// The payload content (file text, or a synthesized body for validation).
    pub content: String,
    /// Minutes since the source was last modified, vs. the assembly clock.
    pub age_minutes: u64,
}

/// Deterministic structural facts about the repo, used to populate the
/// code-native RepoNavigationMap and ValidationCommandPacket contracts.
#[derive(Clone, Debug)]
pub struct RepoFacts {
    pub stack: &'static str,
    pub primary_directories: Vec<String>,
    pub entry_points: Vec<String>,
    pub canonical_docs: Vec<String>,
    pub build_test_commands: Vec<String>,
    pub validation_commands: Vec<String>,
    pub validation_execution_order: Vec<String>,
    pub validation_pass_conditions: Vec<String>,
    pub validation_env_requirements: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct GatherResult {
    pub repo_id: String,
    pub repo_root: PathBuf,
    pub target_rel: String,
    pub facts: RepoFacts,
    pub sources: Vec<GatheredSource>,
}

const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
];

fn file_ref(repo_id: &str, rel: &str) -> String {
    format!("file://{repo_id}/{rel}")
}

fn age_minutes(now: SystemTime, mtime: SystemTime) -> u64 {
    now.duration_since(mtime)
        .map(|d| d.as_secs() / 60)
        .unwrap_or(0)
}

fn read_text(path: &Path) -> Result<(String, SystemTime)> {
    let content = fs::read_to_string(path).map_err(|e| ContextError::Io(format!("{}: {e}", path.display())))?;
    let mtime = fs::metadata(path)
        .and_then(|m| m.modified())
        .map_err(|e| ContextError::Io(format!("{}: {e}", path.display())))?;
    Ok((content, mtime))
}

/// Gather the governed sources + repo facts for `target_rel` under `repo_root`.
pub fn gather(repo_id: &str, repo_root: &Path, target_rel: &str, now: SystemTime) -> Result<GatherResult> {
    if !repo_root.is_dir() {
        return Err(ContextError::RepoNotFound(repo_root.display().to_string()));
    }
    let target_rel = target_rel.trim_start_matches('/').to_string();
    if target_rel.is_empty() {
        return Err(ContextError::BadRequest("target_file is required".into()));
    }
    let target_abs = repo_root.join(&target_rel);
    if !target_abs.is_file() {
        return Err(ContextError::TargetNotFound(target_rel.clone()));
    }

    let mut sources: Vec<GatheredSource> = Vec::new();

    // (1) Target file — ActiveScene.
    let (target_content, target_mtime) = read_text(&target_abs)?;
    sources.push(GatheredSource {
        role: SourceRole::Target,
        payload_ref: file_ref(repo_id, &target_rel),
        rel_path: Some(target_rel.clone()),
        content: target_content,
        age_minutes: age_minutes(now, target_mtime),
    });

    // (2) Adjacent file — same directory, same extension, first by name, != target.
    if let Some((adj_rel, adj_content, adj_mtime)) = find_adjacent(repo_root, &target_rel)? {
        sources.push(GatheredSource {
            role: SourceRole::Adjacent,
            payload_ref: file_ref(repo_id, &adj_rel),
            rel_path: Some(adj_rel),
            content: adj_content,
            age_minutes: age_minutes(now, adj_mtime),
        });
    }

    // (3) Canonical repo-truth doc — AcceptedLoreRecord / RepoNavigationMap.
    let canonical_doc = find_canonical_doc(repo_root);
    if let Some(doc_rel) = &canonical_doc {
        let (doc_content, doc_mtime) = read_text(&repo_root.join(doc_rel))?;
        sources.push(GatheredSource {
            role: SourceRole::RepoTruth,
            payload_ref: format!("doc://{repo_id}/{doc_rel}"),
            rel_path: Some(doc_rel.clone()),
            content: doc_content,
            age_minutes: age_minutes(now, doc_mtime),
        });
    }

    // (4) Validation rule — AcceptedStyleRuleRecord / ValidationCommandPacket.
    let facts = detect_repo_facts(repo_root, &target_rel, canonical_doc.as_deref());
    let validation_body = facts.validation_commands.join("\n");
    sources.push(GatheredSource {
        role: SourceRole::Validation,
        payload_ref: format!("validation://{repo_id}/{}", facts.stack),
        rel_path: None,
        content: if validation_body.is_empty() {
            "no-op".to_string()
        } else {
            validation_body
        },
        // Synthesized from current detection — always fresh.
        age_minutes: 0,
    });

    Ok(GatherResult {
        repo_id: repo_id.to_string(),
        repo_root: repo_root.to_path_buf(),
        target_rel,
        facts,
        sources,
    })
}

fn find_adjacent(repo_root: &Path, target_rel: &str) -> Result<Option<(String, String, SystemTime)>> {
    let target_abs = repo_root.join(target_rel);
    let dir = match target_abs.parent() {
        Some(d) => d.to_path_buf(),
        None => return Ok(None),
    };
    let target_ext = target_abs.extension().and_then(|e| e.to_str()).unwrap_or("");
    let target_name = target_abs.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let mut candidates: Vec<String> = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if name == target_name {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != target_ext {
            continue;
        }
        candidates.push(name);
    }
    candidates.sort();
    let Some(name) = candidates.into_iter().next() else {
        return Ok(None);
    };
    let adj_abs = dir.join(&name);
    let rel = adj_abs
        .strip_prefix(repo_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or(name);
    let (content, mtime) = read_text(&adj_abs)?;
    Ok(Some((rel, content, mtime)))
}

fn find_canonical_doc(repo_root: &Path) -> Option<String> {
    // Preferred: doc/<CODE>SYSTEM.md (the canonical assembled doc).
    if let Ok(entries) = fs::read_dir(repo_root.join("doc")) {
        let mut hits: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
            .filter(|n| n.ends_with("SYSTEM.md"))
            .collect();
        hits.sort();
        if let Some(name) = hits.into_iter().next() {
            return Some(format!("doc/{name}"));
        }
    }
    // Fallbacks in priority order.
    for candidate in ["SYSTEM.md", "README.md", "readme.md", "Readme.md"] {
        if repo_root.join(candidate).is_file() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn top_level_dirs(repo_root: &Path) -> Vec<String> {
    let mut dirs: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(repo_root) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('.') || SKIP_DIRS.contains(&name) {
                    continue;
                }
                dirs.push(name.to_string());
            }
        }
    }
    dirs.sort();
    dirs.truncate(12);
    dirs
}

fn detect_repo_facts(repo_root: &Path, target_rel: &str, canonical_doc: Option<&str>) -> RepoFacts {
    let has = |p: &str| repo_root.join(p).exists();
    let target_is_py = target_rel.ends_with(".py");
    let target_is_rs = target_rel.ends_with(".rs");

    let (stack, mut entry_points, build_test_commands, validation): (
        &'static str,
        Vec<String>,
        Vec<String>,
        (Vec<String>, Vec<String>, Vec<String>, Vec<String>),
    ) = if has("Cargo.toml") || target_is_rs {
        (
            "rust",
            vec!["Cargo.toml".into()],
            vec!["cargo build".into(), "cargo test".into()],
            (
                vec!["cargo fmt --check".into(), "cargo clippy".into(), "cargo test".into()],
                vec!["fmt".into(), "clippy".into(), "test".into()],
                vec!["clippy reports no warnings".into(), "all tests pass".into()],
                vec!["rust toolchain (cargo) on PATH".into()],
            ),
        )
    } else if has("pyproject.toml") || has("requirements.txt") || has("setup.py") || target_is_py {
        (
            "python",
            Vec::new(),
            vec!["python -m pytest".into()],
            (
                vec!["ruff check .".into(), "python -m pytest".into()],
                vec!["lint".into(), "test".into()],
                vec!["ruff reports no errors".into(), "pytest exits 0".into()],
                vec!["python 3.11+ with project venv".into()],
            ),
        )
    } else if has("package.json") || has("tsconfig.json") {
        (
            "node",
            vec!["package.json".into()],
            vec!["npm test".into()],
            (
                vec!["npm run lint".into(), "npm test".into()],
                vec!["lint".into(), "test".into()],
                vec!["lint passes".into(), "tests pass".into()],
                vec!["node + npm on PATH".into()],
            ),
        )
    } else {
        (
            "generic",
            Vec::new(),
            vec!["<repo build command>".into()],
            (
                vec!["<repo verification command>".into()],
                vec!["verify".into()],
                vec!["verification command exits 0".into()],
                vec!["repo toolchain available".into()],
            ),
        )
    };

    // Add discovered entry points common across stacks.
    for ep in ["app", "src", "main.py", "app/__init__.py", "src/main.rs", "src/lib.rs"] {
        if has(ep) {
            entry_points.push(ep.to_string());
        }
    }
    entry_points.sort();
    entry_points.dedup();
    if entry_points.is_empty() {
        entry_points.push(target_rel.to_string());
    }

    let mut canonical_docs = Vec::new();
    if let Some(doc) = canonical_doc {
        canonical_docs.push(doc.to_string());
    } else {
        canonical_docs.push("(no canonical doc found)".to_string());
    }

    let mut primary_directories = top_level_dirs(repo_root);
    if primary_directories.is_empty() {
        primary_directories.push(".".to_string());
    }

    let (validation_commands, validation_execution_order, validation_pass_conditions, validation_env_requirements) =
        validation;

    RepoFacts {
        stack,
        primary_directories,
        entry_points,
        canonical_docs,
        build_test_commands,
        validation_commands,
        validation_execution_order,
        validation_pass_conditions,
        validation_env_requirements,
    }
}
