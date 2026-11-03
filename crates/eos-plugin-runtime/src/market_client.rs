//! Reusable authenticated market transport and signed-policy cache. Tokens are never persisted.
use base64::{engine::general_purpose::STANDARD, Engine};
use eos_plugin_protocol::*;
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{io::Read, path::Path, time::Duration};
pub type Access = eos_plugin_protocol::MarketAccess;
pub struct MarketClient<'a> {
    access: &'a Access,
    http: Client,
    root: Vec<u8>,
}
pub struct Verified {
    pub policy: RuntimePolicy,
    pub release: Option<Release>,
    metadata: tempfile::TempDir,
}
impl<'a> MarketClient<'a> {
    pub fn new(access: &'a Access, root: Vec<u8>) -> Result<Self> {
        let url = reqwest::Url::parse(&access.origin)
            .map_err(|_| error("HOST_INCOMPATIBLE", "invalid market origin"))?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if url.host_str().is_none()
            || !(url.scheme() == "https" || url.scheme() == "http" && local)
            || url.path() != "/"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || access.application_id.is_empty()
        {
            return Err(error(
                "HOST_INCOMPATIBLE",
                "invalid authenticated market context",
            ));
        }
        let anonymous = access.subject_id.is_empty()
            && access.account_token.is_empty()
            && access.market_token.is_empty()
            && !access.local_only;
        if !anonymous
            && (access.subject_id.is_empty()
                || (!access.local_only
                    && (access.account_token.is_empty() || access.market_token.is_empty())))
            || access.environment_id.is_empty()
            || !access
                .environment_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(error(
                "UNAUTHENTICATED",
                "sign in to the internal market first",
            ));
        }
        if root.is_empty() {
            return Err(error(
                "MARKET_UNCONFIGURED",
                "an explicitly trusted root is required",
            ));
        }
        let mut builder = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::none());
        if !access.proxy_url.is_empty() {
            let proxy = &access.proxy_url;
            let parsed = reqwest::Url::parse(proxy)
                .map_err(|_| error("NETWORK_FAILED", "invalid configured network proxy"))?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                return Err(error("NETWORK_FAILED", "invalid configured network proxy"));
            }
            builder =
                builder
                    .proxy(reqwest::Proxy::all(proxy).map_err(|_| {
                        error("NETWORK_FAILED", "invalid configured network proxy")
                    })?);
        }
        let http = builder
            .build()
            .map_err(|_| error("NETWORK_FAILED", "cannot create market transport"))?;
        Ok(Self { access, http, root })
    }
    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::blocking::Response> {
        if self.access.subject_id.is_empty() && path != "/search" {
            return Err(error(
                "UNAUTHENTICATED",
                "sign in to install or manage plugins",
            ));
        }
        if self.access.local_only {
            return Err(error(
                "ONLINE_AUTH_REQUIRED",
                "this operation requires a live market session",
            ));
        }
        let mut req = self.http.request(
            method,
            format!(
                "{}/api/v1/plugin-market{path}",
                self.access.origin.trim_end_matches('/')
            ),
        );
        if !self.access.subject_id.is_empty() {
            req = req
                .bearer_auth(&self.access.account_token)
                .header("x-plugin-market-session", &self.access.market_token);
        }
        if let Some(body) = body {
            req = req.json(&body);
        }
        let response = req.send().map_err(|_| {
            error(
                "NETWORK_FAILED",
                "internal market connection failed; no mutation was replayed",
            )
        })?;
        if response.status().is_redirection() {
            return Err(error("NETWORK_FAILED", "market redirect rejected"));
        }
        Ok(response)
    }
    fn data<T: DeserializeOwned>(&self, response: reqwest::blocking::Response) -> Result<T> {
        let status = response.status();
        let mut bytes = Vec::new();
        response
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| error("NETWORK_FAILED", "market response interrupted"))?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(error("QUOTA_EXCEEDED", "market response exceeds limit"));
        }
        let body: Value = serde_json::from_slice(&bytes)
            .map_err(|_| error("NETWORK_FAILED", "invalid market response"))?;
        if !status.is_success() {
            if matches!(status.as_u16(), 401 | 403) {
                return Err(error("MARKET_ACCESS_REVOKED", "market access was rejected"));
            }
            let code = body["error"]["code"]
                .as_str()
                .unwrap_or("MARKET_REQUEST_FAILED");
            let message = body["error"]["message"]
                .as_str()
                .unwrap_or("market request failed");
            return Err(error(code, message));
        }
        serde_json::from_value(body["data"].clone())
            .map_err(|_| error("SCHEMA_INVALID", "market contract mismatch"))
    }
    pub fn search(&self, request: &SearchRequest) -> Result<SearchResult> {
        self.data(self.request(reqwest::Method::POST, "/search", Some(json!(request)))?)
    }
    pub fn resolve(&self, request: &ResolveRequest) -> Result<ResolveResult> {
        self.data(self.request(reqwest::Method::POST, "/resolve", Some(json!(request)))?)
    }
    pub fn verify(&self, cache: &Path, manifest_digest: Option<&str>) -> Result<Verified> {
        let path = match manifest_digest {
            Some(d) if is_digest(d) => format!("/trust-bundle?manifestDigest={d}"),
            Some(_) => return Err(error("SIGNATURE_INVALID", "invalid release digest")),
            None => "/trust-bundle".into(),
        };
        let bundle: Value = self.data(self.request(reqwest::Method::GET, &path, None)?)?;
        let verified = self.verify_bundle(cache, &bundle, manifest_digest)?;
        std::fs::create_dir_all(cache).map_err(io_error)?;
        let mut file = tempfile::NamedTempFile::new_in(cache).map_err(io_error)?;
        use std::io::Write;
        file.write_all(
            &serde_json::to_vec(&bundle)
                .map_err(|_| error("SCHEMA_INVALID", "invalid trust bundle"))?,
        )
        .map_err(io_error)?;
        file.as_file().sync_all().map_err(io_error)?;
        file.persist(cache.join("verified-bundle.json"))
            .map_err(|e| io_error(e.error))?;
        Ok(verified)
    }
    pub fn cached_policy(&self, cache: &Path) -> Result<RuntimePolicy> {
        let bytes = std::fs::read(cache.join("verified-bundle.json")).map_err(|_| {
            error(
                "ONLINE_AUTH_REQUIRED",
                "no verified offline policy is available",
            )
        })?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(error("SIGNATURE_INVALID", "cached policy exceeds limit"));
        }
        let bundle: Value = serde_json::from_slice(&bytes)
            .map_err(|_| error("SIGNATURE_INVALID", "invalid cached policy"))?;
        Ok(self.verify_bundle(cache, &bundle, None)?.policy)
    }
    fn verify_bundle(
        &self,
        cache: &Path,
        bundle: &Value,
        manifest_digest: Option<&str>,
    ) -> Result<Verified> {
        if bundle["subjectId"].as_str() != Some(&self.access.subject_id)
            || bundle["environmentId"].as_str() != Some(&self.access.environment_id)
        {
            return Err(error("SIGNATURE_INVALID", "catalog identity mismatch"));
        }
        let root = &self.root;
        let metadata = tempfile::tempdir().map_err(io_error)?;
        let dir = metadata.path();
        std::fs::create_dir(dir.join("metadata")).map_err(io_error)?;
        std::fs::create_dir(dir.join("targets")).map_err(io_error)?;
        for (section, destination) in [("metadata", "metadata"), ("targets", "targets")] {
            let entries = bundle[section]
                .as_object()
                .ok_or_else(|| error("SIGNATURE_INVALID", "missing catalog data"))?;
            for (name, value) in entries {
                if !safe_path(name) || name.contains('/') {
                    return Err(error("SIGNATURE_INVALID", "unsafe metadata path"));
                }
                let bytes = STANDARD
                    .decode(
                        value
                            .as_str()
                            .ok_or_else(|| error("SIGNATURE_INVALID", "invalid metadata bytes"))?,
                    )
                    .map_err(|_| error("SIGNATURE_INVALID", "invalid metadata encoding"))?;
                std::fs::write(dir.join(destination).join(name), bytes).map_err(io_error)?;
            }
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| error("IO_FAILED", "verification runtime unavailable"))?;
        let policy_bytes = runtime.block_on(crate::verify_tuf_target(
            root,
            &dir.join("metadata"),
            &dir.join("targets"),
            cache,
            "policy.json",
        ))?;
        let mut policy: RuntimePolicy = serde_json::from_slice(&policy_bytes)
            .map_err(|_| error("SIGNATURE_INVALID", "invalid signed policy"))?;
        if policy.environment_id != self.access.environment_id
            || policy.subject_id != "market-readers"
            || policy.issued_at > unix_time() + 300
            || policy.expires_at <= unix_time()
            || policy.expires_at > policy.issued_at + 24 * 3600
            || policy.revision != bundle["revision"].as_u64().unwrap_or(0)
        {
            return Err(error(
                "SIGNATURE_INVALID",
                "signed policy is expired or inconsistent",
            ));
        }
        policy.subject_id = self.access.subject_id.clone();
        let release = if let Some(digest) = manifest_digest {
            let bytes = runtime.block_on(crate::verify_tuf_target(
                root,
                &dir.join("metadata"),
                &dir.join("targets"),
                cache,
                &format!("release-{digest}.json"),
            ))?;
            let r: Release = serde_json::from_slice(&bytes)
                .map_err(|_| error("SIGNATURE_INVALID", "invalid signed release"))?;
            if r.manifest_digest != digest {
                return Err(error("SIGNATURE_INVALID", "signed release digest mismatch"));
            }
            Some(r)
        } else {
            None
        };
        Ok(Verified {
            policy,
            release,
            metadata,
        })
    }
    pub fn download(
        &self,
        artifact: &Artifact,
        verified: &Verified,
        cache: &Path,
    ) -> Result<Vec<u8>> {
        let auth: DownloadAuthorization = self.data(self.request(
            reqwest::Method::POST,
            "/download-authorizations",
            Some(json!({"digest":artifact.digest})),
        )?)?;
        if auth.digest != artifact.digest || auth.expires_at <= unix_time() {
            return Err(error(
                "NOT_FOUND_OR_FORBIDDEN",
                "download permission expired",
            ));
        }
        let response = self
            .http
            .get(format!(
                "{}/api/v1/plugin-market/artifacts/{}",
                self.access.origin.trim_end_matches('/'),
                artifact.digest
            ))
            .bearer_auth(&self.access.account_token)
            .header("x-plugin-market-session", &self.access.market_token)
            .header("x-plugin-download-token", &auth.token)
            .send()
            .map_err(|_| error("NETWORK_FAILED", "artifact download interrupted"))?;
        if !response.status().is_success() {
            return Err(error("NOT_FOUND_OR_FORBIDDEN", "artifact download refused"));
        }
        if response
            .content_length()
            .is_some_and(|n| n != artifact.length)
            || artifact.length > crate::MAX_PACKAGE
        {
            return Err(error("ARTIFACT_HASH_MISMATCH", "artifact length mismatch"));
        }
        let mut bytes = Vec::new();
        response
            .take(artifact.length + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| error("NETWORK_FAILED", "artifact download interrupted"))?;
        if bytes.len() as u64 != artifact.length || digest(&bytes) != artifact.digest {
            return Err(error(
                "ARTIFACT_HASH_MISMATCH",
                "artifact bytes differ from approved release",
            ));
        }
        std::fs::write(
            verified
                .metadata
                .path()
                .join("targets")
                .join(&artifact.digest),
            &bytes,
        )
        .map_err(io_error)?;
        let root = &self.root;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| error("IO_FAILED", "verification runtime unavailable"))?;
        runtime.block_on(crate::verify_tuf_target(
            root,
            &verified.metadata.path().join("metadata"),
            &verified.metadata.path().join("targets"),
            cache,
            &artifact.digest,
        ))
    }
}
fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |v| v.as_secs())
}
