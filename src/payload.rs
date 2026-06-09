//! Code-native payload bodies.
//!
//! Option-3 of the C1 design: the governed *envelope* is PCC's
//! `ContextBundleManifest` (refs + freshness + authority + hash + replay), but
//! the *content* behind each ref is a real, `.validate()`-passing PCC
//! code-native contract — `KeyFilePacketContract`, `RepoNavigationMapContract`,
//! or `ValidationCommandPacketContract`. Nothing here invents a contract shape;
//! it constructs the ones precomputed-context-core already proves.

use precomputed_context_core as pcc;
use sha2::{Digest, Sha256};

use crate::error::{ContextError, Result};
use crate::gather::{GatheredSource, RepoFacts, SourceRole};

/// A built, validated code-native payload behind one bundle ref.
#[derive(Clone, Debug)]
pub enum TypedPayload {
    KeyFile(pcc::KeyFilePacketContract),
    RepoNav(pcc::RepoNavigationMapContract),
    Validation(pcc::ValidationCommandPacketContract),
}

impl TypedPayload {
    pub fn artifact_class(&self) -> &'static str {
        match self {
            TypedPayload::KeyFile(_) => "key_file_packet",
            TypedPayload::RepoNav(_) => "repo_navigation_map",
            TypedPayload::Validation(_) => "validation_command_packet",
        }
    }

    /// Run the contract's own fail-closed validation.
    pub fn validate(&self) -> std::result::Result<(), String> {
        match self {
            TypedPayload::KeyFile(c) => c.validate(),
            TypedPayload::RepoNav(c) => c.validate(),
            TypedPayload::Validation(c) => c.validate(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            TypedPayload::KeyFile(c) => serde_json::to_value(c).unwrap_or(serde_json::Value::Null),
            TypedPayload::RepoNav(c) => serde_json::to_value(c).unwrap_or(serde_json::Value::Null),
            TypedPayload::Validation(c) => serde_json::to_value(c).unwrap_or(serde_json::Value::Null),
        }
    }
}

pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn artifact_id(payload_ref: &str) -> String {
    let h = content_hash(payload_ref);
    format!("art_{}", &h[..16])
}

/// Build a fully-valid `ArtifactRecord` base for a freshly-gathered, accepted
/// source. Approved + Fresh + Passed resolves to `Admissible` under PCC's
/// canonical admissibility algebra, which `validate_artifact_state` enforces.
#[allow(clippy::too_many_arguments)]
fn base_record(
    artifact_class: pcc::ArtifactClass,
    repo_id: &str,
    payload_ref: &str,
    content: &str,
    title: String,
    operational_purpose: String,
    summary_block: String,
    authority_level: pcc::AuthorityLevel,
    now_rfc3339: &str,
) -> pcc::ArtifactRecord {
    pcc::ArtifactRecord {
        schema_version: "1.0.0".to_string(),
        artifact_id: artifact_id(payload_ref),
        artifact_class,
        repo_id: repo_id.to_string(),
        title,
        operational_purpose,
        summary_block,
        source_refs: vec![payload_ref.to_string()],
        source_ref_hashes: vec![content_hash(content)],
        authority_level,
        lifecycle_state: pcc::LifecycleState::Approved,
        freshness_state: pcc::FreshnessState::Fresh,
        critic_status: pcc::CriticStatus::Passed,
        admissibility_state: pcc::AdmissibilityState::Admissible,
        related_artifact_refs: Vec::new(),
        supersedes_artifact_id: None,
        protocol_refs: Vec::new(),
        created_at: now_rfc3339.to_string(),
        last_validated_at: now_rfc3339.to_string(),
        producer_identity: "context-runtime".to_string(),
        sensitivity_classification: pcc::SensitivityClassification::InternalGeneral,
    }
}

/// Build the code-native payload for a gathered source and validate it.
/// Fails closed (`PayloadInvalid`) if the contract rejects.
pub fn build_payload(
    source: &GatheredSource,
    facts: &RepoFacts,
    repo_id: &str,
    now_rfc3339: &str,
) -> Result<TypedPayload> {
    let payload = match source.role {
        SourceRole::Target | SourceRole::Adjacent => {
            let file_path = source
                .rel_path
                .clone()
                .unwrap_or_else(|| source.payload_ref.clone());
            let (why, authority) = match source.role {
                SourceRole::Target => (
                    "Primary target of the code-fix run.".to_string(),
                    pcc::AuthorityLevel::StrongDerived,
                ),
                _ => (
                    "Neighboring file providing surrounding context for the target.".to_string(),
                    pcc::AuthorityLevel::WeakDerived,
                ),
            };
            let base = base_record(
                pcc::ArtifactClass::KeyFilePacket,
                repo_id,
                &source.payload_ref,
                &source.content,
                format!("Key file: {file_path}"),
                "Provide the file content and edit guardrails for a bounded code fix.".to_string(),
                first_lines(&source.content, 3),
                authority,
                now_rfc3339,
            );
            TypedPayload::KeyFile(pcc::KeyFilePacketContract {
                base,
                file_path,
                why_it_matters: why,
                dependent_surfaces: top_n(&facts.entry_points, 4),
                edit_cautions: vec![
                    "Non-authoritative context bundle; verify the fix with the validation packet."
                        .to_string(),
                    "Fail closed on scope escape — only admitted refs may be modified.".to_string(),
                ],
                read_before_edit_refs: facts.canonical_docs.clone(),
            })
        }
        SourceRole::RepoTruth => {
            let base = base_record(
                pcc::ArtifactClass::RepoNavigationMap,
                repo_id,
                &source.payload_ref,
                &source.content,
                "Repo navigation map".to_string(),
                "Orient the fix within the repo's structure, entry points, and verification."
                    .to_string(),
                first_lines(&source.content, 3),
                pcc::AuthorityLevel::Canonical,
                now_rfc3339,
            );
            TypedPayload::RepoNav(pcc::RepoNavigationMapContract {
                base,
                primary_directories: facts.primary_directories.clone(),
                entry_points: facts.entry_points.clone(),
                canonical_docs: facts.canonical_docs.clone(),
                build_test_commands: facts.build_test_commands.clone(),
            })
        }
        SourceRole::Validation => {
            let base = base_record(
                pcc::ArtifactClass::ValidationCommandPacket,
                repo_id,
                &source.payload_ref,
                &source.content,
                format!("Validation commands ({})", facts.stack),
                "Define how a proposed fix is verified before it can be trusted.".to_string(),
                facts.validation_commands.join("; "),
                pcc::AuthorityLevel::StrongDerived,
                now_rfc3339,
            );
            TypedPayload::Validation(pcc::ValidationCommandPacketContract {
                base,
                commands: facts.validation_commands.clone(),
                execution_order: facts.validation_execution_order.clone(),
                expected_pass_conditions: facts.validation_pass_conditions.clone(),
                environment_requirements: facts.validation_env_requirements.clone(),
            })
        }
    };

    payload
        .validate()
        .map_err(|e| ContextError::PayloadInvalid(format!("{} payload: {e}", source.role.as_str())))?;
    Ok(payload)
}

fn first_lines(content: &str, n: usize) -> String {
    let joined: Vec<&str> = content.lines().take(n).collect();
    let summary = joined.join(" ");
    let summary = summary.trim();
    if summary.is_empty() {
        format!("({} bytes of content)", content.len())
    } else {
        summary.chars().take(280).collect()
    }
}

fn top_n(items: &[String], n: usize) -> Vec<String> {
    let out: Vec<String> = items.iter().take(n).cloned().collect();
    if out.is_empty() {
        vec!["(none detected)".to_string()]
    } else {
        out
    }
}
