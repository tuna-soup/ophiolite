use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

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
    providers: BTreeMap<String, Box<dyn ImportProvider>>,
}

impl ImportProviderRegistry {
    pub fn new() -> Self {
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
            providers.insert(provider.descriptor.provider_id.clone(), Box::new(provider));
        }
        Self { providers }
    }

    pub fn descriptors(&self) -> Vec<ImportProviderDescriptor> {
        let mut descriptors = self
            .providers
            .values()
            .map(|provider| provider.descriptor())
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
        let provider = self
            .registry
            .provider(provider_id)
            .ok_or_else(|| format!("unknown import provider '{provider_id}'"))?;
        let session_id = format!(
            "import-session-{}",
            self.next_session_id.fetch_add(1, Ordering::Relaxed)
        );
        let session = provider.begin_session(session_id.clone(), &request)?;
        self.sessions
            .lock()
            .map_err(|_| "import session store is unavailable".to_string())?
            .insert(session_id, session.clone());
        Ok(session)
    }
}
