//! Pipeline + store integration tests, hermetic against a temp repo.

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::TempRepo;
use context_runtime::assemble::{AssembleParams, assemble};
use context_runtime::error::ContextError;
use context_runtime::pcc;
use context_runtime::store::BundleStore;

fn params(repo_root: PathBuf, max_age: u64) -> AssembleParams {
    AssembleParams {
        repo_id: "fixture".to_string(),
        repo_root,
        target_file: "pkg/target.py".to_string(),
        task_intent_id: "ti_codefix_fixture".to_string(),
        task_family: "code_fix".to_string(),
        task_version: "v1".to_string(),
        max_source_age_minutes: max_age,
        override_posture: pcc::OverridePosture::DisallowAll,
    }
}

#[test]
fn assembles_governed_bundle_with_code_native_payloads() {
    let repo = TempRepo::python();
    let now = SystemTime::now();
    let out = assemble(&params(repo.root.clone(), 100_000), now).expect("assembly should succeed");

    // Envelope is the real PCC manifest.
    assert!(out.manifest.context_bundle_id.starts_with("ctxb_"));
    assert!(!out.manifest.bundle_hash.is_empty());
    assert_eq!(out.manifest.replay_eligibility, pcc::ReplayEligibility::Eligible);
    assert!(!out.manifest.authority_conflict_flag);

    // Four roles gathered: target, adjacent, repo_truth, validation.
    assert_eq!(out.payload_refs.len(), 4);
    assert_eq!(out.manifest.source_inventory.len(), 4);
    assert_eq!(out.payloads.len(), 4);

    // Each payload is a validated code-native PCC contract.
    let classes: Vec<&str> = out.payloads.values().map(|p| p.artifact_class.as_str()).collect();
    assert!(classes.contains(&"key_file_packet"));
    assert!(classes.contains(&"repo_navigation_map"));
    assert!(classes.contains(&"validation_command_packet"));

    // The target payload is a KeyFilePacket pointing at the target file.
    let target_ref = format!("file://fixture/{}", repo.target_rel());
    let target_payload = out.payloads.get(&target_ref).expect("target payload present");
    assert_eq!(target_payload.role, "target");
    assert_eq!(target_payload.source_class, "active_scene");
    assert_eq!(target_payload.artifact_class, "key_file_packet");
    assert_eq!(target_payload.contract["file_path"], serde_json::json!("pkg/target.py"));
}

#[test]
fn assembly_is_deterministic_same_inputs_same_hash() {
    let repo = TempRepo::python();
    let now = SystemTime::now();
    let first = assemble(&params(repo.root.clone(), 100_000), now).expect("first");
    let second = assemble(&params(repo.root.clone(), 100_000), now).expect("second");

    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.manifest.bundle_hash, second.manifest.bundle_hash);
    assert_eq!(first.payload_refs, second.payload_refs);
}

#[test]
fn stale_source_fails_closed() {
    let repo = TempRepo::python();
    // Clock two hours ahead of the freshly-written files, ceiling 60 min.
    let now = SystemTime::now() + Duration::from_secs(2 * 3600);
    let err = assemble(&params(repo.root.clone(), 60), now).expect_err("stale must fail closed");
    match err {
        ContextError::AssemblyRejected(msg) => assert!(msg.contains("stale"), "got: {msg}"),
        other => panic!("expected AssemblyRejected(stale), got {other:?}"),
    }
}

#[test]
fn missing_target_fails_closed() {
    let repo = TempRepo::python();
    let mut p = params(repo.root.clone(), 100_000);
    p.target_file = "pkg/does_not_exist.py".to_string();
    let err = assemble(&p, SystemTime::now()).expect_err("missing target must fail closed");
    assert!(matches!(err, ContextError::TargetNotFound(_)), "got {err:?}");
}

#[test]
fn missing_repo_root_fails_closed() {
    let mut p = params(PathBuf::from("/nonexistent/repo/root/xyz"), 100_000);
    p.target_file = "anything.py".to_string();
    let err = assemble(&p, SystemTime::now()).expect_err("missing repo must fail closed");
    assert!(matches!(err, ContextError::RepoNotFound(_)), "got {err:?}");
}

#[test]
fn store_serves_admitted_refs_and_fails_closed_on_scope_escape() {
    let repo = TempRepo::python();
    let bundle = assemble(&params(repo.root.clone(), 100_000), SystemTime::now()).expect("assembly");
    let bundle_id = bundle.manifest.context_bundle_id.clone();
    let admitted = bundle.payload_refs[0].clone();

    let store = BundleStore::new();
    let stored_id = store.put(bundle);
    assert_eq!(stored_id, bundle_id);

    // Admitted ref resolves to its payload.
    let got = store.get_payload(&bundle_id, &admitted).expect("admitted ref resolves");
    assert_eq!(got.payload_ref, admitted);

    // Non-admitted ref → scope escape (CONFLICT).
    let err = store
        .get_payload(&bundle_id, "file://fixture/not/admitted.py")
        .expect_err("scope escape must fail closed");
    assert!(matches!(err, ContextError::RefNotAdmitted(_)), "got {err:?}");

    // Unknown bundle → not found.
    let err = store
        .get_payload("ctxb_unknown", &admitted)
        .expect_err("unknown bundle must fail closed");
    assert!(matches!(err, ContextError::BundleNotFound(_)), "got {err:?}");
}

/// If the real forgeHQ repo is present, assemble against a real source file and
/// confirm the governed bundle + code-native payloads come out clean.
#[test]
fn assembles_against_real_forgehq_if_present() {
    let forgehq = Path::new(env!("CARGO_MANIFEST_DIR")).join("../forgeHQ");
    let target = "app/services/context_bundle_service.py";
    if !forgehq.join(target).is_file() {
        eprintln!("skipping: forgeHQ not present at {}", forgehq.display());
        return;
    }
    let p = AssembleParams {
        repo_id: "forgehq".to_string(),
        repo_root: forgehq,
        target_file: target.to_string(),
        task_intent_id: "ti_codefix_forgehq".to_string(),
        task_family: "code_fix".to_string(),
        task_version: "v1".to_string(),
        max_source_age_minutes: 100_000_000,
        override_posture: pcc::OverridePosture::DisallowAll,
    };
    let out = assemble(&p, SystemTime::now()).expect("real forgeHQ assembly");
    assert!(out.payload_refs.len() >= 3, "expected target+doc+validation at least");
    let target_ref = format!("file://forgehq/{target}");
    let tp = out.payloads.get(&target_ref).expect("target payload");
    assert_eq!(tp.artifact_class, "key_file_packet");
    assert_eq!(tp.contract["file_path"], serde_json::json!(target));
}
