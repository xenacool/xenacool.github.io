use crate::rhai_session::RhaiSession;
use pystral_core::history::HistoryManager;

// This is intentionally authored outside Rust. The test only supplies the
// normal session boundary; fixture construction, assertions, and deterministic
// progression belong to the same Rhai API used by pg_rpg.
const CORE_PROPERTIES: &str = include_str!("../test-fixtures/pg_rpg_core_properties.rhai");

#[test]
fn authored_core_properties_use_the_runtime_rhai_harness() {
    RhaiSession::new(
        CORE_PROPERTIES,
        HistoryManager::new(),
        String::new(),
        Vec::new(),
        0,
    )
    .unwrap_or_else(|error| panic!("authored pg_rpg core properties failed: {error}"));
}
