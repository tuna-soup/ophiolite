use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const CAPABILITY_REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityKind {
    Operator,
    ImportProvider,
    ChartAdapter,
    WorkflowAction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilitySource {
    BuiltIn,
    OptionalPackage,
    AppLocal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityStability {
    Internal,
    Preview,
    Stable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityLoadPolicy {
    Never,
    OnDemand,
    Startup,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityIsolation {
    InProcess,
    Worker,
    OutOfProcess,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityAvailability {
    Available,
    Deferred,
    Unavailable { reasons: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityValidationState {
    Unknown,
    Valid,
    Invalid { reasons: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityActivationState {
    Dormant,
    Active,
    Failed { reasons: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityHostRequirement {
    pub host: String,
    pub version_requirement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityContractSet {
    pub request: Option<String>,
    pub response: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityDocumentation {
    pub short_help: String,
    pub help_markdown: Option<String>,
    pub help_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilitySurfaceBinding {
    pub surface: String,
    pub binding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityArtifactRef {
    pub platform: String,
    pub path: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct OperatorCapabilityDetail {
    pub family_id: String,
    pub subject_kind: String,
    pub execution_kind: String,
    pub output_lifecycle: String,
    pub deterministic: bool,
    pub parameter_schema_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct ImportProviderCapabilityDetail {
    pub destination_kind: String,
    pub selection_mode: String,
    pub supported_extensions: Vec<String>,
    pub supports_directory: bool,
    pub supports_drag_drop: bool,
    pub supports_deep_link: bool,
    pub requires_active_store: bool,
    pub requires_project_root: bool,
    pub requires_project_well_binding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct ChartAdapterCapabilityDetail {
    pub chart_family_id: String,
    pub input_contract_id: Option<String>,
    pub output_model: String,
    pub embedding_constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct WorkflowActionCapabilityDetail {
    pub action_id: String,
    pub trigger_contexts: Vec<String>,
    pub execution_target: String,
    pub side_effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CapabilityDetail {
    Operator(OperatorCapabilityDetail),
    ImportProvider(ImportProviderCapabilityDetail),
    ChartAdapter(ChartAdapterCapabilityDetail),
    WorkflowAction(WorkflowActionCapabilityDetail),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityRecord {
    pub id: String,
    pub kind: CapabilityKind,
    pub source: CapabilitySource,
    pub provider: String,
    pub name: String,
    pub summary: Option<String>,
    pub version: Option<String>,
    pub stability: CapabilityStability,
    pub availability: CapabilityAvailability,
    pub tags: Vec<String>,
    pub documentation: Vec<CapabilityDocumentation>,
    pub load_policy: CapabilityLoadPolicy,
    pub isolation: CapabilityIsolation,
    pub permissions: Vec<String>,
    pub bindings: Vec<CapabilitySurfaceBinding>,
    pub host_compatibility: Vec<CapabilityHostRequirement>,
    pub contracts: CapabilityContractSet,
    pub artifacts: Vec<CapabilityArtifactRef>,
    pub detail: CapabilityDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityLifecycleRecord {
    pub capability_id: String,
    pub validation: CapabilityValidationState,
    pub activation: CapabilityActivationState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityRegistry {
    pub schema_version: u32,
    pub records: Vec<CapabilityRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct CapabilityLifecycleRegistry {
    pub schema_version: u32,
    pub records: Vec<CapabilityLifecycleRecord>,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self {
            schema_version: CAPABILITY_REGISTRY_SCHEMA_VERSION,
            records: Vec::new(),
        }
    }
}

impl Default for CapabilityLifecycleRegistry {
    fn default() -> Self {
        Self {
            schema_version: CAPABILITY_REGISTRY_SCHEMA_VERSION,
            records: Vec::new(),
        }
    }
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn register(&mut self, record: CapabilityRecord) {
        if let Some(index) = self
            .records
            .iter()
            .position(|candidate| candidate.id == record.id)
        {
            self.records[index] = record;
        } else {
            self.records.push(record);
        }
    }

    pub fn extend<I>(&mut self, records: I)
    where
        I: IntoIterator<Item = CapabilityRecord>,
    {
        for record in records {
            self.register(record);
        }
    }

    pub fn get(&self, id: &str) -> Option<&CapabilityRecord> {
        self.records.iter().find(|record| record.id == id)
    }

    pub fn list_by_kind(&self, kind: CapabilityKind) -> Vec<&CapabilityRecord> {
        self.records
            .iter()
            .filter(|record| record.kind == kind)
            .collect()
    }

    pub fn list_available(&self) -> Vec<&CapabilityRecord> {
        self.records
            .iter()
            .filter(|record| {
                !matches!(
                    record.availability,
                    CapabilityAvailability::Unavailable { .. }
                )
            })
            .collect()
    }
}

impl CapabilityLifecycleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn discover(&mut self, capability_id: impl Into<String>) {
        self.ensure_record(capability_id.into());
    }

    pub fn get(&self, capability_id: &str) -> Option<&CapabilityLifecycleRecord> {
        self.records
            .iter()
            .find(|record| record.capability_id == capability_id)
    }

    pub fn record_validation(
        &mut self,
        capability_id: impl Into<String>,
        reasons: Vec<String>,
    ) -> CapabilityValidationState {
        let record = self.ensure_record(capability_id.into());
        record.validation = if reasons.is_empty() {
            CapabilityValidationState::Valid
        } else {
            CapabilityValidationState::Invalid { reasons }
        };
        record.validation.clone()
    }

    pub fn mark_active(&mut self, capability_id: impl Into<String>) -> Result<(), String> {
        let capability_id = capability_id.into();
        let record = self.ensure_record(capability_id.clone());
        match &record.validation {
            CapabilityValidationState::Unknown => Err(format!(
                "capability `{capability_id}` must be validated before activation"
            )),
            CapabilityValidationState::Invalid { reasons } => Err(format!(
                "capability `{capability_id}` cannot activate: {}",
                reasons.join("; ")
            )),
            CapabilityValidationState::Valid => {
                record.activation = CapabilityActivationState::Active;
                Ok(())
            }
        }
    }

    pub fn mark_dormant(&mut self, capability_id: impl Into<String>) {
        self.ensure_record(capability_id.into()).activation = CapabilityActivationState::Dormant;
    }

    pub fn mark_activation_failed(
        &mut self,
        capability_id: impl Into<String>,
        reasons: Vec<String>,
    ) {
        self.ensure_record(capability_id.into()).activation =
            CapabilityActivationState::Failed { reasons };
    }

    fn ensure_record(&mut self, capability_id: String) -> &mut CapabilityLifecycleRecord {
        if let Some(index) = self
            .records
            .iter()
            .position(|record| record.capability_id == capability_id)
        {
            &mut self.records[index]
        } else {
            self.records.push(CapabilityLifecycleRecord {
                capability_id,
                validation: CapabilityValidationState::Unknown,
                activation: CapabilityActivationState::Dormant,
            });
            self.records
                .last_mut()
                .expect("lifecycle registry record inserted")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_operator(id: &str, name: &str) -> CapabilityRecord {
        CapabilityRecord {
            id: id.to_string(),
            kind: CapabilityKind::Operator,
            source: CapabilitySource::BuiltIn,
            provider: "ophiolite".to_string(),
            name: name.to_string(),
            summary: Some("Sample operator".to_string()),
            version: Some("0.1.0".to_string()),
            stability: CapabilityStability::Preview,
            availability: CapabilityAvailability::Available,
            tags: vec!["sample".to_string()],
            documentation: vec![CapabilityDocumentation {
                short_help: "sample".to_string(),
                help_markdown: None,
                help_url: None,
            }],
            load_policy: CapabilityLoadPolicy::OnDemand,
            isolation: CapabilityIsolation::InProcess,
            permissions: vec!["read_project".to_string()],
            bindings: vec![CapabilitySurfaceBinding {
                surface: "rust".to_string(),
                binding: "ophiolite_sdk".to_string(),
            }],
            host_compatibility: vec![CapabilityHostRequirement {
                host: "ophiolite".to_string(),
                version_requirement: ">=0.1 <0.2".to_string(),
            }],
            contracts: CapabilityContractSet {
                request: Some("request.contract".to_string()),
                response: Some("response.contract".to_string()),
            },
            artifacts: vec![],
            detail: CapabilityDetail::Operator(OperatorCapabilityDetail {
                family_id: "trace_local".to_string(),
                subject_kind: "seismic_trace_data".to_string(),
                execution_kind: "job".to_string(),
                output_lifecycle: "derived_asset".to_string(),
                deterministic: true,
                parameter_schema_id: None,
            }),
        }
    }

    #[test]
    fn registry_replaces_duplicate_ids() {
        let mut registry = CapabilityRegistry::new();
        registry.register(sample_operator("ophiolite.sample", "First"));
        registry.register(sample_operator("ophiolite.sample", "Second"));

        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry
                .get("ophiolite.sample")
                .map(|record| record.name.as_str()),
            Some("Second")
        );
    }

    #[test]
    fn registry_filters_by_kind_and_availability() {
        let mut registry = CapabilityRegistry::new();
        registry.register(sample_operator("ophiolite.available", "Available"));

        let mut unavailable = sample_operator("ophiolite.unavailable", "Unavailable");
        unavailable.availability = CapabilityAvailability::Unavailable {
            reasons: vec!["not installed".to_string()],
        };
        registry.register(unavailable);

        assert_eq!(registry.list_by_kind(CapabilityKind::Operator).len(), 2);
        assert_eq!(registry.list_available().len(), 1);
        assert_eq!(
            registry
                .list_available()
                .first()
                .map(|record| record.id.as_str()),
            Some("ophiolite.available")
        );
    }

    #[test]
    fn lifecycle_requires_validation_before_activation() {
        let mut lifecycle = CapabilityLifecycleRegistry::new();
        lifecycle.discover("ophiolite.sample");

        let error = lifecycle
            .mark_active("ophiolite.sample")
            .expect_err("activation should fail before validation");
        assert!(error.contains("must be validated"));

        let validation = lifecycle.record_validation("ophiolite.sample", Vec::new());
        assert_eq!(validation, CapabilityValidationState::Valid);

        lifecycle
            .mark_active("ophiolite.sample")
            .expect("valid capability should activate");
        assert_eq!(
            lifecycle
                .get("ophiolite.sample")
                .map(|record| &record.activation),
            Some(&CapabilityActivationState::Active)
        );

        lifecycle.mark_dormant("ophiolite.sample");
        assert_eq!(
            lifecycle
                .get("ophiolite.sample")
                .map(|record| &record.activation),
            Some(&CapabilityActivationState::Dormant)
        );
    }

    #[test]
    fn lifecycle_records_invalid_and_failed_capabilities() {
        let mut lifecycle = CapabilityLifecycleRegistry::new();
        lifecycle.discover("ophiolite.unavailable");
        let validation =
            lifecycle.record_validation("ophiolite.unavailable", vec!["not installed".to_string()]);

        assert_eq!(
            validation,
            CapabilityValidationState::Invalid {
                reasons: vec!["not installed".to_string()]
            }
        );
        let error = lifecycle
            .mark_active("ophiolite.unavailable")
            .expect_err("invalid capability should not activate");
        assert!(error.contains("not installed"));

        lifecycle.mark_activation_failed(
            "ophiolite.unavailable",
            vec!["activation failed".to_string()],
        );
        assert_eq!(
            lifecycle
                .get("ophiolite.unavailable")
                .map(|record| &record.activation),
            Some(&CapabilityActivationState::Failed {
                reasons: vec!["activation failed".to_string()]
            })
        );
    }
}
