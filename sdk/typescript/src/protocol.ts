// @generated; schema sha256=3e6c68f09d270f91851850208e54b0e85a007bcd767a6b1e2925f426438a4011
export interface PluginMeta {
  "id": string;
  "publisher": string;
  "name": string;
  "version": string;
  "description": string;
  "license": string;
  "homepage"?: string;
  "repository"?: string;
  "icon"?: string;
  "i18n"?: Record<string, string>;
}
export interface ApplicationRequirement {
  "id": string;
  "version"?: string;
}
export interface CapabilityRequirement {
  "capability": string;
  "version": string;
}
export interface Compatibility {
  "platformApi": string;
  "scope": "portable" | "targeted";
  "applications"?: Array<ApplicationRequirement>;
}
export interface Contribution {
  "kind": "panel" | "page" | "command" | "settings" | "tool" | "event";
  "id": string;
  "title": string;
  "command"?: string;
}
export interface ExecutionLimits {
  "memoryBytes"?: number;
  "fuel"?: number;
  "timeoutMs"?: number;
  "concurrency"?: number;
}
export interface Module {
  "id": string;
  "type": "skills/v1" | "mcp-stdio/v1" | "mcp-http/v1" | "wasm-component/v1" | "webview/v1";
  "required": boolean;
  "entry"?: string;
  "activation": "on-demand" | "on-command" | "on-event" | "on-startup";
  "requires": Array<CapabilityRequirement>;
  "permissions": Array<string>;
  "applications"?: Array<ApplicationRequirement>;
  "args"?: Array<string>;
  "env"?: Record<string, string>;
  "endpoint"?: string;
  "profile"?: "trusted-process" | "sandboxed-process";
  "world"?: string;
  "contributions"?: Array<Contribution>;
  "limits"?: ExecutionLimits;
}
export interface Permission {
  "id": string;
  "capability": string;
  "actions": Array<string>;
  "resources": Record<string, unknown>;
  "reason": string;
}
export interface CredentialRequirement {
  "id": string;
  "kind": "oauth" | "secret";
  "scopes": Array<string>;
  "reason": string;
}
export interface Configuration {
  "schema": string;
  "credentials"?: Array<CredentialRequirement>;
}
export interface Migration {
  "fromVersion": number;
  "toVersion": number;
  "module": string;
  "reversible": boolean;
}
export interface DataContract {
  "version": number;
  "migrations"?: Array<Migration>;
}
export interface Dependency {
  "pluginId": string;
  "version": string;
  "required": boolean;
  "module"?: string;
  "exportApi"?: string;
}
export interface ExportMethod {
  "name": string;
  "inputSchema": string;
  "outputSchema": string;
  "permissions": Array<string>;
}
export interface PluginExport {
  "id": string;
  "module": string;
  "interface": string;
  "version": string;
  "methods": Array<ExportMethod>;
}
export interface Manifest {
  "schemaVersion": number;
  "plugin": PluginMeta;
  "compatibility": Compatibility;
  "modules": Array<Module>;
  "permissions"?: Array<Permission>;
  "configuration"?: Configuration;
  "data"?: DataContract;
  "dependencies"?: Array<Dependency>;
  "exports"?: Array<PluginExport>;
}
export interface Executor {
  "type": string;
  "abi": string;
  "profiles"?: Array<string>;
}
export interface Capability {
  "id": string;
  "version": string;
  "inputSchemaDigest"?: string;
  "outputSchemaDigest"?: string;
  "limits"?: ExecutionLimits;
}
export interface Platform {
  "os": string;
  "arch": string;
  "surface": "desktop" | "web" | "server" | "cli";
}
export interface UiSlot {
  "id": string;
  "kind": string;
  "version": string;
}
export interface HostDescriptor {
  "applicationId": string;
  "hostVersion": string;
  "platformApiVersion": string;
  "environmentId": string;
  "hostInstanceId": string;
  "platform": Platform;
  "executors": Array<Executor>;
  "capabilities": Array<Capability>;
  "uiSlots"?: Array<UiSlot>;
  "policyRevision": number;
}
export interface CompatibilityIssue {
  "code": string;
  "module"?: string;
  "field": string;
  "expected": string;
  "actual": string;
}
export interface CompatibilityResult {
  "status": "compatible" | "incompatible" | "authorization-required";
  "selectedModules": Array<string>;
  "unavailableModules": Array<string>;
  "issues": Array<CompatibilityIssue>;
}
export interface ArtifactTarget {
  "os": string;
  "arch": string;
  "abis": Array<string>;
}
export interface Artifact {
  "digest": string;
  "length": number;
  "manifestDigest": string;
  "filesDigest": string;
  "target": ArtifactTarget;
}
export interface Release {
  "manifest": Manifest;
  "manifestDigest": string;
  "artifacts": Array<Artifact>;
  "status": "draft" | "validated" | "approved" | "published" | "withdrawn" | "revoked";
  "revision": number;
  "publisherApproved": boolean;
}
export interface InstallationScope {
  "kind": "user" | "workspace" | "organization";
  "id": string;
}
export interface InstallPlan {
  "id": string;
  "release": Release;
  "artifact": Artifact;
  "hostDigest": string;
  "subjectId": string;
  "scope": InstallationScope;
  "selectedModules": Array<string>;
  "permissionRequests": Array<Permission>;
  "dependencies"?: Array<Release>;
  "createdAt": number;
  "expiresAt": number;
  "policyRevision": number;
  "planDigest": string;
  "configurationSchemas"?: Record<string, unknown>;
}
export interface Installation {
  "id": string;
  "pluginId": string;
  "name": string;
  "version": string;
  "artifactDigest": string;
  "manifest": Manifest;
  "applicationId": string;
  "hostInstanceId": string;
  "subjectId": string;
  "scope": InstallationScope;
  "generation": number;
  "state": string;
  "enabled": boolean;
  "selectedModules": Array<string>;
  "grants": Array<Permission>;
  "policyRevision": number;
  "leaseExpiresAt": number;
  "lastError"?: string;
  "dataVersion": number;
  "environmentId": string;
  "configuration"?: unknown;
}
export interface SearchRequest {
  "query": string;
  "host": HostDescriptor;
  "cursor"?: string;
  "limit"?: number;
}
export interface MarketEntry {
  "id": string;
  "name": string;
  "description": string;
  "version": string;
  "publisher": string;
  "release": Release;
  "compatibility": CompatibilityResult;
}
export interface SearchResult {
  "results": Array<MarketEntry>;
  "nextCursor"?: string;
  "total": number;
}
export interface ResolveRequest {
  "pluginId": string;
  "version"?: string;
  "host": HostDescriptor;
  "modules"?: Array<string>;
  "scope": InstallationScope;
}
export interface ResolveResult {
  "release": Release;
  "artifact": Artifact;
  "dependencies"?: Array<Release>;
  "policyRevision": number;
  "leaseExpiresAt": number;
  "trustRevision": number;
  "configurationSchemas"?: Record<string, unknown>;
}
export interface OperationResult {
  "installation"?: Installation;
  "plan"?: InstallPlan;
  "needsConsent": boolean;
}
export interface RuntimePolicy {
  "environmentId": string;
  "subjectId": string;
  "revision": number;
  "issuedAt": number;
  "expiresAt": number;
  "allowedDigests": Array<string>;
  "revokedDigests": Array<string>;
}
export interface DownloadAuthorization {
  "digest": string;
  "token": string;
  "expiresAt": number;
}
export interface ErrorBody {
  "code": string;
  "message": string;
  "details": unknown;
  "retryable"?: boolean;
}
export interface MarketMember {
  "userId": string;
  "applicationIds": Array<string>;
  "enabled": boolean;
}
export interface MarketRecord {
  "release": Release;
  "createdBy": string;
  "reviewedBy"?: string;
}
export interface AdminCatalog {
  "environment": string;
  "applications": Array<string>;
  "revision": number;
  "releases": Array<MarketRecord>;
  "mode": "internal";
  "trustConfigured": boolean;
}
export interface ViewPayload {
  "installationId": string;
  "moduleId": string;
  "generation": number;
  "html": string;
  "nonce": string;
}
