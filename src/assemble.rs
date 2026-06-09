//! The assemble pipeline: gather → map to the PCC governance envelope →
//! `precomputed_context_core::assemble_context` → build code-native payloads →
//! return a stored bundle.
//!
//! The envelope (admissibility / freshness / authority / deterministic
//! bundle_hash / replay eligibility) is PCC's `ContextBundleManifest`. The
//! payload behind each ref is a validated PCC code-native contract. Neither is
//! reinvented here — this module is the adapter that maps a code target onto
//! the real contracts.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use precomputed_context_core as pcc;

use crate::error::{ContextError, Result};
use crate::gather::{self, GatheredSource, SourceRole};
use crate::payload::{self, TypedPayload};

/// One stored payload behind a bundle ref.
#[derive(Clone, Debug, serde::Serialize)]
pub struct StoredPayload {
    pub payload_ref: String,
    /// Role in the fix (target/adjacent/repo_truth/validation).
    pub role: String,
    /// PCC governance class used in the envelope (the "scene/lore" vocabulary).
    pub source_class: String,
    /// PCC code-native contract class of the payload body.
    pub artifact_class: String,
    pub content_hash: String,
    pub content: String,
    /// The validated code-native PCC contract, serialized.
    pub contract: serde_json::Value,
}

/// A fully assembled, governed bundle ready to store and serve.
#[derive(Clone, Debug)]
pub struct AssembledBundle {
    pub repo_id: String,
    pub target_rel: String,
    pub manifest: pcc::ContextBundleManifest,
    /// Admitted refs == forgeHQ `context_item_refs`.
    pub payload_refs: Vec<String>,
    pub payloads: HashMap<String, StoredPayload>,
}

#[derive(Clone, Debug)]
pub struct AssembleParams {
    pub repo_id: String,
    pub repo_root: PathBuf,
    pub target_file: String,
    pub task_intent_id: String,
    pub task_family: String,
    pub task_version: String,
    pub max_source_age_minutes: u64,
    pub override_posture: pcc::OverridePosture,
}

/// Map a gathered source's role to the PCC governance `SourceClass`.
fn source_class(role: SourceRole) -> pcc::SourceClass {
    match role {
        SourceRole::Target => pcc::SourceClass::ActiveScene,
        SourceRole::Adjacent => pcc::SourceClass::AdjacentSceneSummaryOrClippedBody,
        SourceRole::RepoTruth => pcc::SourceClass::AcceptedLoreRecord,
        SourceRole::Validation => pcc::SourceClass::AcceptedStyleRuleRecord,
    }
}

fn to_source_input(source: &GatheredSource) -> pcc::SourceInput {
    pcc::SourceInput {
        payload_ref: source.payload_ref.clone(),
        source_class: source_class(source.role),
        age_minutes: source.age_minutes,
        authority_state: pcc::AuthorityState::Accepted,
        is_override: false,
    }
}

fn build_target_refs(sources: &[GatheredSource]) -> pcc::TargetRefs {
    let find = |role: SourceRole| {
        sources
            .iter()
            .find(|s| s.role == role)
            .map(|s| s.payload_ref.clone())
    };
    pcc::TargetRefs {
        active_scene_ref: find(SourceRole::Target),
        adjacent_scene_ref: find(SourceRole::Adjacent),
        accepted_lore_record_refs: sources
            .iter()
            .filter(|s| s.role == SourceRole::RepoTruth)
            .map(|s| s.payload_ref.clone())
            .collect(),
        accepted_style_rule_refs: sources
            .iter()
            .filter(|s| s.role == SourceRole::Validation)
            .map(|s| s.payload_ref.clone())
            .collect(),
    }
}

/// Run the full pipeline for one target file at clock `now`.
pub fn assemble(params: &AssembleParams, now: SystemTime) -> Result<AssembledBundle> {
    let gathered = gather::gather(&params.repo_id, &params.repo_root, &params.target_file, now)?;
    let now_rfc3339: String = DateTime::<Utc>::from(now).to_rfc3339();

    // Envelope: governed by the real PCC contract.
    let request = pcc::ContextAssemblyRequest {
        task_intent_id: params.task_intent_id.clone(),
        task_family: params.task_family.clone(),
        task_version: params.task_version.clone(),
        target_refs: build_target_refs(&gathered.sources),
        allowed_source_classes: vec![
            pcc::SourceClass::ActiveScene,
            pcc::SourceClass::AdjacentSceneSummaryOrClippedBody,
            pcc::SourceClass::AcceptedLoreRecord,
            pcc::SourceClass::AcceptedStyleRuleRecord,
        ],
        freshness_policy: pcc::FreshnessPolicy {
            max_source_age_minutes: params.max_source_age_minutes,
        },
        override_posture: params.override_posture.clone(),
        sources: gathered.sources.iter().map(to_source_input).collect(),
    };

    let output = pcc::assemble_context(&request)
        .map_err(|e| ContextError::AssemblyRejected(e.to_string()))?;

    // Payloads: code-native PCC contracts behind each admitted ref.
    let mut payloads: HashMap<String, StoredPayload> = HashMap::new();
    for source in &gathered.sources {
        let typed: TypedPayload =
            payload::build_payload(source, &gathered.facts, &params.repo_id, &now_rfc3339)?;
        payloads.insert(
            source.payload_ref.clone(),
            StoredPayload {
                payload_ref: source.payload_ref.clone(),
                role: source.role.as_str().to_string(),
                source_class: source_class(source.role).as_str().to_string(),
                artifact_class: typed.artifact_class().to_string(),
                content_hash: payload::content_hash(&source.content),
                content: source.content.clone(),
                contract: typed.to_json(),
            },
        );
    }

    Ok(AssembledBundle {
        repo_id: gathered.repo_id,
        target_rel: gathered.target_rel,
        manifest: output.manifest,
        payload_refs: output.payload_refs,
        payloads,
    })
}
