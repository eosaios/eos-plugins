use eos_plugin_protocol::*;
use eos_plugin_runtime::*;
use std::{fs, path::Path};
fn host_descriptor() -> HostDescriptor {
    serde_json::from_value(serde_json::json!({
 "applicationId":"fixture","hostVersion":"1.0.0","platformApiVersion":"1.0.0","environmentId":"fixture",
 "hostInstanceId":"instance","platform":{"os":"macos","arch":"aarch64","surface":"desktop"},
 "executors":[{"type":"skills/v1","abi":"1.0.0"}],"capabilities":[{"id":"eos.ai.skills","version":"1.0.0"}],"policyRevision":1
})).unwrap()
}
struct Host {
    subject: String,
    active: Vec<String>,
    fail: bool,
}
impl HostAdapter for Host {
    fn descriptor(&self) -> HostDescriptor {
        host_descriptor()
    }
    fn subject(&self) -> String {
        self.subject.clone()
    }
    fn prepare(&mut self, _: &Installation, _: &Path) -> Result<()> {
        Ok(())
    }
    fn activate(&mut self, i: &Installation, _: &Path) -> Result<()> {
        if self.fail {
            return Err(error("ACTIVATION_FAILED", "injected"));
        }
        self.active.push(i.version.clone());
        Ok(())
    }
    fn deactivate(&mut self, _: &Installation) -> Result<()> {
        self.active.clear();
        Ok(())
    }
    fn dispose(&mut self, _: &Installation) -> Result<()> {
        self.active.clear();
        Ok(())
    }
}
fn source(version: &str) -> (tempfile::TempDir, Vec<u8>) {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("plugin.toml"),
        include_str!("../../../fixtures/portable-skills/plugin.toml")
            .replace("version = \"1.0.0\"", &format!("version = \"{version}\"")),
    )
    .unwrap();
    fs::write(d.path().join("LICENSE"), "fixture").unwrap();
    fs::write(d.path().join("README.md"), "fixture").unwrap();
    fs::create_dir_all(d.path().join("skills/writing")).unwrap();
    fs::write(
        d.path().join("skills/writing/SKILL.md"),
        "---\nname: writing\ndescription: fixture\n---\nWrite a document.",
    )
    .unwrap();
    fs::create_dir_all(d.path().join("ui")).unwrap();
    fs::write(
        d.path().join("ui/index.html"),
        "<!doctype html><title>fixture</title>",
    )
    .unwrap();
    let bytes = pack_directory(d.path()).unwrap();
    (d, bytes)
}
fn resolved(bytes: &[u8]) -> ResolveResult {
    let p = read_package(bytes).unwrap();
    let a = Artifact {
        digest: p.digest,
        length: bytes.len() as u64,
        manifest_digest: p.manifest_digest.clone(),
        files_digest: p.files_digest,
        target: ArtifactTarget {
            os: "any".into(),
            arch: "any".into(),
            abis: vec![],
        },
    };
    ResolveResult {
        release: Release {
            manifest: p.manifest,
            manifest_digest: p.manifest_digest,
            artifacts: vec![a.clone()],
            status: "published".into(),
            revision: 1,
            publisher_approved: true,
        },
        artifact: a,
        dependencies: vec![],
        policy_revision: 1,
        lease_expires_at: 1000,
        trust_revision: 1,
        configuration_schemas: Default::default(),
    }
}
fn policy(bytes: &[u8]) -> RuntimePolicy {
    RuntimePolicy {
        environment_id: "fixture".into(),
        subject_id: "alice".into(),
        revision: 1,
        issued_at: 100,
        expires_at: 1000,
        allowed_digests: vec![digest(bytes)],
        revoked_digests: vec![],
    }
}
#[test]
fn install_restart_account_isolation_disable_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    let (_src, bytes) = source("1.0.0");
    let resolved = resolved(&bytes);
    let req = ResolveRequest {
        plugin_id: "eos.document-tools".into(),
        version: None,
        host: host_descriptor(),
        modules: vec![],
        scope: InstallationScope {
            kind: "user".into(),
            id: "alice".into(),
        },
    };
    let plan = store
        .prepare_plan(resolved, &req.host, "alice", &req, 100)
        .unwrap();
    let mut host = Host {
        subject: "alice".into(),
        active: vec![],
        fail: false,
    };
    assert_eq!(
        store
            .install(
                InstallInput {
                    configurations: None,
                    plan_id: &plan.id,
                    plan_digest: &plan.plan_digest,
                    artifact: &bytes,
                    dependency_artifacts: None,
                    policy: &policy(&bytes),
                    now: 101,
                    consented: false
                },
                &mut host
            )
            .unwrap_err()
            .code,
        "CONSENT_REQUIRED"
    );
    let i = store
        .install(
            InstallInput {
                configurations: None,
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &bytes,
                dependency_artifacts: None,
                policy: &policy(&bytes),
                now: 101,
                consented: true,
            },
            &mut host,
        )
        .unwrap();
    assert_eq!(host.active, vec!["1.0.0"]);
    let reopened = PluginStore::open(dir.path().into()).unwrap();
    assert_eq!(
        reopened.list(&host_descriptor(), "alice").unwrap(),
        vec![i.clone()]
    );
    assert!(reopened.list(&host_descriptor(), "bob").unwrap().is_empty());
    reopened.set_enabled(&i.id, false, 102, &mut host).unwrap();
    assert!(host.active.is_empty());
    host.subject = "bob".into();
    assert!(reopened
        .remove(&i.id, RemoveData::Retain, &mut host)
        .is_err());
    host.subject = "alice".into();
    reopened
        .remove(&i.id, RemoveData::Retain, &mut host)
        .unwrap();
    assert!(reopened
        .list(&host_descriptor(), "alice")
        .unwrap()
        .is_empty());
}
#[test]
fn changed_plan_and_mutated_artifact_never_activate() {
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    let (_src, bytes) = source("1.0.0");
    let req = ResolveRequest {
        plugin_id: "eos.document-tools".into(),
        version: None,
        host: host_descriptor(),
        modules: vec![],
        scope: InstallationScope {
            kind: "user".into(),
            id: "alice".into(),
        },
    };
    let p = store
        .prepare_plan(resolved(&bytes), &req.host, "alice", &req, 100)
        .unwrap();
    let mut host = Host {
        subject: "alice".into(),
        active: vec![],
        fail: false,
    };
    assert_eq!(
        store
            .install(
                InstallInput {
                    configurations: None,
                    plan_id: &p.id,
                    plan_digest: "different",
                    artifact: &bytes,
                    dependency_artifacts: None,
                    policy: &policy(&bytes),
                    now: 101,
                    consented: true
                },
                &mut host
            )
            .unwrap_err()
            .code,
        "PLAN_CHANGED"
    );
    let (_src2, changed) = source("1.0.1");
    assert_eq!(
        store
            .install(
                InstallInput {
                    configurations: None,
                    plan_id: &p.id,
                    plan_digest: &p.plan_digest,
                    artifact: &changed,
                    dependency_artifacts: None,
                    policy: &policy(&bytes),
                    now: 101,
                    consented: true
                },
                &mut host
            )
            .unwrap_err()
            .code,
        "ARTIFACT_HASH_MISMATCH"
    );
    assert!(host.active.is_empty());
}
#[test]
fn activation_failure_leaves_no_installed_record() {
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    let (_src, bytes) = source("1.0.0");
    let req = ResolveRequest {
        plugin_id: "eos.document-tools".into(),
        version: None,
        host: host_descriptor(),
        modules: vec![],
        scope: InstallationScope {
            kind: "user".into(),
            id: "alice".into(),
        },
    };
    let p = store
        .prepare_plan(resolved(&bytes), &req.host, "alice", &req, 100)
        .unwrap();
    let mut host = Host {
        subject: "alice".into(),
        active: vec![],
        fail: true,
    };
    assert_eq!(
        store
            .install(
                InstallInput {
                    configurations: None,
                    plan_id: &p.id,
                    plan_digest: &p.plan_digest,
                    artifact: &bytes,
                    dependency_artifacts: None,
                    policy: &policy(&bytes),
                    now: 101,
                    consented: true
                },
                &mut host
            )
            .unwrap_err()
            .code,
        "ACTIVATION_FAILED"
    );
    assert!(store.list(&host_descriptor(), "alice").unwrap().is_empty());
}

#[test]
fn dependency_install_is_atomic_and_dependents_protect_removal() {
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    let (_child, child_bytes) = source("1.0.0");
    let child = resolved(&child_bytes).release;
    let (root_source, _) = source("1.0.0");
    let manifest = root_source.path().join("plugin.toml");
    let text=fs::read_to_string(&manifest).unwrap().replace("eos.document-tools","eos.root-kit")+"\n[[dependencies]]\npluginId = \"eos.document-tools\"\nversion = \"^1.0.0\"\nrequired = true\n";
    fs::write(manifest, text).unwrap();
    let root_bytes = pack_directory(root_source.path()).unwrap();
    let mut root = resolved(&root_bytes);
    root.dependencies = vec![child];
    let req = ResolveRequest {
        plugin_id: "eos.root-kit".into(),
        version: None,
        host: host_descriptor(),
        modules: vec![],
        scope: InstallationScope {
            kind: "user".into(),
            id: "alice".into(),
        },
    };
    let plan = store
        .prepare_plan(root, &req.host, "alice", &req, 100)
        .unwrap();
    let mut p = policy(&root_bytes);
    p.allowed_digests.push(digest(&child_bytes));
    let artifacts = std::collections::BTreeMap::from([(digest(&child_bytes), child_bytes)]);
    let mut host = Host {
        subject: "alice".into(),
        active: vec![],
        fail: false,
    };
    let root = store
        .install(
            InstallInput {
                configurations: None,
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &root_bytes,
                dependency_artifacts: Some(&artifacts),
                policy: &p,
                now: 101,
                consented: true,
            },
            &mut host,
        )
        .unwrap();
    let installed = store.list(&host_descriptor(), "alice").unwrap();
    assert_eq!(installed.len(), 2);
    let child = installed
        .iter()
        .find(|i| i.plugin_id == "eos.document-tools")
        .unwrap();
    assert_eq!(
        store
            .remove(&child.id, RemoveData::Retain, &mut host)
            .unwrap_err()
            .code,
        "DEPENDENCY_CONFLICT"
    );
    store
        .remove(&root.id, RemoveData::Retain, &mut host)
        .unwrap();
    store
        .remove(&child.id, RemoveData::Retain, &mut host)
        .unwrap();
    assert!(store.list(&host_descriptor(), "alice").unwrap().is_empty());
}
#[test]
fn crash_during_activation_is_recovered_from_the_durable_journal() {
    struct CrashHost(Host);
    impl HostAdapter for CrashHost {
        fn descriptor(&self) -> HostDescriptor {
            self.0.descriptor()
        }
        fn subject(&self) -> String {
            self.0.subject()
        }
        fn prepare(&mut self, i: &Installation, p: &Path) -> Result<()> {
            self.0.prepare(i, p)
        }
        fn activate(&mut self, _: &Installation, _: &Path) -> Result<()> {
            panic!("synthetic process interruption")
        }
        fn deactivate(&mut self, i: &Installation) -> Result<()> {
            self.0.deactivate(i)
        }
        fn dispose(&mut self, i: &Installation) -> Result<()> {
            self.0.dispose(i)
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    let (_src, bytes) = source("1.0.0");
    let req = ResolveRequest {
        plugin_id: "eos.document-tools".into(),
        version: None,
        host: host_descriptor(),
        modules: vec![],
        scope: InstallationScope {
            kind: "user".into(),
            id: "alice".into(),
        },
    };
    let plan = store
        .prepare_plan(resolved(&bytes), &req.host, "alice", &req, 100)
        .unwrap();
    let mut crashed = CrashHost(Host {
        subject: "alice".into(),
        active: vec![],
        fail: false,
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| store.install(
            InstallInput {
                configurations: None,
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &bytes,
                dependency_artifacts: None,
                policy: &policy(&bytes),
                now: 101,
                consented: true
            },
            &mut crashed
        )))
        .is_err()
    );
    let reopened = PluginStore::open(dir.path().into()).unwrap();
    reopened.recover(&mut crashed.0).unwrap();
    assert!(reopened
        .list(&host_descriptor(), "alice")
        .unwrap()
        .is_empty());
    assert_eq!(
        fs::read_dir(dir.path().join("transactions"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn upgrade_migrates_private_data_and_activation_failure_restores_previous_generation() {
    struct MigratingHost {
        host: Host,
        fail_upgrade: bool,
    }
    impl HostAdapter for MigratingHost {
        fn descriptor(&self) -> HostDescriptor {
            let mut h = host_descriptor();
            h.executors.push(Executor {
                r#type: "wasm-component/v1".into(),
                abi: "1.0.0".into(),
                profiles: vec![],
            });
            h
        }
        fn subject(&self) -> String {
            self.host.subject()
        }
        fn prepare(&mut self, i: &Installation, p: &Path) -> Result<()> {
            self.host.prepare(i, p)
        }
        fn migrate(
            &mut self,
            _: &Installation,
            _: &Module,
            _: &Path,
            input: serde_json::Value,
        ) -> Result<serde_json::Value> {
            assert_eq!(input["data"], serde_json::json!({"note":"original"}));
            Ok(serde_json::json!({"note":"migrated"}))
        }
        fn activate(&mut self, i: &Installation, p: &Path) -> Result<()> {
            if self.fail_upgrade && i.version == "2.0.0" {
                return Err(error("ACTIVATION_FAILED", "injected upgrade failure"));
            }
            self.host.activate(i, p)
        }
        fn deactivate(&mut self, i: &Installation) -> Result<()> {
            self.host.deactivate(i)
        }
        fn dispose(&mut self, i: &Installation) -> Result<()> {
            self.host.dispose(i)
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    let (_src, bytes) = source("1.0.0");
    let mut host = MigratingHost {
        host: Host {
            subject: "alice".into(),
            active: vec![],
            fail: false,
        },
        fail_upgrade: true,
    };
    let request = ResolveRequest {
        plugin_id: "eos.document-tools".into(),
        version: None,
        host: host.descriptor(),
        modules: vec![],
        scope: InstallationScope {
            kind: "user".into(),
            id: "alice".into(),
        },
    };
    let plan = store
        .prepare_plan(resolved(&bytes), &request.host, "alice", &request, 100)
        .unwrap();
    let installed = store
        .install(
            InstallInput {
                configurations: None,
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &bytes,
                dependency_artifacts: None,
                policy: &policy(&bytes),
                now: 101,
                consented: true,
            },
            &mut host,
        )
        .unwrap();
    let path = store
        .data_dir(&installed.id)
        .unwrap()
        .join("ui-storage.json");
    fs::write(&path, r#"{"note":"original"}"#).unwrap();
    let (upgrade, _) = source("2.0.0");
    let manifest = upgrade.path().join("plugin.toml");
    let text=fs::read_to_string(&manifest).unwrap()+"\n[[modules]]\nid=\"migration\"\ntype=\"wasm-component/v1\"\nworld=\"eos:plugin/component@1.0.0\"\nrequired=false\nentry=\"migrate.wasm\"\nactivation=\"on-demand\"\nrequires=[]\npermissions=[]\n[data]\nversion=1\n[[data.migrations]]\nfromVersion=0\ntoVersion=1\nmodule=\"migration\"\nreversible=false\n";
    fs::write(manifest, text).unwrap();
    fs::write(
        upgrade.path().join("migrate.wasm"),
        b"synthetic host fixture",
    )
    .unwrap();
    let upgraded = pack_directory(upgrade.path()).unwrap();
    let plan = store
        .prepare_plan(resolved(&upgraded), &request.host, "alice", &request, 102)
        .unwrap();
    assert_eq!(
        store
            .install(
                InstallInput {
                    configurations: None,
                    plan_id: &plan.id,
                    plan_digest: &plan.plan_digest,
                    artifact: &upgraded,
                    dependency_artifacts: None,
                    policy: &policy(&upgraded),
                    now: 103,
                    consented: true
                },
                &mut host
            )
            .unwrap_err()
            .code,
        "ACTIVATION_FAILED"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap(),
        serde_json::json!({"note":"original"})
    );
    assert_eq!(
        store.list(&host.descriptor(), "alice").unwrap(),
        vec![installed]
    );
    host.fail_upgrade = false;
    let installed = store
        .install(
            InstallInput {
                configurations: None,
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &upgraded,
                dependency_artifacts: None,
                policy: &policy(&upgraded),
                now: 104,
                consented: true,
            },
            &mut host,
        )
        .unwrap();
    assert_eq!(
        (installed.version.as_str(), installed.data_version),
        ("2.0.0", 1)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(path).unwrap()).unwrap(),
        serde_json::json!({"note":"migrated"})
    );
}

#[test]
fn configuration_is_validated_against_the_confirmed_package_schema() {
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    let (source, _) = source("1.0.0");
    let path = source.path().join("plugin.toml");
    fs::write(
        &path,
        fs::read_to_string(&path).unwrap()
            + "\n[configuration]\nschema=\"configuration.schema.json\"\n",
    )
    .unwrap();
    let schema = serde_json::json!({"type":"object","properties":{"language":{"enum":["zh","en"]}},"required":["language"],"additionalProperties":false});
    fs::write(
        source.path().join("configuration.schema.json"),
        schema.to_string(),
    )
    .unwrap();
    let bytes = pack_directory(source.path()).unwrap();
    let mut resolution = resolved(&bytes);
    resolution
        .configuration_schemas
        .insert("eos.document-tools".into(), schema);
    let request = ResolveRequest {
        plugin_id: "eos.document-tools".into(),
        version: None,
        host: host_descriptor(),
        modules: vec![],
        scope: InstallationScope {
            kind: "user".into(),
            id: "alice".into(),
        },
    };
    let plan = store
        .prepare_plan(resolution, &request.host, "alice", &request, 100)
        .unwrap();
    let mut host = Host {
        subject: "alice".into(),
        active: vec![],
        fail: false,
    };
    let bad = store
        .install(
            InstallInput {
                configurations: None,
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &bytes,
                dependency_artifacts: None,
                policy: &policy(&bytes),
                now: 101,
                consented: true,
            },
            &mut host,
        )
        .unwrap_err();
    assert_eq!(bad.code, "CONFIGURATION_INVALID");
    assert!(host.active.is_empty());
    let values = std::collections::BTreeMap::from([(
        "eos.document-tools".into(),
        serde_json::json!({"language":"zh"}),
    )]);
    let installed = store
        .install(
            InstallInput {
                configurations: Some(&values),
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &bytes,
                dependency_artifacts: None,
                policy: &policy(&bytes),
                now: 102,
                consented: true,
            },
            &mut host,
        )
        .unwrap();
    assert_eq!(
        installed.configuration,
        serde_json::json!({"language":"zh"})
    );
    let mut updated = resolved(&bytes);
    updated.configuration_schemas = plan.configuration_schemas.clone();
    let plan = store
        .prepare_plan(updated, &request.host, "alice", &request, 103)
        .unwrap();
    let values = std::collections::BTreeMap::from([(
        "eos.document-tools".into(),
        serde_json::json!({"language":"en"}),
    )]);
    let configured = store
        .install(
            InstallInput {
                configurations: Some(&values),
                plan_id: &plan.id,
                plan_digest: &plan.plan_digest,
                artifact: &bytes,
                dependency_artifacts: None,
                policy: &policy(&bytes),
                now: 104,
                consented: true,
            },
            &mut host,
        )
        .unwrap();
    assert_eq!(
        (configured.generation, configured.configuration),
        (2, serde_json::json!({"language":"en"}))
    );
}

#[test]
fn clock_rollback_cannot_extend_an_offline_lease() {
    let dir = tempfile::tempdir().unwrap();
    let store = PluginStore::open(dir.path().into()).unwrap();
    store.check_clock(1000).unwrap();
    assert_eq!(store.check_clock(200).unwrap_err().code, "CLOCK_ROLLBACK");
    store.check_clock(1010).unwrap();
}
