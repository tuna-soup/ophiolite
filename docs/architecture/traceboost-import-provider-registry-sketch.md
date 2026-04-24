# TraceBoost Import Provider Registry Sketch

This document sketches the first app-local backend boundary for the unified import architecture
accepted in `ADR-0029`.

## Scope

This is intentionally a TraceBoost app-layer sketch, not a shared `ophiolite` crate API or a public platform command surface.

The goal is to standardize:

- orchestration
- session state
- normalized blockers and results
- provider registration

With the introduction of `crates/ophiolite-capabilities`, provider discovery should now be read as:

- shared capability vocabulary and discovery record shape are platform-owned
- provider activation, session lifecycle, and app transport remain TraceBoost-owned

The goal is not to replace:

- `SegyImportPlan`
- `HorizonSourceImportCanonicalDraft`
- `ProjectWellSourceImportCanonicalDraft`
- `ProjectWellTimeDepthImportCanonicalDraft`
- any other domain-owned canonical payload

## Proposed backend shape

```rust
pub struct ImportManagerState {
    providers: ImportProviderRegistry,
    sessions: ImportSessionStore,
}

pub struct ImportProviderRegistry {
    capabilities: CapabilityRegistry,
    providers: BTreeMap<ImportProviderId, Box<dyn ImportProvider>>,
}

pub trait ImportProvider: Send + Sync {
    fn id(&self) -> ImportProviderId;
    fn descriptor(&self) -> ImportProviderDescriptor;
    fn begin_session(&self, request: BeginImportSessionRequest) -> Result<BeginImportSessionResponse, ImportError>;
    fn inspect(&self, request: ImportInspectRequest) -> Result<ImportInspectEnvelope, ImportError>;
    fn preview(&self, request: ImportPreviewRequest) -> Result<ImportPreviewEnvelope, ImportError>;
    fn validate(&self, request: ImportValidateRequest) -> Result<ImportValidationEnvelope, ImportError>;
    fn commit(&self, request: ImportCommitRequest) -> Result<ImportCommitEnvelope, ImportError>;
    fn load_recipes(&self, request: ImportRecipeLoadRequest) -> Result<ImportRecipeLoadResponse, ImportError>;
    fn save_recipe(&self, request: ImportRecipeSaveRequest) -> Result<ImportRecipeSaveResponse, ImportError>;
}
```

## Provider ids

Recommended provider ids:

- `seismic_volume`
- `horizons`
- `well_sources`
- `velocity_functions`
- `checkshot_vsp`
- `manual_time_depth_picks`
- `well_time_depth_authored_model`
- `well_time_depth_model`
- `vendor_project`

## Shared session envelope

Recommended normalized session metadata:

```rust
pub struct ImportSessionEnvelope {
    pub session_id: String,
    pub provider_id: ImportProviderId,
    pub source_refs: Vec<ImportSourceRef>,
    pub destination_kind: ImportDestinationKind,
    pub destination_ref: Option<ImportDestinationRef>,
    pub activation_intent: ImportActivationIntent,
    pub status: ImportSessionStatus,
    pub diagnostics: Vec<ImportDiagnostic>,
}
```

Recommended destination kinds:

- `runtime_store`
- `project_asset`
- `project_archive`

Recommended status values:

- `initialized`
- `inspected`
- `preview_ready`
- `draft_edited`
- `validated`
- `committed`
- `failed`

## Shared request and response envelopes

The common layer should carry provider-neutral metadata plus typed provider payloads.

Suggested pattern:

```rust
pub struct ImportInspectRequest {
    pub session: ImportSessionEnvelope,
    pub provider_input: serde_json::Value,
}

pub struct ImportPreviewRequest {
    pub session: ImportSessionEnvelope,
    pub provider_input: serde_json::Value,
}

pub struct ImportValidateRequest {
    pub session: ImportSessionEnvelope,
    pub provider_input: serde_json::Value,
}

pub struct ImportCommitRequest {
    pub session: ImportSessionEnvelope,
    pub provider_input: serde_json::Value,
    pub conflict_policies: Vec<ImportConflictSelection>,
    pub activation_policy: ImportActivationPolicy,
}
```

The outer manager may use `serde_json::Value` or an internally tagged enum at the app boundary.
The important constraint is that typed provider payloads remain provider-owned once control reaches
the provider implementation.

## Validation model

Validation must distinguish:

- `previewable`
- `committable`
- `activatable`

Recommended blocker shape:

```rust
pub enum ImportBlockerKind {
    IdentityMissing,
    GeometryUnresolved,
    CrsUnresolved,
    MappingIncomplete,
    DestinationUnavailable,
    ConflictRequiresConfirmation,
}

pub struct ImportBlocker {
    pub kind: ImportBlockerKind,
    pub code: String,
    pub message: String,
    pub target_ref: Option<String>,
}
```

This lets the frontend render one consistent review surface across providers.

## Conflict model

Conflicts must be structured, not string-only.

```rust
pub enum ImportConflictPolicy {
    CreateNew,
    ReuseExisting,
    ReplaceExisting,
    MergeAppend,
    MergePatch,
    Skip,
}

pub struct ImportConflict {
    pub conflict_kind: String,
    pub target_ref: String,
    pub default_policy: ImportConflictPolicy,
    pub allowed_policies: Vec<ImportConflictPolicy>,
    pub requires_confirmation: bool,
}

pub struct ImportConflictSelection {
    pub target_ref: String,
    pub selected_policy: ImportConflictPolicy,
}
```

## Normalized result model

Recommended normalized result:

```rust
pub enum ImportOutcome {
    PreviewOnly,
    SourceOnlyCommitted,
    PartialCanonicalCommit,
    CanonicalCommit,
    CommitFailed,
}

pub struct ImportCommitEnvelope {
    pub session: ImportSessionEnvelope,
    pub outcome: ImportOutcome,
    pub previewable: bool,
    pub committable: bool,
    pub activatable: bool,
    pub canonical_assets: Vec<ImportedCanonicalAssetSummary>,
    pub preserved_sources: Vec<PreservedSourceSummary>,
    pub dropped_items: Vec<DroppedImportItem>,
    pub blockers: Vec<ImportBlocker>,
    pub warnings: Vec<ImportWarning>,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub activation_effects: Vec<ImportActivationEffect>,
    pub refresh_scopes: Vec<ImportRefreshScope>,
    pub provider_detail: serde_json::Value,
}
```

Recommended refresh scopes:

- `workspace_registry`
- `active_dataset`
- `survey_map`
- `imported_horizons`
- `velocity_models`
- `project_inventory`
- `project_survey_horizons`
- `project_well_time_depth`
- `project_residuals`

## Provider responsibilities

Each provider implementation should own:

- source-specific inspection/parsing
- recipe application
- suggested canonical draft generation
- provider-specific validation
- provider-specific commit execution
- provider-specific detail payloads for preview and commit

The common manager should own:

- session creation and lookup
- provider discovery
- normalized diagnostics aggregation
- normalized blockers and conflicts rendering inputs
- normalized outcome reporting
- dispatch of post-commit refresh scopes

## Existing TraceBoost mapping

The current code already suggests clear initial provider ownership:

- `seismic_volume`
  - wraps `preflight_import_command`, `scan_segy_import_command`,
    `validate_segy_import_plan_command`, `import_segy_with_plan_command`,
    `import_dataset_command`, and `open_dataset_command`
- `horizons`
  - wraps `preview_horizon_xyz_import_command`, `preview_horizon_source_import_command`,
    `commit_horizon_source_import_command`, and `import_horizon_xyz_command`
- `well_sources`
  - wraps `preview_project_well_import_command` and `commit_project_well_sources_command`
- `velocity_functions`
  - wraps `import_velocity_functions_model_command`
- `checkshot_vsp`, `manual_time_depth_picks`, `well_time_depth_authored_model`,
  `well_time_depth_model`
  - wrap the existing preview/commit time-depth commands
- `vendor_project`
  - wraps the vendor scan/plan/commit bridge from ADR-0026

## Recommended first rollout

1. Build the registry and session envelope around existing commands without removing them.
2. Implement normalized result mapping for `seismic_volume`, `horizons`, and `well_sources` first.
3. Add time-depth asset providers next.
4. Add velocity-function import after deciding whether its current one-step flow needs a richer
   preview phase.
5. Fold vendor project import into the same shell only after the base manager is stable.
