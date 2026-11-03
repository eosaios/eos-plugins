use super::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Serialize, Deserialize)]
struct Journal {
    previous: Vec<Installation>,
    candidates: Vec<Installation>,
    phase: String,
    #[serde(default)]
    data_previous: BTreeMap<String, serde_json::Value>,
}
impl PluginStore {
    pub fn install(
        &self,
        input: InstallInput<'_>,
        host: &mut dyn HostAdapter,
    ) -> Result<Installation> {
        let _lock = self.lock()?;
        self.recover_locked(host)?;
        let plan = self.plan(input.plan_id, &host.subject())?;
        if plan.plan_digest != input.plan_digest
            || plan.expires_at <= input.now
            || plan.host_digest != canonical_digest(&host.descriptor())?
            || plan.policy_revision != input.policy.revision
            || input.policy.expires_at <= input.now
        {
            return Err(error("PLAN_CHANGED", "plan or policy no longer valid"));
        }
        if input.policy.subject_id != host.subject()
            || input.policy.environment_id != host.descriptor().environment_id
        {
            return Err(error("NOT_FOUND_OR_FORBIDDEN", "policy identity mismatch"));
        }
        if !input.consented {
            return Err(error(
                "CONSENT_REQUIRED",
                "explicit plan confirmation required",
            ));
        }
        let descriptor = host.descriptor();
        let mut previous = Vec::new();
        let mut candidates = Vec::new();
        let mut reused = BTreeSet::new();
        let mut releases = plan.dependencies.clone();
        releases.push(plan.release.clone());
        for release in &releases {
            let artifact = select_artifact(release, &descriptor)?;
            if !input.policy.allowed_digests.contains(&artifact.digest)
                || input.policy.revoked_digests.contains(&artifact.digest)
            {
                return Err(error(
                    "PACKAGE_REVOKED",
                    "artifact denied by current signed policy",
                ));
            }
            let is_root = release.manifest.plugin.id == plan.release.manifest.plugin.id;
            let bytes = if is_root {
                input.artifact
            } else {
                input
                    .dependency_artifacts
                    .and_then(|v| v.get(&artifact.digest))
                    .map(Vec::as_slice)
                    .ok_or_else(|| {
                        error(
                            "DEPENDENCY_CONFLICT",
                            "missing verified dependency artifact",
                        )
                    })?
            };
            let package = read_package(bytes)?;
            verify_package(&package, &artifact, release)?;
            let key = Self::install_key(
                &descriptor,
                &host.subject(),
                &plan.scope,
                &release.manifest.plugin.id,
            )?;
            let old = if self.record(&key).exists() {
                Some(load::<Installation>(&self.record(&key))?)
            } else {
                None
            };
            if let Some(old) = &old {
                if old.artifact_digest == artifact.digest && old.enabled {
                    if is_root {
                        let desired = input
                            .configurations
                            .and_then(|values| values.get(&release.manifest.plugin.id))
                            .unwrap_or(&old.configuration);
                        if old.selected_modules == plan.selected_modules
                            && desired == &old.configuration
                        {
                            return Err(error(
                                "RESOURCE_CONFLICT",
                                "this version and configuration are already installed",
                            ));
                        }
                    } else {
                        let mut current = old.clone();
                        current.lease_expires_at = input.policy.expires_at;
                        current.policy_revision = input.policy.revision;
                        candidates.push(current);
                        previous.push(old.clone());
                        reused.insert(key);
                        continue;
                    }
                }
            }
            let archive = self.root.join("archives").join(&artifact.digest);
            let mut temporary =
                tempfile::NamedTempFile::new_in(self.root.join("archives")).map_err(io_error)?;
            temporary.write_all(bytes).map_err(io_error)?;
            temporary.as_file().sync_all().map_err(io_error)?;
            temporary.persist(archive).map_err(|e| io_error(e.error))?;
            let dir = self.package_dir(&artifact.digest)?;
            if dir.exists() {
                verify_inventory(&dir, &package)?;
            } else {
                unpack_verified(&package, &dir)?;
            }
            let selected = if is_root {
                plan.selected_modules.clone()
            } else {
                compatible(&release.manifest, &descriptor, &[]).selected_modules
            };
            let candidate = Installation {
                id: key,
                plugin_id: release.manifest.plugin.id.clone(),
                name: release.manifest.plugin.name.clone(),
                version: release.manifest.plugin.version.clone(),
                artifact_digest: artifact.digest,
                manifest: release.manifest.clone(),
                environment_id: descriptor.environment_id.clone(),
                application_id: descriptor.application_id.clone(),
                host_instance_id: descriptor.host_instance_id.clone(),
                subject_id: host.subject(),
                scope: plan.scope.clone(),
                generation: old.as_ref().map_or(1, |i| i.generation + 1),
                state: "active".into(),
                enabled: true,
                selected_modules: selected.clone(),
                grants: permission_requests(&release.manifest, &selected),
                policy_revision: input.policy.revision,
                lease_expires_at: input.policy.expires_at,
                last_error: None,
                configuration: input
                    .configurations
                    .and_then(|v| v.get(&release.manifest.plugin.id))
                    .cloned()
                    .or_else(|| old.as_ref().map(|i| i.configuration.clone()))
                    .unwrap_or_else(|| serde_json::json!({})),
                data_version: release.manifest.data.as_ref().map_or(0, |v| v.version),
            };
            if let Some(configuration) = &candidate.manifest.configuration {
                let schema: serde_json::Value = serde_json::from_slice(
                    package
                        .files
                        .get(&configuration.schema)
                        .ok_or_else(|| error("SCHEMA_INVALID", "configuration schema missing"))?,
                )
                .map_err(|_| error("SCHEMA_INVALID", "invalid configuration schema"))?;
                if plan.configuration_schemas.get(&candidate.plugin_id) != Some(&schema) {
                    return Err(error(
                        "PLAN_CHANGED",
                        "configuration schema differs from confirmed plan",
                    ));
                }
                validate_configuration(&schema, &candidate.configuration)?;
            }
            if let Some(old) = old {
                previous.push(old);
            }
            candidates.push(candidate);
        }
        self.check_dependency_graph(&descriptor, &host.subject(), &candidates)?;
        let root_candidate = candidates
            .iter()
            .find(|i| i.plugin_id == plan.release.manifest.plugin.id)
            .cloned()
            .ok_or_else(|| error("PLAN_CHANGED", "root installation missing"))?;
        let path = self
            .root
            .join("transactions")
            .join(format!("{}.json", plan.id));
        let mut journal = Journal {
            previous: previous.clone(),
            candidates: candidates.clone(),
            phase: "prepared".into(),
            data_previous: BTreeMap::new(),
        };
        atomic(&path, &journal)?;
        let result = (|| {
            for candidate in &candidates {
                if !reused.contains(&candidate.id) {
                    host.prepare(candidate, &self.checked_package_dir(candidate)?)?;
                }
            }
            let mut staged = BTreeMap::new();
            for candidate in &candidates {
                if let Some(old) = previous
                    .iter()
                    .find(|i| i.id == candidate.id && i.data_version != candidate.data_version)
                {
                    if candidate.data_version < old.data_version {
                        return Err(error(
                            "MIGRATION_REQUIRED",
                            "data downgrade requires an explicit reversible migration",
                        ));
                    }
                    let data_path = self.data_dir(&candidate.id)?.join("ui-storage.json");
                    let original = if data_path.exists() {
                        load::<serde_json::Value>(&data_path)?
                    } else {
                        serde_json::Value::Null
                    };
                    journal
                        .data_previous
                        .insert(candidate.id.clone(), original.clone());
                    atomic(&path, &journal)?;
                    let mut value = original;
                    let mut version = old.data_version;
                    while version < candidate.data_version {
                        let contract = candidate.manifest.data.as_ref().ok_or_else(|| {
                            error("MIGRATION_REQUIRED", "missing migration contract")
                        })?;
                        let steps: Vec<_> = contract
                            .migrations
                            .iter()
                            .filter(|m| {
                                m.from_version == version
                                    && m.to_version > version
                                    && m.to_version <= candidate.data_version
                            })
                            .collect();
                        if steps.len() != 1 {
                            return Err(error(
                                "MIGRATION_REQUIRED",
                                "migration chain is missing or ambiguous",
                            ));
                        }
                        let step = steps[0];
                        let module = candidate
                            .manifest
                            .modules
                            .iter()
                            .find(|m| m.id == step.module && m.r#type == "wasm-component/v1")
                            .ok_or_else(|| {
                                error(
                                    "MIGRATION_REQUIRED",
                                    "migration requires a pure WASM module",
                                )
                            })?;
                        value=host.migrate(candidate,module,&self.checked_package_dir(candidate)?,serde_json::json!({"fromVersion":version,"toVersion":step.to_version,"data":value}))?;
                        if !value.is_object()
                            || serde_json::to_vec(&value)
                                .map_err(|_| error("SCHEMA_INVALID", "invalid migration output"))?
                                .len()
                                > 1024 * 1024
                        {
                            return Err(error(
                                "QUOTA_EXCEEDED",
                                "migration must return a private storage object up to 1 MiB",
                            ));
                        }
                        version = step.to_version;
                    }
                    staged.insert(candidate.id.clone(), value);
                }
            }
            journal.phase = "switching".into();
            atomic(&path, &journal)?;
            for old in previous.iter().rev() {
                if !reused.contains(&old.id) {
                    host.deactivate(old)?;
                }
            }
            for (id, value) in staged {
                atomic(&self.data_dir(&id)?.join("ui-storage.json"), &value)?;
            }
            for candidate in &candidates {
                if !reused.contains(&candidate.id) {
                    host.activate(candidate, &self.checked_package_dir(candidate)?)?;
                }
            }
            for candidate in &candidates {
                atomic(&self.record(&candidate.id), candidate)?;
            }
            journal.phase = "committed".into();
            atomic(&path, &journal)?;
            for candidate in &journal.candidates {
                if candidate.enabled {
                    host.commit(candidate)?;
                }
            }
            Ok(())
        })();
        if let Err(original) = result {
            self.restore_data(&journal)?;
            for candidate in candidates.iter().rev() {
                if !reused.contains(&candidate.id) {
                    host.dispose(candidate)?;
                }
            }
            for candidate in &candidates {
                if let Some(old) = previous.iter().find(|i| i.id == candidate.id) {
                    atomic(&self.record(&old.id), old)?;
                } else if self.record(&candidate.id).exists() {
                    fs::remove_file(self.record(&candidate.id)).map_err(io_error)?;
                }
            }
            for old in &previous {
                if old.enabled && old.lease_expires_at > input.now && !reused.contains(&old.id) {
                    host.activate(old, &self.checked_package_dir(old)?)?;
                    host.commit(old)?;
                }
            }
            fs::remove_file(&path).map_err(io_error)?;
            return Err(original);
        }
        fs::remove_file(&path).map_err(io_error)?;
        fs::remove_file(self.root.join("plans").join(format!("{}.json", plan.id)))
            .map_err(io_error)?;
        Ok(root_candidate)
    }
    pub(super) fn switch_state(
        &self,
        previous: &Installation,
        candidate: &Installation,
        now: u64,
        host: &mut dyn HostAdapter,
    ) -> Result<()> {
        let path = self
            .root
            .join("transactions")
            .join(format!("{}.json", Uuid::new_v4()));
        let mut journal = Journal {
            previous: vec![previous.clone()],
            candidates: vec![candidate.clone()],
            phase: "switching".into(),
            data_previous: BTreeMap::new(),
        };
        atomic(&path, &journal)?;
        let result = (|| {
            if candidate.enabled {
                host.prepare(candidate, &self.checked_package_dir(candidate)?)?;
            }
            host.deactivate(previous)?;
            if candidate.enabled {
                host.activate(candidate, &self.checked_package_dir(candidate)?)?;
            }
            atomic(&self.record(&candidate.id), candidate)?;
            journal.phase = "committed".into();
            atomic(&path, &journal)?;
            for candidate in &journal.candidates {
                if candidate.enabled {
                    host.commit(candidate)?;
                }
            }
            Ok(())
        })();
        if let Err(original) = result {
            host.dispose(candidate)?;
            atomic(&self.record(&previous.id), previous)?;
            if previous.enabled && previous.lease_expires_at > now {
                host.activate(previous, &self.checked_package_dir(previous)?)?;
                host.commit(previous)?;
            }
            fs::remove_file(path).map_err(io_error)?;
            return Err(original);
        }
        fs::remove_file(path).map_err(io_error)?;
        Ok(())
    }
    pub fn recover(&self, host: &mut dyn HostAdapter) -> Result<()> {
        let _lock = self.lock()?;
        self.recover_locked(host)
    }
    pub(super) fn recover_locked(&self, host: &mut dyn HostAdapter) -> Result<()> {
        for entry in fs::read_dir(self.root.join("transactions")).map_err(io_error)? {
            let path = entry.map_err(io_error)?.path();
            let journal: Journal = load(&path)?;
            if journal.candidates.iter().any(|i| {
                i.subject_id != host.subject()
                    || i.application_id != host.descriptor().application_id
                    || i.environment_id != host.descriptor().environment_id
                    || i.host_instance_id != host.descriptor().host_instance_id
            }) {
                continue;
            }
            if journal.phase == "committed" {
                for candidate in &journal.candidates {
                    atomic(&self.record(&candidate.id), candidate)?;
                }
            } else {
                self.restore_data(&journal)?;
                for candidate in journal.candidates.iter().rev() {
                    host.dispose(candidate)?;
                }
                for candidate in &journal.candidates {
                    if let Some(old) = journal.previous.iter().find(|i| i.id == candidate.id) {
                        atomic(&self.record(&old.id), old)?;
                    } else if self.record(&candidate.id).exists() {
                        fs::remove_file(self.record(&candidate.id)).map_err(io_error)?;
                    }
                }
            }
            fs::remove_file(path).map_err(io_error)?;
        }
        Ok(())
    }
    fn restore_data(&self, journal: &Journal) -> Result<()> {
        for (id, value) in &journal.data_previous {
            let path = self.data_dir(id)?.join("ui-storage.json");
            if value.is_null() {
                if path.exists() {
                    fs::remove_file(path).map_err(io_error)?;
                }
            } else {
                atomic(&path, value)?;
            }
        }
        Ok(())
    }
    pub(super) fn check_dependency_graph(
        &self,
        host: &HostDescriptor,
        subject: &str,
        candidates: &[Installation],
    ) -> Result<()> {
        let scope = candidates
            .first()
            .map(|i| &i.scope)
            .ok_or_else(|| error("PLAN_CHANGED", "empty installation transaction"))?;
        let mut final_state: BTreeMap<String, Installation> = self
            .read_list(host, subject)?
            .into_iter()
            .filter(|i| i.enabled && &i.scope == scope)
            .map(|i| (i.plugin_id.clone(), i))
            .collect();
        for i in candidates {
            final_state.insert(i.plugin_id.clone(), i.clone());
        }
        for i in final_state.values() {
            for dep in &i.manifest.dependencies {
                if !dep.required
                    && !dep
                        .module
                        .as_ref()
                        .is_some_and(|m| i.selected_modules.contains(m))
                {
                    continue;
                }
                let target = final_state.get(&dep.plugin_id).ok_or_else(|| {
                    error(
                        "DEPENDENCY_CONFLICT",
                        "an installed dependent would lose its dependency",
                    )
                })?;
                let req = semver::VersionReq::parse(&dep.version)
                    .map_err(|_| error("DEPENDENCY_CONFLICT", "invalid version range"))?;
                let version = semver::Version::parse(&target.version).map_err(|_| {
                    error(
                        "DEPENDENCY_CONFLICT",
                        "invalid installed dependency version",
                    )
                })?;
                if !req.matches(&version) {
                    return Err(error(
                        "DEPENDENCY_CONFLICT",
                        format!("{} requires {} {}", i.plugin_id, dep.plugin_id, dep.version),
                    ));
                }
            }
        }
        fn visit(
            id: &str,
            all: &BTreeMap<String, Installation>,
            pending: &mut BTreeSet<String>,
            done: &mut BTreeSet<String>,
        ) -> Result<()> {
            if done.contains(id) {
                return Ok(());
            }
            if !pending.insert(id.into()) {
                return Err(error("DEPENDENCY_CONFLICT", "dependency cycle"));
            }
            if let Some(i) = all.get(id) {
                for dep in &i.manifest.dependencies {
                    if dep.required
                        || dep
                            .module
                            .as_ref()
                            .is_some_and(|m| i.selected_modules.contains(m))
                    {
                        visit(&dep.plugin_id, all, pending, done)?;
                    }
                }
            }
            pending.remove(id);
            done.insert(id.into());
            Ok(())
        }
        let mut done = BTreeSet::new();
        for id in final_state.keys() {
            visit(id, &final_state, &mut BTreeSet::new(), &mut done)?;
        }
        Ok(())
    }
}
