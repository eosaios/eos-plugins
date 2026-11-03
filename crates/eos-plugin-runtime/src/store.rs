//! Durable installation state. Host resources participate via prepare/activate/deactivate.
use crate::{read_package, unpack_verified, Package};
use eos_plugin_policy::{compatible, permission_requests, select_artifact};
use eos_plugin_protocol::*;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub trait HostAdapter {
    fn descriptor(&self) -> HostDescriptor;
    fn subject(&self) -> String;
    fn prepare(&mut self, installation: &Installation, package_dir: &Path) -> Result<()>;
    fn migrate(
        &mut self,
        _installation: &Installation,
        _module: &Module,
        _package_dir: &Path,
        _input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        Err(error(
            "MIGRATION_UNSUPPORTED",
            "host does not offer pure data migrations",
        ))
    }
    fn activate(&mut self, installation: &Installation, package_dir: &Path) -> Result<()>;
    /// Opens capabilities only after the installation records and commit marker are durable.
    fn commit(&mut self, _installation: &Installation) -> Result<()> {
        Ok(())
    }
    fn deactivate(&mut self, installation: &Installation) -> Result<()>;
    fn dispose(&mut self, installation: &Installation) -> Result<()>;
}
#[path = "store_transaction.rs"]
mod transaction;
pub struct PluginStore {
    root: PathBuf,
}
impl PluginStore {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).map_err(io_error)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).map_err(io_error)?;
        }
        for name in [
            "installations",
            "plans",
            "packages",
            "archives",
            "transactions",
            "data",
        ] {
            fs::create_dir_all(root.join(name)).map_err(io_error)?;
        }
        Ok(Self { root })
    }
    fn lock(&self) -> Result<fs::File> {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join(".lock"))
            .map_err(io_error)?;
        file.lock_exclusive().map_err(io_error)?;
        Ok(file)
    }
    fn install_key(
        host: &HostDescriptor,
        subject: &str,
        scope: &InstallationScope,
        id: &str,
    ) -> Result<String> {
        canonical_digest(&(
            host.environment_id.as_str(),
            host.application_id.as_str(),
            host.host_instance_id.as_str(),
            subject,
            scope,
            id,
        ))
    }
    fn record(&self, id: &str) -> PathBuf {
        self.root.join("installations").join(format!("{id}.json"))
    }
    pub fn package_dir(&self, digest: &str) -> Result<PathBuf> {
        if !is_digest(digest) {
            return Err(error("ARCHIVE_UNSAFE", "invalid artifact digest"));
        }
        Ok(self.root.join("packages").join(digest))
    }
    pub fn checked_package_dir(&self, installation: &Installation) -> Result<PathBuf> {
        if !is_digest(&installation.artifact_digest) {
            return Err(error("ARCHIVE_UNSAFE", "invalid installation digest"));
        }
        let bytes = fs::read(
            self.root
                .join("archives")
                .join(&installation.artifact_digest),
        )
        .map_err(io_error)?;
        let package = read_package(&bytes)?;
        if package.digest != installation.artifact_digest
            || package.manifest != installation.manifest
        {
            return Err(error(
                "ARTIFACT_HASH_MISMATCH",
                "installed artifact changed",
            ));
        }
        let dir = self.package_dir(&installation.artifact_digest)?;
        verify_inventory(&dir, &package)?;
        Ok(dir)
    }
    pub fn check_clock(&self, now: u64) -> Result<()> {
        let _lock = self.lock()?;
        let path = self.root.join("clock.json");
        let highest = if path.exists() {
            load::<u64>(&path)?
        } else {
            0
        };
        if now.saturating_add(300) < highest {
            return Err(error(
                "CLOCK_ROLLBACK",
                "system clock moved backwards; restore accurate time before running plugins",
            ));
        }
        if now > highest {
            atomic(&path, &now)?;
        }
        Ok(())
    }
    pub fn data_dir(&self, installation_id: &str) -> Result<PathBuf> {
        if !is_digest(installation_id) {
            return Err(error("ARCHIVE_UNSAFE", "invalid installation ID"));
        }
        let dir = self.root.join("data").join(installation_id);
        fs::create_dir_all(&dir).map_err(io_error)?;
        Ok(dir)
    }
    pub fn list(&self, host: &HostDescriptor, subject: &str) -> Result<Vec<Installation>> {
        let _lock = self.lock()?;
        self.read_list(host, subject)
    }
    fn read_list(&self, host: &HostDescriptor, subject: &str) -> Result<Vec<Installation>> {
        let mut result = Vec::new();
        for entry in fs::read_dir(self.root.join("installations")).map_err(io_error)? {
            let p = entry.map_err(io_error)?.path();
            if p.extension().is_some_and(|v| v == "json") {
                let i: Installation = load(&p)?;
                if i.environment_id == host.environment_id
                    && i.application_id == host.application_id
                    && i.host_instance_id == host.host_instance_id
                    && i.subject_id == subject
                {
                    result.push(i);
                }
            }
        }
        result.sort_by(|a, b| a.plugin_id.cmp(&b.plugin_id));
        Ok(result)
    }
    pub fn prepare_plan(
        &self,
        resolved: ResolveResult,
        host: &HostDescriptor,
        subject: &str,
        request: &ResolveRequest,
        now: u64,
    ) -> Result<InstallPlan> {
        let _lock = self.lock()?;
        if host.environment_id != request.host.environment_id
            || host.application_id != request.host.application_id
        {
            return Err(error("HOST_INCOMPATIBLE", "host context changed"));
        }
        if resolved.release.status != "published"
            || !resolved.release.publisher_approved
            || resolved.lease_expires_at <= now
        {
            return Err(error("PACKAGE_REVOKED", "release unavailable"));
        }
        let compatibility = compatible(&resolved.release.manifest, host, &request.modules);
        if compatibility.status == "incompatible" {
            let mut e = error(
                "HOST_INCOMPATIBLE",
                "required plugin capabilities unavailable",
            );
            e.details = serde_json::to_value(compatibility)
                .map_err(|_| error("SCHEMA_INVALID", "invalid result"))?;
            return Err(e);
        }
        let artifact = select_artifact(&resolved.release, host)?;
        if artifact != resolved.artifact {
            return Err(error(
                "PLAN_CHANGED",
                "resolved artifact does not match host",
            ));
        }
        let mut permissions =
            permission_requests(&resolved.release.manifest, &compatibility.selected_modules);
        for dependency in &resolved.dependencies {
            let chosen = eos_plugin_policy::compatible(&dependency.manifest, host, &[]);
            if chosen.status == "incompatible" {
                return Err(error("DEPENDENCY_CONFLICT", "dependency incompatible"));
            }
            for mut permission in
                permission_requests(&dependency.manifest, &chosen.selected_modules)
            {
                permission.id = format!("{}:{}", dependency.manifest.plugin.id, permission.id);
                permission.reason =
                    format!("{}: {}", dependency.manifest.plugin.name, permission.reason);
                permissions.push(permission);
            }
        }
        let mut plan = InstallPlan {
            id: Uuid::new_v4().to_string(),
            release: resolved.release.clone(),
            artifact,
            host_digest: canonical_digest(host)?,
            subject_id: subject.into(),
            scope: request.scope.clone(),
            selected_modules: compatibility.selected_modules.clone(),
            permission_requests: permissions,
            dependencies: resolved.dependencies,
            created_at: now,
            expires_at: now + 600,
            policy_revision: resolved.policy_revision,
            configuration_schemas: resolved.configuration_schemas,
            plan_digest: String::new(),
        };
        plan.plan_digest = canonical_digest(&plan)?;
        atomic(
            &self.root.join("plans").join(format!("{}.json", plan.id)),
            &plan,
        )?;
        Ok(plan)
    }
    pub fn plan(&self, id: &str, subject: &str) -> Result<InstallPlan> {
        if Uuid::parse_str(id).is_err() {
            return Err(error("PLAN_EXPIRED", "invalid plan"));
        }
        let mut p: InstallPlan = load(&self.root.join("plans").join(format!("{id}.json")))?;
        if p.subject_id != subject {
            return Err(error("NOT_FOUND_OR_FORBIDDEN", "plan inaccessible"));
        }
        let confirmed = p.plan_digest.clone();
        p.plan_digest.clear();
        if canonical_digest(&p)? != confirmed {
            return Err(error("PLAN_CHANGED", "persisted plan changed"));
        }
        p.plan_digest = confirmed;
        Ok(p)
    }
    pub fn set_enabled(
        &self,
        id: &str,
        enabled: bool,
        now: u64,
        host: &mut dyn HostAdapter,
    ) -> Result<Installation> {
        let _lock = self.lock()?;
        self.recover_locked(host)?;
        let mut i = self.owned(id, host)?;
        if enabled {
            if i.lease_expires_at <= now || matches!(i.state.as_str(), "revoked" | "policy-blocked")
            {
                return Err(error(
                    "POLICY_LEASE_EXPIRED",
                    "refresh policy before enabling",
                ));
            }
            if compatible(&i.manifest, &host.descriptor(), &i.selected_modules).status
                == "incompatible"
            {
                return Err(error("HOST_INCOMPATIBLE", "host no longer supports plugin"));
            }
        } else {
            self.ensure_no_dependents(&i, host)?;
        }
        let previous = i.clone();
        i.enabled = enabled;
        i.generation += 1;
        i.state = if enabled { "active" } else { "disabled" }.into();
        if enabled {
            self.check_dependency_graph(&host.descriptor(), &host.subject(), &[i.clone()])?;
        }
        self.switch_state(&previous, &i, now, host)?;
        Ok(i)
    }
    fn owned(&self, id: &str, host: &dyn HostAdapter) -> Result<Installation> {
        if !is_digest(id) {
            return Err(error("NOT_FOUND_OR_FORBIDDEN", "invalid installation ID"));
        }
        let i: Installation = load(&self.record(id))?;
        if i.environment_id != host.descriptor().environment_id
            || i.subject_id != host.subject()
            || i.application_id != host.descriptor().application_id
            || i.host_instance_id != host.descriptor().host_instance_id
        {
            return Err(error("NOT_FOUND_OR_FORBIDDEN", "installation inaccessible"));
        }
        Ok(i)
    }
    pub fn remove(&self, id: &str, mode: RemoveData, host: &mut dyn HostAdapter) -> Result<()> {
        let _lock = self.lock()?;
        self.recover_locked(host)?;
        let i = self.owned(id, host)?;
        self.ensure_no_dependents(&i, host)?;
        host.deactivate(&i)?;
        host.dispose(&i)?;
        fs::remove_file(self.record(id)).map_err(io_error)?;
        if matches!(mode, RemoveData::Purge) && self.root.join("data").join(id).exists() {
            fs::remove_dir_all(self.root.join("data").join(id)).map_err(io_error)?;
        }
        Ok(())
    }
    pub fn apply_policy(
        &self,
        policy: &RuntimePolicy,
        now: u64,
        host: &mut dyn HostAdapter,
    ) -> Result<Vec<Installation>> {
        let _lock = self.lock()?;
        if policy.subject_id != host.subject()
            || policy.environment_id != host.descriptor().environment_id
        {
            return Err(error("NOT_FOUND_OR_FORBIDDEN", "policy identity mismatch"));
        }
        let mut list = self.read_list(&host.descriptor(), &host.subject())?;
        for i in &mut list {
            if policy.revision < i.policy_revision {
                return Err(error("PLAN_CHANGED", "policy revision rollback"));
            }
            if policy.expires_at <= now
                || policy.revoked_digests.contains(&i.artifact_digest)
                || !policy.allowed_digests.contains(&i.artifact_digest)
            {
                host.deactivate(i)?;
                i.enabled = false;
                i.state = if policy.revoked_digests.contains(&i.artifact_digest) {
                    "revoked"
                } else {
                    "policy-blocked"
                }
                .into();
            } else {
                i.lease_expires_at = policy.expires_at;
                if i.state == "policy-blocked" {
                    i.state = "disabled".into();
                }
            }
            i.policy_revision = policy.revision;
            atomic(&self.record(&i.id), i)?;
        }
        loop {
            let blocked: std::collections::BTreeSet<_> = list
                .iter()
                .filter(|i| !i.enabled)
                .map(|i| {
                    (
                        i.scope.kind.clone(),
                        i.scope.id.clone(),
                        i.plugin_id.clone(),
                    )
                })
                .collect();
            let mut changed = false;
            for i in &mut list {
                if i.enabled
                    && i.manifest.dependencies.iter().any(|d| {
                        (d.required
                            || d.module
                                .as_ref()
                                .is_some_and(|m| i.selected_modules.contains(m)))
                            && blocked.contains(&(
                                i.scope.kind.clone(),
                                i.scope.id.clone(),
                                d.plugin_id.clone(),
                            ))
                    })
                {
                    host.deactivate(i)?;
                    i.enabled = false;
                    i.state = "policy-blocked".into();
                    atomic(&self.record(&i.id), i)?;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(list)
    }
    fn ensure_no_dependents(&self, target: &Installation, host: &dyn HostAdapter) -> Result<()> {
        if self
            .read_list(&host.descriptor(), &host.subject())?
            .iter()
            .any(|i| {
                i.id != target.id
                    && i.enabled
                    && i.scope == target.scope
                    && i.manifest.dependencies.iter().any(|d| {
                        d.plugin_id == target.plugin_id
                            && (d.required
                                || d.module
                                    .as_ref()
                                    .is_some_and(|m| i.selected_modules.contains(m)))
                    })
            })
        {
            return Err(error(
                "DEPENDENCY_CONFLICT",
                "disable or remove dependent plugins first",
            ));
        }
        Ok(())
    }
}
pub enum RemoveData {
    Retain,
    Purge,
}
pub struct InstallInput<'a> {
    pub configurations: Option<&'a std::collections::BTreeMap<String, serde_json::Value>>,
    pub plan_id: &'a str,
    pub plan_digest: &'a str,
    pub artifact: &'a [u8],
    pub dependency_artifacts: Option<&'a std::collections::BTreeMap<String, Vec<u8>>>,
    pub policy: &'a RuntimePolicy,
    pub now: u64,
    pub consented: bool,
}
pub fn verify_package(package: &Package, artifact: &Artifact, release: &Release) -> Result<()> {
    if package.digest != artifact.digest
        || package.manifest_digest != artifact.manifest_digest
        || package.files_digest != artifact.files_digest
        || package.manifest != release.manifest
    {
        return Err(error(
            "ARTIFACT_HASH_MISMATCH",
            "artifact does not match signed release",
        ));
    }
    Ok(())
}
fn verify_inventory(dir: &Path, package: &Package) -> Result<()> {
    fn walk(dir: &Path, root: &Path, paths: &mut std::collections::BTreeSet<String>) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(io_error)? {
            let p = entry.map_err(io_error)?.path();
            let m = fs::symlink_metadata(&p).map_err(io_error)?;
            if m.file_type().is_symlink() {
                return Err(error("ARCHIVE_UNSAFE", "cached package symlink"));
            }
            if m.is_dir() {
                walk(&p, root, paths)?;
            } else if m.is_file() {
                let name = p
                    .strip_prefix(root)
                    .map_err(|_| error("ARCHIVE_UNSAFE", "cached path escape"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                paths.insert(name);
            } else {
                return Err(error("ARCHIVE_UNSAFE", "cached special file"));
            }
        }
        Ok(())
    }
    let mut paths = std::collections::BTreeSet::new();
    walk(dir, dir, &mut paths)?;
    if paths != package.files.keys().cloned().collect() {
        return Err(error("ARTIFACT_HASH_MISMATCH", "cached inventory changed"));
    }
    for record in &package.records {
        let path = dir.join(&record.path);
        let mut parent = path.as_path();
        while parent != dir {
            if fs::symlink_metadata(parent)
                .map_err(io_error)?
                .file_type()
                .is_symlink()
            {
                return Err(error("ARCHIVE_UNSAFE", "cached package symlink"));
            }
            parent = parent
                .parent()
                .ok_or_else(|| error("ARCHIVE_UNSAFE", "invalid cached path"))?;
        }
        let bytes = fs::read(path).map_err(io_error)?;
        if digest(&bytes) != record.digest {
            return Err(error(
                "ARTIFACT_HASH_MISMATCH",
                "cached package was modified",
            ));
        }
    }
    Ok(())
}
fn load<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path).map_err(io_error)?)
        .map_err(|_| error("STORAGE_CORRUPT", "invalid persisted plugin state"))
}
fn atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| error("IO_FAILED", "invalid state path"))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    tmp.write_all(
        &serde_json::to_vec(value).map_err(|_| error("SCHEMA_INVALID", "cannot persist state"))?,
    )
    .map_err(io_error)?;
    tmp.as_file().sync_all().map_err(io_error)?;
    tmp.persist(path).map_err(|e| io_error(e.error))?;
    fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(io_error)?;
    Ok(())
}
