use crate::pg_rpg::{NamedTextAsset, VirtualRhaiWorkspace};
use crate::rhai_session::RhaiSession;
use pystral_core::history::HistoryManager;
use serde::Serialize;

// This is intentionally authored outside Rust. The test only supplies the
// normal session boundary; fixture construction, assertions, and deterministic
// progression belong to the same Rhai API used by pg_rpg.
const CORE_PROPERTIES: &str = include_str!("../test-fixtures/pg_rpg_core_properties.rhai");

#[derive(Debug, Serialize)]
struct AuthoredCaseResult {
    case_name: String,
    source: &'static str,
    seed: u64,
    status: &'static str,
    details: String,
}

#[test]
fn authored_core_properties_use_the_runtime_rhai_harness() {
    let mut session = RhaiSession::new(
        CORE_PROPERTIES,
        HistoryManager::new(),
        String::new(),
        Vec::new(),
        0,
    )
    .unwrap_or_else(|error| panic!("authored pg_rpg core properties failed: {error}"));
    let results = [
        "unit_health_property",
        "deterministic_boundary_property",
        "npc_decision_provenance_property",
    ]
    .into_iter()
    .map(|case_name| match session.run_named_case_json(case_name) {
        Ok(details) => AuthoredCaseResult {
            case_name: case_name.to_string(),
            source: "crates/runtime/test-fixtures/pg_rpg_core_properties.rhai",
            seed: 42,
            status: "passed",
            details,
        },
        Err(error) => panic!("authored case {case_name} failed: {error}"),
    })
    .collect::<Vec<_>>();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|result| result.status == "passed"));
    let _structured_report = serde_json::to_string(&results).unwrap();
}

#[test]
fn virtual_workspace_sessions_are_isolated_and_replayable() {
    let workspace = VirtualRhaiWorkspace::new(
        "mod/main.rhai",
        vec![NamedTextAsset {
            path: "mod/main.rhai".into(),
            contents: "fn authored_case() { 42 }".into(),
        }],
    )
    .unwrap();
    let mut first = RhaiSession::from_virtual_workspace(
        &workspace,
        HistoryManager::new(),
        String::new(),
        Vec::new(),
        0,
        42,
    )
    .unwrap();
    let second = RhaiSession::from_virtual_workspace(
        &workspace,
        HistoryManager::new(),
        String::new(),
        Vec::new(),
        0,
        42,
    )
    .unwrap();
    assert_eq!(first.run_named_case_json("authored_case").unwrap(), "42");
    assert_eq!(first.replay_header(), second.replay_header());
    assert_eq!(first.replay_header().unwrap().entrypoint, "mod/main.rhai");
}

#[test]
fn authored_case_protocol_runs_through_typed_runtime_request() {
    let workspace = VirtualRhaiWorkspace::new(
        "main.rhai",
        vec![NamedTextAsset {
            path: "main.rhai".into(),
            contents: r#"fn authored_case() { #{ status: "passed", value: 7 } }"#.into(),
        }],
    )
    .unwrap();
    let (response, logs) = crate::Runtime::new().process_request(crate::RuntimeRequest::RunRhaiCase {
        workspace,
        case_name: "authored_case".into(),
        seed: 99,
    });
    assert!(logs.is_empty());
    match response {
        crate::RuntimeResponse::RhaiCaseResult {
            case_name,
            seed,
            replay_header,
            details,
        } => {
            assert_eq!(case_name, "authored_case");
            assert_eq!(seed, 99);
            assert_eq!(replay_header.unwrap().seed, 99);
            assert!(details.contains("\"value\":7"));
        }
        other => panic!("unexpected response: {other:?}"),
    }
}
