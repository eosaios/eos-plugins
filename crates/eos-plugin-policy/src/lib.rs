//! Deterministic, IO-free policy shared by servers and hosts.
use eos_plugin_protocol::*;
use semver::{Version, VersionReq};

fn matches(range: &str, version: &str) -> bool {
    VersionReq::parse(range)
        .ok()
        .zip(Version::parse(version).ok())
        .is_some_and(|(r, v)| r.matches(&v))
}
fn apps_match(apps: &[ApplicationRequirement], host: &HostDescriptor) -> bool {
    apps.is_empty()
        || apps.iter().any(|a| {
            a.id == host.application_id
                && a.version
                    .as_deref()
                    .is_none_or(|v| matches(v, &host.host_version))
        })
}
pub fn compatible(
    m: &Manifest,
    host: &HostDescriptor,
    requested: &[String],
) -> CompatibilityResult {
    let mut result = CompatibilityResult {
        status: "compatible".into(),
        selected_modules: vec![],
        unavailable_modules: vec![],
        issues: vec![],
    };
    let mut push =
        |module: Option<String>, code: &str, field: &str, expected: String, actual: String| {
            result.issues.push(CompatibilityIssue {
                code: code.into(),
                module,
                field: field.into(),
                expected,
                actual,
            });
        };
    if !matches(&m.compatibility.platform_api, &host.platform_api_version) {
        push(
            None,
            "HOST_INCOMPATIBLE",
            "platformApi",
            m.compatibility.platform_api.clone(),
            host.platform_api_version.clone(),
        );
    }
    if m.compatibility.scope == "targeted" && !apps_match(&m.compatibility.applications, host) {
        push(
            None,
            "HOST_INCOMPATIBLE",
            "applications",
            format!("{:?}", m.compatibility.applications),
            host.application_id.clone(),
        );
    }
    if let Some(configuration) = &m.configuration {
        for credential in &configuration.credentials {
            let capability = format!("eos.credentials.{}", credential.kind);
            if !host.capabilities.iter().any(|c| c.id == capability) {
                push(
                    None,
                    "CAPABILITY_UNSUPPORTED",
                    "configuration.credentials",
                    capability,
                    "not provided".into(),
                );
            }
        }
    }
    for id in requested {
        if !m.modules.iter().any(|v| &v.id == id) {
            push(
                Some(id.clone()),
                "SCHEMA_INVALID",
                "modules",
                "known module".into(),
                id.clone(),
            );
        }
    }
    for module in &m.modules {
        let selected = module.required || requested.contains(&module.id);
        let mut issues = Vec::new();
        let mut reject = |code: &str, field: &str, expected: String, actual: String| {
            issues.push(CompatibilityIssue {
                code: code.into(),
                module: Some(module.id.clone()),
                field: field.into(),
                expected,
                actual,
            });
        };
        if !apps_match(&module.applications, host) {
            reject(
                "HOST_INCOMPATIBLE",
                "applications",
                "supported application".into(),
                host.application_id.clone(),
            );
        }
        match host
            .executors
            .iter()
            .find(|e| e.r#type == module.r#type && e.abi == "1.0.0")
        {
            None => reject(
                "EXECUTOR_UNSUPPORTED",
                "type",
                module.r#type.clone(),
                "not provided".into(),
            ),
            Some(exec) => {
                if let Some(profile) = &module.profile {
                    if !exec.profiles.contains(profile) {
                        reject(
                            "ISOLATION_UNAVAILABLE",
                            "profile",
                            profile.clone(),
                            "not provided".into(),
                        );
                    }
                }
            }
        }
        for r in &module.requires {
            if !host
                .capabilities
                .iter()
                .any(|c| c.id == r.capability && matches(&r.version, &c.version))
            {
                reject(
                    "CAPABILITY_UNSUPPORTED",
                    "requires",
                    format!("{} {}", r.capability, r.version),
                    "not provided".into(),
                );
            }
        }
        for c in &module.contributions {
            if !host.ui_slots.iter().any(|s| s.kind == c.kind) {
                reject(
                    "CAPABILITY_UNSUPPORTED",
                    "contributions",
                    c.kind.clone(),
                    "no compatible slot".into(),
                );
            }
        }
        if issues.is_empty() {
            if selected {
                result.selected_modules.push(module.id.clone());
            }
        } else {
            result.unavailable_modules.push(module.id.clone());
            if selected {
                result.issues.extend(issues);
            }
        }
    }
    if !result.issues.is_empty() {
        result.status = "incompatible".into();
    } else if !permission_requests(m, &result.selected_modules).is_empty()
        || m.modules.iter().any(|v| {
            result.selected_modules.contains(&v.id)
                && (v.profile.as_deref() == Some("trusted-process")
                    || matches!(v.activation.as_str(), "on-event" | "on-startup"))
        })
    {
        result.status = "authorization-required".into();
    }
    result
}
pub fn permission_requests(m: &Manifest, selected: &[String]) -> Vec<Permission> {
    m.permissions
        .iter()
        .filter(|p| {
            m.modules
                .iter()
                .any(|v| selected.contains(&v.id) && v.permissions.contains(&p.id))
        })
        .cloned()
        .collect()
}
pub fn select_artifact(release: &Release, host: &HostDescriptor) -> Result<Artifact> {
    let mut candidates: Vec<(&Artifact, u8)> = release
        .artifacts
        .iter()
        .filter(|a| a.manifest_digest == release.manifest_digest)
        .filter_map(|a| {
            let exact = a.target.os == host.platform.os && a.target.arch == host.platform.arch;
            let portable = a.target.os == "any" && a.target.arch == "any";
            if (exact || portable)
                && a.target.abis.iter().all(|abi| {
                    host.executors
                        .iter()
                        .any(|e| format!("{}@{}", e.r#type, e.abi) == *abi)
                })
            {
                Some((a, if exact { 2 } else { 1 }))
            } else {
                None
            }
        })
        .collect();
    candidates.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    match candidates.first() {
        None => Err(error("HOST_INCOMPATIBLE", "no matching artifact")),
        Some((_, score)) if candidates.iter().filter(|(_, s)| s == score).count() != 1 => Err(
            error("TARGET_AMBIGUOUS", "multiple artifacts at same priority"),
        ),
        Some((a, _)) => Ok((*a).clone()),
    }
}
pub fn authorize(request: &Permission, grants: &[Permission]) -> Result<()> {
    // Resource containment is intentionally exact until an owning capability defines a narrower selector.
    if grants.iter().any(|g| {
        g.capability == request.capability
            && request.actions.iter().all(|v| g.actions.contains(v))
            && g.resources == request.resources
    }) {
        Ok(())
    } else {
        Err(error(
            "GRANT_REVOKED",
            "capability/action/resource is not granted",
        ))
    }
}
pub fn needs_reconsent(old: &Installation, plan: &InstallPlan) -> bool {
    old.subject_id != plan.subject_id
        || old.scope != plan.scope
        || plan
            .permission_requests
            .iter()
            .any(|r| authorize(r, &old.grants).is_err())
        || plan
            .release
            .manifest
            .modules
            .iter()
            .filter(|v| plan.selected_modules.contains(&v.id))
            .any(|v| {
                !old.manifest.modules.iter().any(|o| {
                    old.selected_modules.contains(&o.id)
                        && o.id == v.id
                        && o.r#type == v.r#type
                        && o.profile == v.profile
                        && o.endpoint == v.endpoint
                        && o.activation == v.activation
                })
            })
}

/// The skills capability owns its resource selector. Unknown selectors never widen a grant.
pub fn authorize_skill(
    permission: &Permission,
    grants: &[Permission],
    plugin_id: &str,
    name: &str,
) -> Result<()> {
    authorize(permission, grants)?;
    if permission.capability != "eos.ai.skills"
        || !permission.actions.iter().any(|a| a == "invoke")
        || !name.starts_with(&format!("{plugin_id}/"))
    {
        return Err(error(
            "GRANT_REVOKED",
            "skill invocation is outside the installation",
        ));
    }
    if permission.resources.is_empty() {
        return Ok(());
    }
    if permission.resources.len() != 1 {
        return Err(error(
            "RESOURCE_SELECTOR_UNSUPPORTED",
            "unknown skills resource selector",
        ));
    }
    let names = permission
        .resources
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            error(
                "RESOURCE_SELECTOR_UNSUPPORTED",
                "skills selector must name exact skills",
            )
        })?;
    if names.iter().any(|v| v.as_str() == Some(name)) {
        Ok(())
    } else {
        Err(error(
            "GRANT_REVOKED",
            "skill was not included in the resource grant",
        ))
    }
}
