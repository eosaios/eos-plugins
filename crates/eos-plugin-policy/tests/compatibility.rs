use eos_plugin_policy::*;
use eos_plugin_protocol::*;
fn host() -> HostDescriptor {
    serde_json::from_value(serde_json::json!({
 "applicationId":"fixture-web","hostVersion":"1.0.0","platformApiVersion":"1.0.0","environmentId":"fixture",
 "hostInstanceId":"fixture-instance","platform":{"os":"web","arch":"any","surface":"web"},
 "executors":[{"type":"skills/v1","abi":"1.0.0"},{"type":"webview/v1","abi":"1.0.0"}],
 "capabilities":[{"id":"eos.ai.skills","version":"1.0.0"},{"id":"eos.ui.panel","version":"1.0.0"}],
 "uiSlots":[{"id":"panel","kind":"panel","version":"1.0.0"}],"policyRevision":1
})).unwrap()
}
#[test]
fn portable_requires_real_capabilities_and_optional_modules_are_explicit() {
    let m = parse_manifest(include_bytes!(
        "../../../fixtures/portable-skills/plugin.toml"
    ))
    .unwrap();
    let mut h = host();
    let r = compatible(&m, &h, &[]);
    assert_eq!(
        r,
        CompatibilityResult {
            status: "compatible".into(),
            selected_modules: vec!["writing-skills".into()],
            unavailable_modules: vec![],
            issues: vec![]
        }
    );
    h.executors.retain(|e| e.r#type != "webview/v1");
    assert_eq!(
        compatible(&m, &h, &[]).unavailable_modules,
        vec!["document-panel"]
    );
    assert_eq!(
        compatible(&m, &h, &["document-panel".into()]).status,
        "incompatible"
    );
    h.capabilities.clear();
    assert_eq!(
        compatible(&m, &h, &[]).issues[0].code,
        "CAPABILITY_UNSUPPORTED"
    );
}
#[test]
fn targeted_is_not_granted_by_product_name_alone() {
    let m = parse_manifest(include_bytes!(
        "../../../fixtures/targeted-tasks/plugin.toml"
    ))
    .unwrap();
    let mut h = host();
    h.application_id = "eos-todo".into();
    assert_eq!(compatible(&m, &h, &[]).status, "incompatible");
    h.executors.push(Executor {
        r#type: "wasm-component/v1".into(),
        abi: "1.0.0".into(),
        profiles: vec![],
    });
    h.capabilities.push(Capability {
        id: "eos.todo.tasks.write".into(),
        version: "1.0.0".into(),
        input_schema_digest: None,
        output_schema_digest: None,
        limits: None,
    });
    assert_eq!(compatible(&m, &h, &[]).status, "authorization-required");
    h.application_id = "other".into();
    assert_eq!(compatible(&m, &h, &[]).status, "incompatible");
}
