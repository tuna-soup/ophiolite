use ophiolite_capabilities::{
    CapabilityAvailability, CapabilityContractSet, CapabilityDetail, CapabilityDocumentation,
    CapabilityIsolation, CapabilityKind, CapabilityLifecycleRegistry, CapabilityLoadPolicy,
    CapabilityRecord, CapabilityRegistry, CapabilitySource, CapabilityStability,
    CapabilitySurfaceBinding, ImportProviderCapabilityDetail,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const TRACEBOOST_IMPORT_PROVIDER_CAPABILITY_PROVIDER: &str = "traceboost-desktop";
const IMPORT_PROVIDER_ICON_ID_BINDING: &str = "traceboost.import_provider.icon_id";
const IMPORT_PROVIDER_GROUP_BINDING: &str = "traceboost.import_provider.group";
const IMPORT_PROVIDER_ORDERING_BINDING: &str = "traceboost.import_provider.ordering";
const IMPORT_PROVIDER_IMPLEMENTED_BINDING: &str = "traceboost.import_provider.implemented";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProviderDescriptor {
    pub provider_id: String,
    pub label: String,
    pub description: String,
    pub icon_id: String,
    pub group: String,
    pub ordering: u32,
    pub destination_kind: String,
    pub selection_mode: String,
    pub supported_extensions: Vec<String>,
    pub supports_directory: bool,
    pub requires_active_store: bool,
    pub requires_project_root: bool,
    pub requires_project_well_binding: bool,
    pub supports_drag_drop: bool,
    pub supports_deep_link: bool,
    pub implemented: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionDiagnostic {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionEnvelope {
    pub session_id: String,
    pub provider_id: String,
    pub source_refs: Vec<String>,
    pub destination_kind: String,
    pub destination_ref: Option<String>,
    pub activation_intent: String,
    pub status: String,
    pub diagnostics: Vec<ImportSessionDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListImportProvidersResponse {
    pub providers: Vec<ImportProviderDescriptor>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginImportSessionRequest {
    pub provider_id: String,
    pub source_refs: Option<Vec<String>>,
    pub destination_ref: Option<String>,
    pub activation_intent: Option<String>,
}

pub trait ImportProvider: Send + Sync {
    fn descriptor(&self) -> ImportProviderDescriptor;

    fn begin_session(
        &self,
        session_id: String,
        request: &BeginImportSessionRequest,
    ) -> Result<ImportSessionEnvelope, String>;
}

#[derive(Debug)]
struct StaticImportProvider {
    descriptor: ImportProviderDescriptor,
}

impl StaticImportProvider {
    fn new(
        provider_id: &str,
        label: &str,
        description: &str,
        icon_id: &str,
        group: &str,
        ordering: u32,
        destination_kind: &str,
        selection_mode: &str,
        supported_extensions: &[&str],
        supports_directory: bool,
        requires_active_store: bool,
        requires_project_root: bool,
        requires_project_well_binding: bool,
    ) -> Self {
        Self {
            descriptor: ImportProviderDescriptor {
                provider_id: provider_id.to_string(),
                label: label.to_string(),
                description: description.to_string(),
                icon_id: icon_id.to_string(),
                group: group.to_string(),
                ordering,
                destination_kind: destination_kind.to_string(),
                selection_mode: selection_mode.to_string(),
                supported_extensions: supported_extensions
                    .iter()
                    .map(|value| value.to_string())
                    .collect(),
                supports_directory,
                requires_active_store,
                requires_project_root,
                requires_project_well_binding,
                supports_drag_drop: true,
                supports_deep_link: true,
                implemented: true,
            },
        }
    }
}

impl ImportProvider for StaticImportProvider {
    fn descriptor(&self) -> ImportProviderDescriptor {
        self.descriptor.clone()
    }

    fn begin_session(
        &self,
        session_id: String,
        request: &BeginImportSessionRequest,
    ) -> Result<ImportSessionEnvelope, String> {
        let source_refs = request
            .source_refs
            .as_ref()
            .map(|values| {
                values
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut diagnostics = Vec::new();
        if source_refs.is_empty() {
            diagnostics.push(ImportSessionDiagnostic {
                level: "info".to_string(),
                message: format!(
                    "Choose source data to begin the {} flow.",
                    self.descriptor.label
                ),
            });
        } else {
            diagnostics.push(ImportSessionDiagnostic {
                level: "info".to_string(),
                message: format!(
                    "Prepared {} source reference{} for {}.",
                    source_refs.len(),
                    if source_refs.len() == 1 { "" } else { "s" },
                    self.descriptor.label
                ),
            });
        }

        Ok(ImportSessionEnvelope {
            session_id,
            provider_id: self.descriptor.provider_id.clone(),
            source_refs,
            destination_kind: self.descriptor.destination_kind.clone(),
            destination_ref: request
                .destination_ref
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string()),
            activation_intent: request
                .activation_intent
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .unwrap_or("open_after_commit")
                .to_string(),
            status: "initialized".to_string(),
            diagnostics,
        })
    }
}

pub struct ImportProviderRegistry {
    capabilities: CapabilityRegistry,
    lifecycle: Mutex<CapabilityLifecycleRegistry>,
    providers: BTreeMap<String, Box<dyn ImportProvider>>,
}

impl ImportProviderRegistry {
    pub fn new() -> Self {
        let mut capabilities = CapabilityRegistry::new();
        let mut lifecycle = CapabilityLifecycleRegistry::new();
        let mut providers: BTreeMap<String, Box<dyn ImportProvider>> = BTreeMap::new();
        for provider in [
            StaticImportProvider::new(
                "authored_model",
                "Authored Model",
                "Preview a well time-depth authored model JSON and commit it into the selected project well context.",
                "time_depth_model",
                "well_time_depth",
                70,
                "project_asset",
                "single_file",
                &["json"],
                false,
                false,
                true,
                true,
            ),
            StaticImportProvider::new(
                "checkshot_vsp",
                "Checkshot/VSP",
                "Preview a checkshot or VSP observation-set JSON and commit it into the selected project well context.",
                "checkshot",
                "well_time_depth",
                50,
                "project_asset",
                "single_file",
                &["json"],
                false,
                false,
                true,
                true,
            ),
            StaticImportProvider::new(
                "compiled_model",
                "Compiled Model",
                "Preview a compiled well time-depth model JSON and commit it into the selected project well context.",
                "compiled_model",
                "well_time_depth",
                80,
                "project_asset",
                "single_file",
                &["json"],
                false,
                false,
                true,
                true,
            ),
            StaticImportProvider::new(
                "horizons",
                "Horizons",
                "Parse horizon XYZ files, confirm the CRS path, and commit them into the active survey store.",
                "horizon",
                "seismic",
                20,
                "runtime_store",
                "multi_file",
                &["xyz"],
                false,
                true,
                false,
                false,
            ),
            StaticImportProvider::new(
                "seismic_volume",
                "Seismic Volume",
                "Inspect and import SEG-Y, Zarr, or MDIO seismic sources into a managed runtime store.",
                "seismic_volume",
                "seismic",
                10,
                "runtime_store",
                "single_file",
                &["sgy", "segy", "zarr", "mdio"],
                false,
                false,
                false,
                false,
            ),
            StaticImportProvider::new(
                "well_sources",
                "Well Sources",
                "Preview selected well files, confirm the canonical draft, and commit the defensible slices into project storage.",
                "well_sources",
                "wells",
                40,
                "project_asset",
                "multi_file",
                &["las", "asc", "txt", "csv", "dlis"],
                false,
                false,
                true,
                false,
            ),
            StaticImportProvider::new(
                "manual_picks",
                "Manual Picks",
                "Preview a manual time-depth picks JSON and commit it into the selected project well context.",
                "manual_picks",
                "well_time_depth",
                60,
                "project_asset",
                "single_file",
                &["json"],
                false,
                false,
                true,
                true,
            ),
            StaticImportProvider::new(
                "vendor_project",
                "Vendor Project",
                "Scan an external vendor project export, plan the canonical translation, and commit the selected assets into the active project.",
                "vendor_project",
                "projects",
                90,
                "project_asset",
                "directory",
                &[],
                true,
                false,
                true,
                false,
            ),
            StaticImportProvider::new(
                "velocity_functions",
                "Velocity Functions",
                "Import sparse interval or RMS velocity functions into the active seismic volume and compile a survey velocity model.",
                "velocity_functions",
                "seismic",
                30,
                "runtime_store",
                "single_file",
                &["txt", "csv"],
                false,
                true,
                false,
                false,
            ),
        ] {
            let descriptor = provider.descriptor();
            let capability = capability_record_from_descriptor(&descriptor);
            lifecycle.discover(capability.id.clone());
            lifecycle.record_validation(
                capability.id.clone(),
                validate_import_provider_capability(&capability, true),
            );
            capabilities.register(capability);
            providers.insert(descriptor.provider_id.clone(), Box::new(provider));
        }
        Self {
            capabilities,
            lifecycle: Mutex::new(lifecycle),
            providers,
        }
    }

    pub fn descriptors(&self) -> Vec<ImportProviderDescriptor> {
        let mut descriptors = self
            .capabilities
            .list_by_kind(CapabilityKind::ImportProvider)
            .into_iter()
            .filter(|record| {
                !matches!(
                    record.availability,
                    CapabilityAvailability::Unavailable { .. }
                )
            })
            .filter_map(import_provider_descriptor_from_capability)
            .collect::<Vec<_>>();
        descriptors.sort_by(|left, right| {
            left.ordering
                .cmp(&right.ordering)
                .then_with(|| left.label.cmp(&right.label))
        });
        descriptors
    }

    pub fn provider(&self, provider_id: &str) -> Option<&dyn ImportProvider> {
        self.providers
            .get(provider_id)
            .map(|provider| provider.as_ref())
    }

    pub fn prepare_activation(&self, provider_id: &str) -> Result<(), String> {
        let Some(capability) = self.capabilities.get(provider_id) else {
            return Err(format!(
                "import capability `{provider_id}` is not registered for discovery"
            ));
        };
        let reasons = validate_import_provider_capability(
            capability,
            self.providers.contains_key(provider_id),
        );
        let mut lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| "import capability lifecycle is unavailable".to_string())?;
        lifecycle.record_validation(provider_id.to_string(), reasons.clone());
        if !reasons.is_empty() {
            lifecycle.mark_activation_failed(provider_id.to_string(), reasons.clone());
            return Err(format!(
                "import provider `{provider_id}` cannot activate: {}",
                reasons.join("; ")
            ));
        }
        lifecycle.mark_active(provider_id.to_string())
    }

    pub fn finish_activation(&self, provider_id: &str) {
        if let Ok(mut lifecycle) = self.lifecycle.lock() {
            lifecycle.mark_dormant(provider_id.to_string());
        }
    }

    pub fn fail_activation(&self, provider_id: &str, reason: String) {
        if let Ok(mut lifecycle) = self.lifecycle.lock() {
            lifecycle.mark_activation_failed(provider_id.to_string(), vec![reason]);
        }
    }

    #[cfg(test)]
    fn from_parts(
        providers: BTreeMap<String, Box<dyn ImportProvider>>,
        capabilities: CapabilityRegistry,
    ) -> Self {
        let mut lifecycle = CapabilityLifecycleRegistry::new();
        for record in &capabilities.records {
            lifecycle.discover(record.id.clone());
            lifecycle.record_validation(
                record.id.clone(),
                validate_import_provider_capability(
                    record,
                    providers.contains_key(record.id.as_str()),
                ),
            );
        }
        Self {
            capabilities,
            lifecycle: Mutex::new(lifecycle),
            providers,
        }
    }

    #[cfg(test)]
    fn lifecycle_activation_state(&self, provider_id: &str) -> Option<String> {
        self.lifecycle
            .lock()
            .ok()
            .and_then(|lifecycle| lifecycle.get(provider_id).cloned())
            .map(|record| match record.activation {
                ophiolite_capabilities::CapabilityActivationState::Dormant => "dormant".to_string(),
                ophiolite_capabilities::CapabilityActivationState::Active => "active".to_string(),
                ophiolite_capabilities::CapabilityActivationState::Failed { .. } => {
                    "failed".to_string()
                }
            })
    }
}

pub struct ImportManagerState {
    registry: ImportProviderRegistry,
    sessions: Mutex<HashMap<String, ImportSessionEnvelope>>,
    next_session_id: AtomicU64,
}

impl ImportManagerState {
    pub fn initialize() -> Self {
        Self {
            registry: ImportProviderRegistry::new(),
            sessions: Mutex::new(HashMap::new()),
            next_session_id: AtomicU64::new(1),
        }
    }

    pub fn list_providers(&self) -> ListImportProvidersResponse {
        ListImportProvidersResponse {
            providers: self.registry.descriptors(),
        }
    }

    pub fn begin_session(
        &self,
        request: BeginImportSessionRequest,
    ) -> Result<ImportSessionEnvelope, String> {
        let provider_id = request.provider_id.trim();
        if provider_id.is_empty() {
            return Err("import session requires a provider id".to_string());
        }
        self.registry.prepare_activation(provider_id)?;
        let provider = self
            .registry
            .provider(provider_id)
            .ok_or_else(|| format!("unknown import provider '{provider_id}'"))?;
        let session_id = format!(
            "import-session-{}",
            self.next_session_id.fetch_add(1, Ordering::Relaxed)
        );
        let session = match provider.begin_session(session_id.clone(), &request) {
            Ok(session) => {
                self.registry.finish_activation(provider_id);
                session
            }
            Err(error) => {
                self.registry.fail_activation(provider_id, error.clone());
                return Err(error);
            }
        };
        self.sessions
            .lock()
            .map_err(|_| "import session store is unavailable".to_string())?
            .insert(session_id, session.clone());
        Ok(session)
    }

    #[cfg(test)]
    fn with_registry(registry: ImportProviderRegistry) -> Self {
        Self {
            registry,
            sessions: Mutex::new(HashMap::new()),
            next_session_id: AtomicU64::new(1),
        }
    }
}

fn capability_record_from_descriptor(descriptor: &ImportProviderDescriptor) -> CapabilityRecord {
    CapabilityRecord {
        id: descriptor.provider_id.clone(),
        kind: CapabilityKind::ImportProvider,
        source: CapabilitySource::AppLocal,
        provider: TRACEBOOST_IMPORT_PROVIDER_CAPABILITY_PROVIDER.to_string(),
        name: descriptor.label.clone(),
        summary: Some(descriptor.description.clone()),
        version: None,
        stability: CapabilityStability::Stable,
        availability: if descriptor.implemented {
            CapabilityAvailability::Available
        } else {
            CapabilityAvailability::Deferred
        },
        tags: vec![descriptor.group.clone()],
        documentation: vec![CapabilityDocumentation {
            short_help: descriptor.description.clone(),
            help_markdown: None,
            help_url: None,
        }],
        load_policy: CapabilityLoadPolicy::OnDemand,
        isolation: CapabilityIsolation::InProcess,
        permissions: vec![],
        bindings: vec![
            CapabilitySurfaceBinding {
                surface: IMPORT_PROVIDER_ICON_ID_BINDING.to_string(),
                binding: descriptor.icon_id.clone(),
            },
            CapabilitySurfaceBinding {
                surface: IMPORT_PROVIDER_GROUP_BINDING.to_string(),
                binding: descriptor.group.clone(),
            },
            CapabilitySurfaceBinding {
                surface: IMPORT_PROVIDER_ORDERING_BINDING.to_string(),
                binding: descriptor.ordering.to_string(),
            },
            CapabilitySurfaceBinding {
                surface: IMPORT_PROVIDER_IMPLEMENTED_BINDING.to_string(),
                binding: descriptor.implemented.to_string(),
            },
        ],
        host_compatibility: vec![],
        contracts: CapabilityContractSet {
            request: None,
            response: None,
        },
        artifacts: vec![],
        detail: CapabilityDetail::ImportProvider(ImportProviderCapabilityDetail {
            destination_kind: descriptor.destination_kind.clone(),
            selection_mode: descriptor.selection_mode.clone(),
            supported_extensions: descriptor.supported_extensions.clone(),
            supports_directory: descriptor.supports_directory,
            supports_drag_drop: descriptor.supports_drag_drop,
            supports_deep_link: descriptor.supports_deep_link,
            requires_active_store: descriptor.requires_active_store,
            requires_project_root: descriptor.requires_project_root,
            requires_project_well_binding: descriptor.requires_project_well_binding,
        }),
    }
}

fn import_provider_descriptor_from_capability(
    record: &CapabilityRecord,
) -> Option<ImportProviderDescriptor> {
    let CapabilityDetail::ImportProvider(detail) = &record.detail else {
        return None;
    };

    Some(ImportProviderDescriptor {
        provider_id: record.id.clone(),
        label: record.name.clone(),
        description: record
            .summary
            .clone()
            .or_else(|| {
                record
                    .documentation
                    .first()
                    .map(|documentation| documentation.short_help.clone())
            })
            .unwrap_or_default(),
        icon_id: capability_binding(record, IMPORT_PROVIDER_ICON_ID_BINDING)
            .unwrap_or(record.id.as_str())
            .to_string(),
        group: capability_binding(record, IMPORT_PROVIDER_GROUP_BINDING)
            .or_else(|| record.tags.first().map(String::as_str))
            .unwrap_or_default()
            .to_string(),
        ordering: capability_u32_binding(record, IMPORT_PROVIDER_ORDERING_BINDING).unwrap_or(0),
        destination_kind: detail.destination_kind.clone(),
        selection_mode: detail.selection_mode.clone(),
        supported_extensions: detail.supported_extensions.clone(),
        supports_directory: detail.supports_directory,
        requires_active_store: detail.requires_active_store,
        requires_project_root: detail.requires_project_root,
        requires_project_well_binding: detail.requires_project_well_binding,
        supports_drag_drop: detail.supports_drag_drop,
        supports_deep_link: detail.supports_deep_link,
        implemented: capability_bool_binding(record, IMPORT_PROVIDER_IMPLEMENTED_BINDING)
            .unwrap_or(matches!(
                record.availability,
                CapabilityAvailability::Available
            )),
    })
}

fn capability_binding<'a>(record: &'a CapabilityRecord, surface: &str) -> Option<&'a str> {
    record
        .bindings
        .iter()
        .find(|binding| binding.surface == surface)
        .map(|binding| binding.binding.as_str())
}

fn capability_u32_binding(record: &CapabilityRecord, surface: &str) -> Option<u32> {
    capability_binding(record, surface)?.parse().ok()
}

fn capability_bool_binding(record: &CapabilityRecord, surface: &str) -> Option<bool> {
    match capability_binding(record, surface)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn validate_import_provider_capability(
    record: &CapabilityRecord,
    has_provider_implementation: bool,
) -> Vec<String> {
    let mut reasons = match &record.availability {
        CapabilityAvailability::Available => Vec::new(),
        CapabilityAvailability::Deferred => vec![
            "capability is discovered but deferred and is not ready for activation".to_string(),
        ],
        CapabilityAvailability::Unavailable { reasons } => reasons.clone(),
    };
    if !capability_bool_binding(record, IMPORT_PROVIDER_IMPLEMENTED_BINDING).unwrap_or(matches!(
        record.availability,
        CapabilityAvailability::Available
    )) {
        reasons.push(
            "capability is discovery-only and does not currently expose an activation implementation"
                .to_string(),
        );
    }
    if !has_provider_implementation {
        reasons.push("no provider implementation is registered for this capability id".to_string());
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_comes_from_capabilities_while_activation_uses_provider_implementations() {
        let mut providers: BTreeMap<String, Box<dyn ImportProvider>> = BTreeMap::new();
        providers.insert(
            "shared_provider".to_string(),
            Box::new(StaticImportProvider::new(
                "shared_provider",
                "Implementation Provider",
                "Backs activation in this test.",
                "impl_icon",
                "impl_group",
                5,
                "project_asset",
                "single_file",
                &["json"],
                false,
                false,
                false,
                false,
            )),
        );

        let discovery_descriptor = ImportProviderDescriptor {
            provider_id: "shared_provider".to_string(),
            label: "Discovery Provider".to_string(),
            description: "Backs listing only in this test.".to_string(),
            icon_id: "discovery_icon".to_string(),
            group: "discovery_group".to_string(),
            ordering: 15,
            destination_kind: "runtime_store".to_string(),
            selection_mode: "directory".to_string(),
            supported_extensions: vec!["sgy".to_string(), "zarr".to_string()],
            supports_directory: true,
            requires_active_store: true,
            requires_project_root: false,
            requires_project_well_binding: false,
            supports_drag_drop: false,
            supports_deep_link: true,
            implemented: true,
        };
        let mut capabilities = CapabilityRegistry::new();
        capabilities.register(capability_record_from_descriptor(&discovery_descriptor));

        let state = ImportManagerState::with_registry(ImportProviderRegistry::from_parts(
            providers,
            capabilities,
        ));

        let providers = state.list_providers().providers;
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider_id, "shared_provider");
        assert_eq!(providers[0].label, "Discovery Provider");
        assert_eq!(providers[0].icon_id, "discovery_icon");
        assert_eq!(providers[0].group, "discovery_group");
        assert_eq!(providers[0].ordering, 15);

        let session = state
            .begin_session(BeginImportSessionRequest {
                provider_id: "shared_provider".to_string(),
                source_refs: Some(vec!["/tmp/source.json".to_string()]),
                destination_ref: None,
                activation_intent: None,
            })
            .expect("activation should still resolve through provider implementations");
        assert_eq!(session.provider_id, "shared_provider");
        assert_eq!(session.destination_kind, "project_asset");
        assert_eq!(
            state
                .registry
                .lifecycle_activation_state("shared_provider")
                .as_deref(),
            Some("dormant")
        );
    }

    #[test]
    fn discovery_only_capability_cannot_activate() {
        let providers: BTreeMap<String, Box<dyn ImportProvider>> = BTreeMap::new();
        let discovery_descriptor = ImportProviderDescriptor {
            provider_id: "discovery_only_provider".to_string(),
            label: "Discovery Only Provider".to_string(),
            description: "Visible for discovery but not activation.".to_string(),
            icon_id: "discovery_only_icon".to_string(),
            group: "discovery_group".to_string(),
            ordering: 15,
            destination_kind: "runtime_store".to_string(),
            selection_mode: "directory".to_string(),
            supported_extensions: vec!["sgy".to_string()],
            supports_directory: true,
            requires_active_store: true,
            requires_project_root: false,
            requires_project_well_binding: false,
            supports_drag_drop: false,
            supports_deep_link: true,
            implemented: false,
        };
        let mut capabilities = CapabilityRegistry::new();
        capabilities.register(capability_record_from_descriptor(&discovery_descriptor));

        let state = ImportManagerState::with_registry(ImportProviderRegistry::from_parts(
            providers,
            capabilities,
        ));

        let error = state
            .begin_session(BeginImportSessionRequest {
                provider_id: "discovery_only_provider".to_string(),
                source_refs: Some(vec!["/tmp/source.json".to_string()]),
                destination_ref: None,
                activation_intent: None,
            })
            .expect_err("discovery-only capability should fail activation");
        assert!(error.contains("discovery-only"));
        assert_eq!(
            state
                .registry
                .lifecycle_activation_state("discovery_only_provider")
                .as_deref(),
            Some("failed")
        );
    }
}
