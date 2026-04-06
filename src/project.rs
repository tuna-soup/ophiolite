use crate::project_assets::{
    data_filename, depth_reference_for_kind, drilling_extent, drilling_metadata,
    parse_drilling_csv, parse_pressure_csv, parse_tops_csv, parse_trajectory_csv, pressure_extent,
    pressure_metadata, read_drilling_rows, read_pressure_rows, read_tops_rows,
    read_trajectory_rows, tops_extent, tops_metadata, trajectory_extent, trajectory_metadata,
    vertical_datum_for_kind, write_drilling_package, write_pressure_package, write_tops_package,
    write_trajectory_package,
};
use crate::project_contracts::{
    ResolvedWellPanelSourceDto, ResolvedWellPanelWellDto, WELL_PANEL_CONTRACT_VERSION,
    WellPanelDepthSampleDto, WellPanelDrillingObservationDto, WellPanelDrillingSetDto,
    WellPanelLogCurveDto, WellPanelPressureObservationDto, WellPanelPressureSetDto,
    WellPanelRequestDto, WellPanelTopRowDto, WellPanelTopSetDto, WellPanelTrajectoryDto,
    WellPanelTrajectoryRowDto,
};
use crate::{
    AssetBindingInput, AssetTableMetadata, DepthRangeQuery, DrillingObservationRow, IndexKind,
    IngestIssue, LasError, LasFile, PressureObservationRow, Provenance, Result, TopRow,
    TrajectoryRow, WellInfo, package_metadata_for, read_path, revision_token_for_bytes,
    write_package_overwrite,
};
use ophiolite_compute::{
    ComputeCatalog, ComputeExecutionManifest, ComputeParameterValue, ComputeRegistry,
    CurveSemanticDescriptor, CurveSemanticSource, CurveSemanticType, DrillingObservationDataRow,
    LogCurveData, PressureObservationDataRow, TopDataRow, TrajectoryDataRow,
    classify_curve_semantic,
};
use ophiolite_core::{CurveItem, LasValue, SectionItems, derive_canonical_alias};
use ophiolite_package::open_package;
use ophiolite_seismic::{SeismicAssetFamily, SeismicTraceDataDescriptor, VolumeDescriptor};
use ophiolite_seismic_runtime::{TbvolManifest, describe_store, open_store};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PROJECT_SCHEMA_VERSION: &str = "0.1.0";
const PROJECT_MANIFEST_FILENAME: &str = "ophiolite-project.json";
const PROJECT_CATALOG_FILENAME: &str = "catalog.sqlite";
const ASSET_MANIFEST_FILENAME: &str = "asset_manifest.json";
const PROJECT_REVISION_STORE_DIRNAME: &str = ".ophiolite";
const PROJECT_ASSET_REVISION_STORE_DIRNAME: &str = "asset-revisions";
const PROJECT_STAGING_DIRNAME: &str = "staging";

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OphioliteProjectManifest {
    pub schema_version: String,
    pub created_at_unix_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WellId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WellboreId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetCollectionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetRevisionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetKind {
    Log,
    Trajectory,
    TopSet,
    PressureObservation,
    DrillingObservation,
    SeismicTraceData,
}

impl AssetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Trajectory => "trajectory",
            Self::TopSet => "top_set",
            Self::PressureObservation => "pressure_observation",
            Self::DrillingObservation => "drilling_observation",
            Self::SeismicTraceData => "seismic_trace_data",
        }
    }

    fn asset_dir_name(&self) -> &'static str {
        match self {
            Self::Log => "logs",
            Self::Trajectory => "trajectory",
            Self::TopSet => "tops",
            Self::PressureObservation => "pressure",
            Self::DrillingObservation => "drilling",
            Self::SeismicTraceData => "seismic-trace-data",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "log" => Ok(Self::Log),
            "trajectory" => Ok(Self::Trajectory),
            "top_set" => Ok(Self::TopSet),
            "pressure_observation" => Ok(Self::PressureObservation),
            "drilling_observation" => Ok(Self::DrillingObservation),
            "seismic_trace_data" => Ok(Self::SeismicTraceData),
            _ => Err(LasError::Validation(format!(
                "unknown asset kind '{value}' in project catalog"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetStatus {
    Imported,
    Validated,
    Bound,
    NeedsReview,
    Rejected,
    Superseded,
}

impl AssetStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Imported => "imported",
            Self::Validated => "validated",
            Self::Bound => "bound",
            Self::NeedsReview => "needs_review",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "imported" => Ok(Self::Imported),
            "validated" => Ok(Self::Validated),
            "bound" => Ok(Self::Bound),
            "needs_review" => Ok(Self::NeedsReview),
            "rejected" => Ok(Self::Rejected),
            "superseded" => Ok(Self::Superseded),
            _ => Err(LasError::Validation(format!(
                "unknown asset status '{value}' in project catalog"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WellIdentifierSet {
    pub primary_name: Option<String>,
    pub uwi: Option<String>,
    pub api: Option<String>,
    pub operator_aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DepthReference {
    MeasuredDepth,
    TrueVerticalDepth,
    TrueVerticalDepthSubsea,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerticalDatum {
    KellyBushing,
    GroundLevel,
    DrillFloor,
    MeanSeaLevel,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinateReference {
    pub name: Option<String>,
    pub geodetic_datum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnitSystem {
    pub depth_unit: Option<String>,
    pub coordinate_unit: Option<String>,
    pub pressure_unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetExtent {
    pub index_kind: Option<IndexKind>,
    pub start: Option<f64>,
    pub stop: Option<f64>,
    pub row_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BulkDataDescriptor {
    pub relative_path: String,
    pub media_type: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetBlobRef {
    pub relative_path: String,
    pub media_type: String,
    pub byte_count: u64,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurveValueChangeSummary {
    pub curve_name: String,
    pub changed_value_count: usize,
    pub first_changed_row: Option<usize>,
    pub last_changed_row: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LogAssetDiffSummary {
    pub metadata_changed: bool,
    pub row_count_changed: bool,
    pub curve_count_changed: bool,
    pub curves_added: Vec<String>,
    pub curves_removed: Vec<String>,
    pub modified_curves: Vec<CurveValueChangeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StructuredAssetDiffSummary {
    pub rows_added: usize,
    pub rows_removed: usize,
    pub rows_updated: usize,
    pub extent_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DirectoryAssetDiffSummary {
    pub entry_count_changed: bool,
    pub changed_path_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetDiffSummary {
    Log(LogAssetDiffSummary),
    Trajectory(StructuredAssetDiffSummary),
    TopSet(StructuredAssetDiffSummary),
    PressureObservation(StructuredAssetDiffSummary),
    DrillingObservation(StructuredAssetDiffSummary),
    SeismicTraceData(DirectoryAssetDiffSummary),
    MetadataOnly { changed_fields: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetRevisionRecord {
    pub revision_id: AssetRevisionId,
    pub asset_id: AssetId,
    pub logical_asset_id: AssetId,
    pub asset_kind: AssetKind,
    pub parent_revision_id: Option<AssetRevisionId>,
    pub package_snapshot_rel_path: String,
    pub created_at_unix_seconds: u64,
    pub metadata_blob: AssetBlobRef,
    pub data_blob: AssetBlobRef,
    pub diff_summary: AssetDiffSummary,
    #[serde(default)]
    pub change_summary: String,
}

#[derive(Debug)]
struct StagedAssetSnapshot {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceArtifactRef {
    pub source_path: String,
    pub original_filename: String,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetReferenceMetadata {
    pub identifiers: WellIdentifierSet,
    pub coordinate_reference: Option<CoordinateReference>,
    pub vertical_datum: Option<VerticalDatum>,
    pub depth_reference: DepthReference,
    pub unit_system: UnitSystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub asset_kind: AssetKind,
    pub asset_schema_version: String,
    pub logical_asset_id: AssetId,
    pub storage_asset_id: AssetId,
    pub well_id: WellId,
    pub wellbore_id: WellboreId,
    pub asset_collection_id: AssetCollectionId,
    pub source_artifacts: Vec<SourceArtifactRef>,
    pub provenance: Provenance,
    pub diagnostics: Vec<IngestIssue>,
    pub extents: AssetExtent,
    pub bulk_data_descriptors: Vec<BulkDataDescriptor>,
    pub reference_metadata: AssetReferenceMetadata,
    pub created_at_unix_seconds: u64,
    pub imported_at_unix_seconds: u64,
    pub supersedes: Option<AssetId>,
    pub derived_from: Option<AssetId>,
    #[serde(default)]
    pub curve_semantics: Vec<CurveSemanticDescriptor>,
    #[serde(default)]
    pub compute_manifest: Option<ComputeExecutionManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WellRecord {
    pub id: WellId,
    pub name: String,
    pub identifiers: WellIdentifierSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WellboreRecord {
    pub id: WellboreId,
    pub well_id: WellId,
    pub name: String,
    pub identifiers: WellIdentifierSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetCollectionRecord {
    pub id: AssetCollectionId,
    pub wellbore_id: WellboreId,
    pub asset_kind: AssetKind,
    pub name: String,
    pub logical_asset_id: AssetId,
    pub status: AssetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: AssetId,
    pub logical_asset_id: AssetId,
    pub collection_id: AssetCollectionId,
    pub well_id: WellId,
    pub wellbore_id: WellboreId,
    pub asset_kind: AssetKind,
    pub status: AssetStatus,
    pub package_path: String,
    pub manifest: AssetManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSummary {
    pub root: String,
    pub catalog_path: String,
    pub manifest_path: String,
    pub well_count: usize,
    pub wellbore_count: usize,
    pub asset_collection_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WellSummary {
    pub well: WellRecord,
    pub wellbore_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WellboreSummary {
    pub wellbore: WellboreRecord,
    pub collection_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetCollectionSummary {
    pub collection: AssetCollectionRecord,
    pub asset_count: usize,
    pub current_asset_id: Option<AssetId>,
    pub superseded_asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAssetSummary {
    pub asset: AssetRecord,
    pub is_current: bool,
    pub supersedes: Option<AssetId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportResolution {
    pub status: AssetStatus,
    pub well_id: WellId,
    pub wellbore_id: WellboreId,
    pub created_well: bool,
    pub created_wellbore: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAssetImportResult {
    pub resolution: ImportResolution,
    pub collection: AssetCollectionRecord,
    pub asset: AssetRecord,
}

pub type ProjectAssetImportResult = LogAssetImportResult;
pub type SeismicAssetImportResult = ProjectAssetImportResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeismicAssetMetadata {
    pub family: SeismicAssetFamily,
    pub descriptor: VolumeDescriptor,
    pub trace_data_descriptor: SeismicTraceDataDescriptor,
    pub store: TbvolManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectComputeRunRequest {
    pub source_asset_id: AssetId,
    pub function_id: String,
    pub curve_bindings: BTreeMap<String, String>,
    pub parameters: BTreeMap<String, ComputeParameterValue>,
    pub output_collection_name: Option<String>,
    pub output_mnemonic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectComputeRunResult {
    pub collection: AssetCollectionRecord,
    pub asset: AssetRecord,
    pub execution: ComputeExecutionManifest,
}

pub struct OphioliteProject {
    root: PathBuf,
    catalog_path: PathBuf,
    connection: Connection,
}

impl OphioliteProject {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(root.join("assets"))?;
        let manifest = OphioliteProjectManifest {
            schema_version: PROJECT_SCHEMA_VERSION.to_string(),
            created_at_unix_seconds: now_unix_seconds(),
        };
        fs::write(
            root.join(PROJECT_MANIFEST_FILENAME),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        let catalog_path = root.join(PROJECT_CATALOG_FILENAME);
        let connection = Connection::open(&catalog_path).map_err(|error| {
            LasError::Storage(format!("failed to open project catalog: {error}"))
        })?;
        initialize_project_schema(&connection)?;
        Ok(Self {
            root,
            catalog_path,
            connection,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        let manifest_path = root.join(PROJECT_MANIFEST_FILENAME);
        if !manifest_path.exists() {
            return Err(LasError::Validation(format!(
                "project manifest not found at '{}'",
                manifest_path.display()
            )));
        }
        let _: OphioliteProjectManifest =
            serde_json::from_str(&fs::read_to_string(manifest_path)?)?;
        let catalog_path = root.join(PROJECT_CATALOG_FILENAME);
        let connection = Connection::open(&catalog_path).map_err(|error| {
            LasError::Storage(format!("failed to open project catalog: {error}"))
        })?;
        initialize_project_schema(&connection)?;
        Ok(Self {
            root,
            catalog_path,
            connection,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn catalog_path(&self) -> &Path {
        &self.catalog_path
    }

    pub fn summary(&self) -> Result<ProjectSummary> {
        let wells = self.list_wells()?;
        let well_count = wells.len();
        let mut wellbore_count = 0usize;
        let mut asset_collection_count = 0usize;
        let mut asset_count = 0usize;

        for well in &wells {
            let wellbores = self.list_wellbores(&well.id)?;
            wellbore_count += wellbores.len();
            for wellbore in &wellbores {
                asset_collection_count += self.list_asset_collections(&wellbore.id)?.len();
                asset_count += self.list_assets(&wellbore.id, None)?.len();
            }
        }

        Ok(ProjectSummary {
            root: self.root.display().to_string(),
            catalog_path: self.catalog_path.display().to_string(),
            manifest_path: self
                .root
                .join(PROJECT_MANIFEST_FILENAME)
                .display()
                .to_string(),
            well_count,
            wellbore_count,
            asset_collection_count,
            asset_count,
        })
    }

    pub fn list_wells(&self) -> Result<Vec<WellRecord>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, primary_name, identifiers_json FROM wells ORDER BY primary_name")
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(WellRecord {
                    id: WellId(row.get(0)?),
                    name: row.get(1)?,
                    identifiers: serde_json::from_str::<WellIdentifierSet>(
                        &row.get::<_, String>(2)?,
                    )
                    .map_err(sql_json_error)?,
                })
            })
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn well_summaries(&self) -> Result<Vec<WellSummary>> {
        self.list_wells()?
            .into_iter()
            .map(|well| {
                let wellbores = self.list_wellbores(&well.id)?;
                let asset_count = wellbores.iter().try_fold(0usize, |count, wellbore| {
                    self.list_assets(&wellbore.id, None)
                        .map(|assets| count + assets.len())
                })?;
                Ok(WellSummary {
                    well,
                    wellbore_count: wellbores.len(),
                    asset_count,
                })
            })
            .collect()
    }

    pub fn list_wellbores(&self, well_id: &WellId) -> Result<Vec<WellboreRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, well_id, primary_name, identifiers_json FROM wellbores WHERE well_id = ?1 ORDER BY primary_name",
        ).map_err(sqlite_error)?;
        let rows = statement
            .query_map([&well_id.0], |row| {
                Ok(WellboreRecord {
                    id: WellboreId(row.get(0)?),
                    well_id: WellId(row.get(1)?),
                    name: row.get(2)?,
                    identifiers: serde_json::from_str::<WellIdentifierSet>(
                        &row.get::<_, String>(3)?,
                    )
                    .map_err(sql_json_error)?,
                })
            })
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn wellbore_summaries(&self, well_id: &WellId) -> Result<Vec<WellboreSummary>> {
        self.list_wellbores(well_id)?
            .into_iter()
            .map(|wellbore| {
                let collection_count = self.list_asset_collections(&wellbore.id)?.len();
                let asset_count = self.list_assets(&wellbore.id, None)?.len();
                Ok(WellboreSummary {
                    wellbore,
                    collection_count,
                    asset_count,
                })
            })
            .collect()
    }

    pub fn list_asset_collections(
        &self,
        wellbore_id: &WellboreId,
    ) -> Result<Vec<AssetCollectionRecord>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, wellbore_id, asset_kind, name, logical_asset_id, status
             FROM asset_collections
             WHERE wellbore_id = ?1
             ORDER BY asset_kind, name",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([&wellbore_id.0], |row| {
                Ok(AssetCollectionRecord {
                    id: AssetCollectionId(row.get(0)?),
                    wellbore_id: WellboreId(row.get(1)?),
                    asset_kind: AssetKind::from_str(&row.get::<_, String>(2)?)
                        .map_err(sql_validation_error)?,
                    name: row.get(3)?,
                    logical_asset_id: AssetId(row.get(4)?),
                    status: AssetStatus::from_str(&row.get::<_, String>(5)?)
                        .map_err(sql_validation_error)?,
                })
            })
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn asset_collection_summaries(
        &self,
        wellbore_id: &WellboreId,
    ) -> Result<Vec<AssetCollectionSummary>> {
        self.list_asset_collections(wellbore_id)?
            .into_iter()
            .map(|collection| {
                let assets = self.list_assets(wellbore_id, Some(collection.asset_kind.clone()))?;
                let collection_assets = assets
                    .into_iter()
                    .filter(|asset| asset.collection_id == collection.id)
                    .collect::<Vec<_>>();
                let current_asset_id = collection_assets
                    .iter()
                    .find(|asset| asset.status == AssetStatus::Bound)
                    .map(|asset| asset.id.clone());
                let superseded_asset_count = collection_assets
                    .iter()
                    .filter(|asset| asset.status == AssetStatus::Superseded)
                    .count();
                Ok(AssetCollectionSummary {
                    collection,
                    asset_count: collection_assets.len(),
                    current_asset_id,
                    superseded_asset_count,
                })
            })
            .collect()
    }

    pub fn list_assets(
        &self,
        wellbore_id: &WellboreId,
        asset_kind: Option<AssetKind>,
    ) -> Result<Vec<AssetRecord>> {
        let (sql, params): (&str, Vec<String>) = match asset_kind {
            Some(kind) => (
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE wellbore_id = ?1 AND asset_kind = ?2
                 ORDER BY created_at_unix_seconds DESC",
                vec![wellbore_id.0.clone(), kind.as_str().to_string()],
            ),
            None => (
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE wellbore_id = ?1
                 ORDER BY created_at_unix_seconds DESC",
                vec![wellbore_id.0.clone()],
            ),
        };

        let mut statement = self.connection.prepare(sql).map_err(sqlite_error)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let manifest_text: String = row.get(8)?;
                let manifest = serde_json::from_str::<AssetManifest>(&manifest_text)
                    .map_err(sql_json_error)?;
                Ok(AssetRecord {
                    id: AssetId(row.get(0)?),
                    logical_asset_id: AssetId(row.get(1)?),
                    collection_id: AssetCollectionId(row.get(2)?),
                    well_id: WellId(row.get(3)?),
                    wellbore_id: WellboreId(row.get(4)?),
                    asset_kind: AssetKind::from_str(&row.get::<_, String>(5)?)
                        .map_err(sql_validation_error)?,
                    status: AssetStatus::from_str(&row.get::<_, String>(6)?)
                        .map_err(sql_validation_error)?,
                    package_path: self
                        .root
                        .join(row.get::<_, String>(7)?)
                        .to_string_lossy()
                        .into(),
                    manifest,
                })
            })
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn asset_summaries(
        &self,
        wellbore_id: &WellboreId,
        asset_kind: Option<AssetKind>,
    ) -> Result<Vec<ProjectAssetSummary>> {
        self.list_assets(wellbore_id, asset_kind)?
            .into_iter()
            .map(|asset| {
                Ok(ProjectAssetSummary {
                    is_current: asset.status == AssetStatus::Bound,
                    supersedes: asset.manifest.supersedes.clone(),
                    asset,
                })
            })
            .collect()
    }

    pub fn import_las(
        &mut self,
        las_path: impl AsRef<Path>,
        collection_name: Option<&str>,
    ) -> Result<LogAssetImportResult> {
        let las_path = las_path.as_ref();
        let file = read_path(las_path, &Default::default())?;
        let identifiers = identifiers_from_well_info(&file.well_info());
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore(&well.id, &identifiers)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                las_path
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "log".to_string());
        let collection =
            self.resolve_or_create_collection(&wellbore.id, AssetKind::Log, &collection_name)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::Log.asset_dir_name())
            .join(format!("{}.laspkg", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        write_package_overwrite(&file, &staged.root)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = log_asset_manifest(
            &file,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            supersedes.clone(),
        );
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind: AssetKind::Log,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest: manifest.clone(),
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::Log(Default::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok(LogAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    pub fn import_trajectory_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_trajectory_csv(csv_path.as_ref())?;
        self.import_structured_asset(
            csv_path.as_ref(),
            binding,
            AssetKind::Trajectory,
            collection_name,
            |root| write_trajectory_package(root, &rows),
            trajectory_metadata(&rows),
            structured_asset_extent(AssetKind::Trajectory, trajectory_extent(&rows)),
        )
    }

    pub fn import_tops_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_tops_csv(csv_path.as_ref())?;
        self.import_structured_asset(
            csv_path.as_ref(),
            binding,
            AssetKind::TopSet,
            collection_name,
            |root| write_tops_package(root, &rows),
            tops_metadata(&rows),
            structured_asset_extent(AssetKind::TopSet, tops_extent(&rows)),
        )
    }

    pub fn import_pressure_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_pressure_csv(csv_path.as_ref())?;
        self.import_structured_asset(
            csv_path.as_ref(),
            binding,
            AssetKind::PressureObservation,
            collection_name,
            |root| write_pressure_package(root, &rows),
            pressure_metadata(&rows),
            structured_asset_extent(AssetKind::PressureObservation, pressure_extent(&rows)),
        )
    }

    pub fn import_drilling_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_drilling_csv(csv_path.as_ref())?;
        self.import_structured_asset(
            csv_path.as_ref(),
            binding,
            AssetKind::DrillingObservation,
            collection_name,
            |root| write_drilling_package(root, &rows),
            drilling_metadata(&rows),
            structured_asset_extent(AssetKind::DrillingObservation, drilling_extent(&rows)),
        )
    }

    pub fn import_seismic_trace_data_store(
        &mut self,
        store_root: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<SeismicAssetImportResult> {
        self.import_seismic_store_with_kind(
            store_root,
            binding,
            AssetKind::SeismicTraceData,
            SeismicAssetFamily::Volume,
            collection_name,
        )
    }

    fn import_seismic_store_with_kind(
        &mut self,
        store_root: impl AsRef<Path>,
        binding: &AssetBindingInput,
        asset_kind: AssetKind,
        family: SeismicAssetFamily,
        collection_name: Option<&str>,
    ) -> Result<SeismicAssetImportResult> {
        let store_root = store_root.as_ref();
        let descriptor = describe_store(store_root).map_err(|error| {
            LasError::Storage(format!("failed to describe seismic store: {error}"))
        })?;
        let handle = open_store(store_root)
            .map_err(|error| LasError::Storage(format!("failed to open seismic store: {error}")))?;
        let metadata = SeismicAssetMetadata {
            family,
            trace_data_descriptor: SeismicTraceDataDescriptor::from(&descriptor),
            descriptor,
            store: handle.manifest,
        };

        self.import_seismic_asset(store_root, binding, asset_kind, collection_name, &metadata)
    }

    pub fn read_trajectory_rows(
        &self,
        asset_id: &AssetId,
        range: Option<&DepthRangeQuery>,
    ) -> Result<Vec<TrajectoryRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Trajectory)?;
        read_trajectory_rows(Path::new(&asset.package_path), range)
    }

    pub fn read_tops(&self, asset_id: &AssetId) -> Result<Vec<TopRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::TopSet)?;
        read_tops_rows(Path::new(&asset.package_path))
    }

    pub fn read_pressure_observations(
        &self,
        asset_id: &AssetId,
        range: Option<&DepthRangeQuery>,
    ) -> Result<Vec<PressureObservationRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::PressureObservation)?;
        read_pressure_rows(Path::new(&asset.package_path), range)
    }

    pub fn read_drilling_observations(
        &self,
        asset_id: &AssetId,
        range: Option<&DepthRangeQuery>,
    ) -> Result<Vec<DrillingObservationRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::DrillingObservation)?;
        read_drilling_rows(Path::new(&asset.package_path), range)
    }

    pub fn read_log_curve_data(&self, asset_id: &AssetId) -> Result<Vec<LogCurveData>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        let package = open_package(&asset.package_path)?;
        let semantics = if asset.manifest.curve_semantics.is_empty() {
            classify_log_curves_from_file(package.file())
        } else {
            asset.manifest.curve_semantics.clone()
        };
        log_curve_data_for_compute(package.file(), &semantics)
    }

    pub fn resolve_well_panel_source(
        &self,
        request: &WellPanelRequestDto,
    ) -> Result<ResolvedWellPanelSourceDto> {
        if request.wellbore_ids.is_empty() {
            return Err(LasError::Validation(
                "well-panel request requires at least one wellbore id".to_string(),
            ));
        }
        if let (Some(depth_min), Some(depth_max)) = (request.depth_min, request.depth_max) {
            if depth_min > depth_max {
                return Err(LasError::Validation(
                    "well-panel request requires depth_min <= depth_max".to_string(),
                ));
            }
        }

        let wells = request
            .wellbore_ids
            .iter()
            .map(|wellbore_id| {
                self.resolve_well_panel_well(
                    &WellboreId(wellbore_id.clone()),
                    request.depth_min,
                    request.depth_max,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ResolvedWellPanelSourceDto {
            schema_version: WELL_PANEL_CONTRACT_VERSION,
            id: format!("well-panel:{}", request.wellbore_ids.join(",")),
            name: "Resolved Well Panel Source".to_string(),
            wells,
        })
    }

    pub fn asset_record(&self, asset_id: &AssetId) -> Result<AssetRecord> {
        self.asset_by_id(asset_id)
    }

    pub fn asset_revisions(&self, asset_id: &AssetId) -> Result<Vec<AssetRevisionRecord>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT revision_json
                 FROM asset_revisions
                 WHERE asset_id = ?1
                 ORDER BY created_at_unix_seconds",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([&asset_id.0], |row| {
                serde_json::from_str::<AssetRevisionRecord>(&row.get::<_, String>(0)?)
                    .map_err(sql_json_error)
            })
            .map_err(sqlite_error)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn current_asset_revision(
        &self,
        asset_id: &AssetId,
    ) -> Result<Option<AssetRevisionRecord>> {
        let mut revisions = self.asset_revisions(asset_id)?;
        Ok(revisions.pop())
    }

    pub fn overwrite_trajectory_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[TrajectoryRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Trajectory)?;
        let previous_rows = self.read_trajectory_rows(asset_id, None)?;
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_trajectory_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = trajectory_metadata(rows).schema_version;
        asset.manifest.extents =
            structured_asset_extent(AssetKind::Trajectory, trajectory_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::Trajectory,
                previous_rows.as_slice(),
                rows,
                asset.manifest.extents
                    != structured_asset_extent(
                        AssetKind::Trajectory,
                        trajectory_extent(&previous_rows),
                    ),
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn overwrite_tops_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[TopRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::TopSet)?;
        let previous_rows = self.read_tops(asset_id)?;
        let previous_extent =
            structured_asset_extent(AssetKind::TopSet, tops_extent(&previous_rows));
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_tops_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = tops_metadata(rows).schema_version;
        asset.manifest.extents = structured_asset_extent(AssetKind::TopSet, tops_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::TopSet,
                previous_rows.as_slice(),
                rows,
                previous_extent != asset.manifest.extents,
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn overwrite_pressure_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[PressureObservationRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::PressureObservation)?;
        let previous_rows = self.read_pressure_observations(asset_id, None)?;
        let previous_extent = structured_asset_extent(
            AssetKind::PressureObservation,
            pressure_extent(&previous_rows),
        );
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_pressure_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = pressure_metadata(rows).schema_version;
        asset.manifest.extents =
            structured_asset_extent(AssetKind::PressureObservation, pressure_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::PressureObservation,
                previous_rows.as_slice(),
                rows,
                previous_extent != asset.manifest.extents,
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn overwrite_drilling_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[DrillingObservationRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::DrillingObservation)?;
        let previous_rows = self.read_drilling_observations(asset_id, None)?;
        let previous_extent = structured_asset_extent(
            AssetKind::DrillingObservation,
            drilling_extent(&previous_rows),
        );
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_drilling_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = drilling_metadata(rows).schema_version;
        asset.manifest.extents =
            structured_asset_extent(AssetKind::DrillingObservation, drilling_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::DrillingObservation,
                previous_rows.as_slice(),
                rows,
                previous_extent != asset.manifest.extents,
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn log_curve_semantics(&self, asset_id: &AssetId) -> Result<Vec<CurveSemanticDescriptor>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        Ok(asset.manifest.curve_semantics)
    }

    pub fn set_log_curve_semantic_override(
        &mut self,
        asset_id: &AssetId,
        curve_name: &str,
        semantic_type: CurveSemanticType,
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        let previous_semantics = asset.manifest.curve_semantics.clone();
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);

        let mut curve_semantics = if asset.manifest.curve_semantics.is_empty() {
            classify_log_curves_from_package(&asset.package_path)?
        } else {
            asset.manifest.curve_semantics.clone()
        };

        let descriptor = curve_semantics
            .iter_mut()
            .find(|item| item.curve_name == curve_name)
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "curve '{}' not found in log asset '{}'",
                    curve_name, asset.id.0
                ))
            })?;
        descriptor.semantic_type = semantic_type;
        descriptor.source = CurveSemanticSource::Override;
        asset.manifest.curve_semantics = curve_semantics;
        let staged = stage_existing_asset_root(&self.root, &asset)?;
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let changed_fields =
            semantic_diff_fields(&previous_semantics, &asset.manifest.curve_semantics);
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            AssetDiffSummary::MetadataOnly { changed_fields },
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn sync_log_asset_head_revision(
        &mut self,
        asset_id: &AssetId,
    ) -> Result<AssetRevisionRecord> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        let current = open_package(&asset.package_path)?;
        let parent = self.current_asset_revision(asset_id)?;
        let staged = stage_existing_asset_root(&self.root, &asset)?;
        let diff_summary = if let Some(previous) = &parent {
            let snapshot_root = self.root.join(&previous.package_snapshot_rel_path);
            if snapshot_root.exists() {
                let previous_package = open_package(&snapshot_root)?;
                AssetDiffSummary::Log(diff_log_files(previous_package.file(), current.file()))
            } else {
                default_asset_diff_summary(&AssetKind::Log)
            }
        } else {
            default_asset_diff_summary(&AssetKind::Log)
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent.as_ref().map(|item| &item.revision_id),
            diff_summary,
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        Ok(revision)
    }

    pub fn list_compute_catalog(&self, asset_id: &AssetId) -> Result<ComputeCatalog> {
        let asset = self.asset_by_id(asset_id)?;
        let registry = ComputeRegistry::new();
        match asset.asset_kind {
            AssetKind::Log => {
                let semantics = if asset.manifest.curve_semantics.is_empty() {
                    classify_log_curves_from_package(&asset.package_path)?
                } else {
                    asset.manifest.curve_semantics.clone()
                };
                let package = open_package(&asset.package_path)?;
                let numeric_curve_names = package
                    .file()
                    .curves
                    .iter()
                    .filter_map(|curve| curve.numeric_data().map(|_| curve.mnemonic.clone()))
                    .collect::<Vec<_>>();
                Ok(registry.catalog_for_log_asset(&semantics, &numeric_curve_names))
            }
            AssetKind::Trajectory => Ok(registry.catalog_for_trajectory_asset()),
            AssetKind::TopSet => Ok(registry.catalog_for_top_set_asset()),
            AssetKind::PressureObservation => Ok(registry.catalog_for_pressure_asset()),
            AssetKind::DrillingObservation => Ok(registry.catalog_for_drilling_asset()),
            AssetKind::SeismicTraceData => Err(LasError::Validation(
                "compute catalog is not implemented for seismic assets yet".to_string(),
            )),
        }
    }

    pub fn run_compute(
        &mut self,
        request: &ProjectComputeRunRequest,
    ) -> Result<ProjectComputeRunResult> {
        let source_asset = self.asset_by_id(&request.source_asset_id)?;
        let source_collection = self.collection_by_id(&source_asset.collection_id)?;
        let registry = ComputeRegistry::new();

        let (collection, asset, execution) = match source_asset.asset_kind {
            AssetKind::Log => {
                let source_package = open_package(&source_asset.package_path)?;
                let source_file = source_package.file();
                let semantics = if source_asset.manifest.curve_semantics.is_empty() {
                    classify_log_curves_from_package(&source_asset.package_path)?
                } else {
                    source_asset.manifest.curve_semantics.clone()
                };
                let log_curves = log_curve_data_for_compute(source_file, &semantics)?;
                let (mut execution, computed_curve) = registry.run_log_compute(
                    &request.function_id,
                    &log_curves,
                    &request.curve_bindings,
                    &request.parameters,
                    request.output_mnemonic.as_deref(),
                )?;
                execution.source_asset_id = source_asset.id.0.clone();
                execution.source_logical_asset_id = source_asset.logical_asset_id.0.clone();
                execution.executed_at_unix_seconds = now_unix_seconds();

                let collection_name = request.output_collection_name.clone().unwrap_or_else(|| {
                    format!(
                        "{} / Derived / {}",
                        source_collection.name, execution.function_name
                    )
                });
                let collection = self.resolve_or_create_collection(
                    &source_asset.wellbore_id,
                    AssetKind::Log,
                    &collection_name,
                )?;
                let storage_asset_id = AssetId(unique_id("asset"));
                let package_rel_path = PathBuf::from("assets")
                    .join(AssetKind::Log.asset_dir_name())
                    .join(format!("{}.laspkg", storage_asset_id.0));
                let package_root = self.root.join(&package_rel_path);
                let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
                let derived_file = build_derived_log_file(
                    source_file,
                    &source_asset,
                    &collection,
                    &storage_asset_id,
                    &computed_curve,
                    &execution,
                );
                write_package_overwrite(&derived_file, &staged.root)?;

                let supersedes = self
                    .latest_active_asset_for_collection(&collection.id)?
                    .map(|asset| asset.id);
                let manifest = derived_log_asset_manifest(
                    &derived_file,
                    &source_asset,
                    &collection,
                    &storage_asset_id,
                    supersedes.clone(),
                    &computed_curve,
                    &execution,
                );
                write_asset_manifest(&staged.root, &manifest)?;
                if let Some(asset_id) = &supersedes {
                    self.mark_asset_superseded(asset_id)?;
                }
                let asset = AssetRecord {
                    id: storage_asset_id,
                    logical_asset_id: collection.logical_asset_id.clone(),
                    collection_id: collection.id.clone(),
                    well_id: source_asset.well_id.clone(),
                    wellbore_id: source_asset.wellbore_id.clone(),
                    asset_kind: AssetKind::Log,
                    status: AssetStatus::Bound,
                    package_path: package_root.to_string_lossy().into_owned(),
                    manifest,
                };
                let revision = self.build_asset_revision_from_snapshot(
                    &asset,
                    None,
                    AssetDiffSummary::Log(Default::default()),
                    &staged,
                )?;
                self.commit_asset_revision(&asset, &revision)?;
                self.insert_asset(&asset, &package_rel_path)?;
                (collection, asset, execution)
            }
            AssetKind::Trajectory => {
                let rows = self.read_trajectory_rows(&source_asset.id, None)?;
                let compute_rows = trajectory_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_trajectory_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    trajectory_rows_from_compute(&derived_rows),
                    AssetKind::Trajectory,
                )?
            }
            AssetKind::TopSet => {
                let rows = self.read_tops(&source_asset.id)?;
                let compute_rows = top_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_top_set_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    top_rows_from_compute(&derived_rows),
                    AssetKind::TopSet,
                )?
            }
            AssetKind::PressureObservation => {
                let rows = self.read_pressure_observations(&source_asset.id, None)?;
                let compute_rows = pressure_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_pressure_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    pressure_rows_from_compute(&derived_rows),
                    AssetKind::PressureObservation,
                )?
            }
            AssetKind::DrillingObservation => {
                let rows = self.read_drilling_observations(&source_asset.id, None)?;
                let compute_rows = drilling_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_drilling_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    drilling_rows_from_compute(&derived_rows),
                    AssetKind::DrillingObservation,
                )?
            }
            AssetKind::SeismicTraceData => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for seismic assets yet".to_string(),
                ));
            }
        };

        Ok(ProjectComputeRunResult {
            collection,
            asset,
            execution,
        })
    }

    fn persist_structured_compute_result(
        &mut self,
        source_asset: &AssetRecord,
        source_collection: &AssetCollectionRecord,
        request: &ProjectComputeRunRequest,
        mut execution: ComputeExecutionManifest,
        rows: StructuredComputedRows,
        asset_kind: AssetKind,
    ) -> Result<(AssetCollectionRecord, AssetRecord, ComputeExecutionManifest)> {
        execution.source_asset_id = source_asset.id.0.clone();
        execution.source_logical_asset_id = source_asset.logical_asset_id.0.clone();
        execution.executed_at_unix_seconds = now_unix_seconds();

        let collection_name = request.output_collection_name.clone().unwrap_or_else(|| {
            format!(
                "{} / Derived / {}",
                source_collection.name, execution.function_name
            )
        });
        let collection = self.resolve_or_create_collection(
            &source_asset.wellbore_id,
            asset_kind.clone(),
            &collection_name,
        )?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(asset_kind.asset_dir_name())
            .join(match asset_kind {
                AssetKind::Log => format!("{}.laspkg", storage_asset_id.0),
                _ => format!("{}.ophiolite-asset", storage_asset_id.0),
            });
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = write_structured_compute_rows(
            &staged.root,
            source_asset,
            &collection,
            &storage_asset_id,
            supersedes.clone(),
            &rows,
            &execution,
            asset_kind.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: source_asset.well_id.clone(),
            wellbore_id: source_asset.wellbore_id.clone(),
            asset_kind,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset.asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok((collection, asset, execution))
    }

    pub fn assets_covering_depth_range(
        &self,
        wellbore_id: &WellboreId,
        depth_min: f64,
        depth_max: f64,
    ) -> Result<Vec<AssetRecord>> {
        if depth_min > depth_max {
            return Err(LasError::Validation(String::from(
                "depth range requires depth_min <= depth_max",
            )));
        }
        Ok(self
            .list_assets(wellbore_id, None)?
            .into_iter()
            .filter(|asset| asset_covers_depth_range(asset, depth_min, depth_max))
            .collect())
    }

    fn resolve_well_panel_well(
        &self,
        wellbore_id: &WellboreId,
        depth_min: Option<f64>,
        depth_max: Option<f64>,
    ) -> Result<ResolvedWellPanelWellDto> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        let current_assets = self
            .asset_summaries(wellbore_id, None)?
            .into_iter()
            .filter(|summary| summary.is_current)
            .collect::<Vec<_>>();

        let mut logs = Vec::new();
        let mut trajectories = Vec::new();
        let mut top_sets = Vec::new();
        let mut pressure_observations = Vec::new();
        let mut drilling_observations = Vec::new();
        let mut depth_samples = Vec::new();

        for summary in current_assets {
            let asset = summary.asset;
            let collection = self.collection_by_id(&asset.collection_id)?;
            match asset.asset_kind {
                AssetKind::Log => {
                    for curve in self.read_log_curve_data(&asset.id)? {
                        let filtered =
                            filter_log_curve_for_depth_range(&curve, depth_min, depth_max);
                        if filtered.depths.is_empty() {
                            continue;
                        }
                        depth_samples.extend(filtered.depths.iter().copied());
                        logs.push(WellPanelLogCurveDto {
                            asset_id: asset.id.0.clone(),
                            logical_asset_id: asset.logical_asset_id.0.clone(),
                            asset_name: collection.name.clone(),
                            curve_name: filtered.curve_name,
                            original_mnemonic: filtered.original_mnemonic,
                            unit: filtered.unit,
                            semantic_type: format!("{:?}", filtered.semantic_type),
                            depths: filtered.depths,
                            values: filtered.values,
                        });
                    }
                }
                AssetKind::Trajectory => {
                    let range = depth_query(depth_min, depth_max);
                    let rows = self.read_trajectory_rows(&asset.id, range.as_ref())?;
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().map(|row| row.measured_depth));
                    trajectories.push(WellPanelTrajectoryDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelTrajectoryRowDto {
                                measured_depth: row.measured_depth,
                                true_vertical_depth: row.true_vertical_depth,
                                azimuth_deg: row.azimuth_deg,
                                inclination_deg: row.inclination_deg,
                                northing_offset: row.northing_offset,
                                easting_offset: row.easting_offset,
                            })
                            .collect(),
                    });
                }
                AssetKind::TopSet => {
                    let rows = filter_top_rows_for_depth_range(
                        self.read_tops(&asset.id)?,
                        depth_min,
                        depth_max,
                    );
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().map(|row| row.top_depth));
                    top_sets.push(WellPanelTopSetDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelTopRowDto {
                                name: row.name,
                                top_depth: row.top_depth,
                                base_depth: row.base_depth,
                                source: row.source,
                                depth_reference: row.depth_reference,
                            })
                            .collect(),
                    });
                }
                AssetKind::PressureObservation => {
                    let range = depth_query(depth_min, depth_max);
                    let rows = self.read_pressure_observations(&asset.id, range.as_ref())?;
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().filter_map(|row| row.measured_depth));
                    pressure_observations.push(WellPanelPressureSetDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelPressureObservationDto {
                                measured_depth: row.measured_depth,
                                pressure: row.pressure,
                                phase: row.phase,
                                test_kind: row.test_kind,
                                timestamp: row.timestamp,
                            })
                            .collect(),
                    });
                }
                AssetKind::DrillingObservation => {
                    let range = depth_query(depth_min, depth_max);
                    let rows = self.read_drilling_observations(&asset.id, range.as_ref())?;
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().filter_map(|row| row.measured_depth));
                    drilling_observations.push(WellPanelDrillingSetDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelDrillingObservationDto {
                                measured_depth: row.measured_depth,
                                event_kind: row.event_kind,
                                value: row.value,
                                unit: row.unit,
                                timestamp: row.timestamp,
                                comment: row.comment,
                            })
                            .collect(),
                    });
                }
                AssetKind::SeismicTraceData => {}
            }
        }

        let panel_depth_mapping = identity_panel_depth_mapping(depth_samples);
        Ok(ResolvedWellPanelWellDto {
            well_id: wellbore.well_id.0.clone(),
            wellbore_id: wellbore.id.0.clone(),
            name: wellbore.name,
            native_depth_datum: "measured_depth".to_string(),
            panel_depth_mapping,
            logs,
            trajectories,
            top_sets,
            pressure_observations,
            drilling_observations,
        })
    }

    fn import_structured_asset<F>(
        &mut self,
        source_path: &Path,
        binding: &AssetBindingInput,
        asset_kind: AssetKind,
        collection_name: Option<&str>,
        writer: F,
        metadata: AssetTableMetadata,
        extent: AssetExtent,
    ) -> Result<ProjectAssetImportResult>
    where
        F: FnOnce(&Path) -> Result<()>,
    {
        let identifiers = identifiers_from_binding(binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, binding)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                source_path
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| asset_kind.as_str().to_string());
        let collection =
            self.resolve_or_create_collection(&wellbore.id, asset_kind.clone(), &collection_name)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(asset_kind.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        writer(&staged.root)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = structured_asset_manifest(
            source_path,
            &metadata,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            asset_kind.clone(),
            extent,
            identifiers.clone(),
            supersedes.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset.asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    fn import_seismic_asset(
        &mut self,
        source_root: &Path,
        binding: &AssetBindingInput,
        asset_kind: AssetKind,
        collection_name: Option<&str>,
        metadata: &SeismicAssetMetadata,
    ) -> Result<SeismicAssetImportResult> {
        let identifiers = identifiers_from_binding(binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, binding)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                source_root
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| asset_kind.as_str().to_string());
        let collection =
            self.resolve_or_create_collection(&wellbore.id, asset_kind.clone(), &collection_name)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(asset_kind.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        copy_path(source_root, &staged.root.join("store"))?;
        fs::write(
            staged.root.join("metadata.json"),
            serde_json::to_vec_pretty(metadata)?,
        )?;

        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = seismic_asset_manifest(
            source_root,
            metadata,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            asset_kind.clone(),
            identifiers.clone(),
            supersedes.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }

        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset.asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    fn asset_by_id(&self, asset_id: &AssetId) -> Result<AssetRecord> {
        self.connection
            .query_row(
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE id = ?1",
                params![asset_id.0],
                |row| {
                    let manifest = serde_json::from_str::<AssetManifest>(&row.get::<_, String>(8)?)
                        .map_err(sql_json_error)?;
                    Ok(AssetRecord {
                        id: AssetId(row.get(0)?),
                        logical_asset_id: AssetId(row.get(1)?),
                        collection_id: AssetCollectionId(row.get(2)?),
                        well_id: WellId(row.get(3)?),
                        wellbore_id: WellboreId(row.get(4)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                        status: AssetStatus::from_str(&row.get::<_, String>(6)?)
                            .map_err(sql_validation_error)?,
                        package_path: self
                            .root
                            .join(row.get::<_, String>(7)?)
                            .to_string_lossy()
                            .into_owned(),
                        manifest,
                    })
                },
            )
            .map_err(sqlite_error)
    }

    fn collection_by_id(&self, collection_id: &AssetCollectionId) -> Result<AssetCollectionRecord> {
        self.connection
            .query_row(
                "SELECT id, wellbore_id, asset_kind, name, logical_asset_id, status
                 FROM asset_collections
                 WHERE id = ?1",
                params![collection_id.0],
                |row| {
                    Ok(AssetCollectionRecord {
                        id: AssetCollectionId(row.get(0)?),
                        wellbore_id: WellboreId(row.get(1)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(2)?)
                            .map_err(sql_validation_error)?,
                        name: row.get(3)?,
                        logical_asset_id: AssetId(row.get(4)?),
                        status: AssetStatus::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                    })
                },
            )
            .map_err(sqlite_error)
    }

    fn wellbore_by_id(&self, wellbore_id: &WellboreId) -> Result<WellboreRecord> {
        self.connection
            .query_row(
                "SELECT id, well_id, primary_name, identifiers_json
                 FROM wellbores
                 WHERE id = ?1",
                params![wellbore_id.0],
                |row| {
                    Ok(WellboreRecord {
                        id: WellboreId(row.get(0)?),
                        well_id: WellId(row.get(1)?),
                        name: row.get(2)?,
                        identifiers: serde_json::from_str::<WellIdentifierSet>(
                            &row.get::<_, String>(3)?,
                        )
                        .map_err(sql_json_error)?,
                    })
                },
            )
            .map_err(sqlite_error)
    }

    fn resolve_or_create_well(
        &self,
        identifiers: &WellIdentifierSet,
    ) -> Result<(WellRecord, bool)> {
        if let Some(well) = self.find_matching_well(identifiers)? {
            return Ok((well, false));
        }

        let well = WellRecord {
            id: WellId(unique_id("well")),
            name: identifiers
                .primary_name
                .clone()
                .unwrap_or_else(|| "Unknown Well".to_string()),
            identifiers: identifiers.clone(),
        };
        self.connection.execute(
            "INSERT INTO wells (id, primary_name, normalized_name, uwi, api, identifiers_json, created_at_unix_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                well.id.0,
                well.name,
                normalized_text(&well.name),
                optional_db_text(&well.identifiers.uwi),
                optional_db_text(&well.identifiers.api),
                serde_json::to_string(&well.identifiers)?,
                now_unix_seconds() as i64,
            ],
        ).map_err(sqlite_error)?;
        Ok((well, true))
    }

    fn resolve_or_create_wellbore(
        &self,
        well_id: &WellId,
        identifiers: &WellIdentifierSet,
    ) -> Result<(WellboreRecord, bool)> {
        let wellbore_name = identifiers
            .primary_name
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let normalized = normalized_text(&wellbore_name);
        let existing = self
            .connection
            .query_row(
                "SELECT id, well_id, primary_name, identifiers_json
                 FROM wellbores
                 WHERE well_id = ?1 AND normalized_name = ?2",
                params![well_id.0, normalized],
                |row| {
                    Ok(WellboreRecord {
                        id: WellboreId(row.get(0)?),
                        well_id: WellId(row.get(1)?),
                        name: row.get(2)?,
                        identifiers: serde_json::from_str::<WellIdentifierSet>(
                            &row.get::<_, String>(3)?,
                        )
                        .map_err(sql_json_error)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        if let Some(wellbore) = existing {
            return Ok((wellbore, false));
        }

        let wellbore = WellboreRecord {
            id: WellboreId(unique_id("wellbore")),
            well_id: well_id.clone(),
            name: wellbore_name,
            identifiers: identifiers.clone(),
        };
        self.connection.execute(
            "INSERT INTO wellbores (id, well_id, primary_name, normalized_name, identifiers_json, created_at_unix_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                wellbore.id.0,
                wellbore.well_id.0,
                wellbore.name,
                normalized_text(&wellbore.name),
                serde_json::to_string(&wellbore.identifiers)?,
                now_unix_seconds() as i64,
            ],
        ).map_err(sqlite_error)?;
        Ok((wellbore, true))
    }

    fn resolve_or_create_wellbore_for_binding(
        &self,
        well_id: &WellId,
        binding: &AssetBindingInput,
    ) -> Result<(WellboreRecord, bool)> {
        let identifiers = WellIdentifierSet {
            primary_name: Some(binding.wellbore_name.clone()),
            uwi: binding.uwi.clone(),
            api: binding.api.clone(),
            operator_aliases: binding.operator_aliases.clone(),
        };
        self.resolve_or_create_wellbore(well_id, &identifiers)
    }

    fn resolve_or_create_collection(
        &self,
        wellbore_id: &WellboreId,
        asset_kind: AssetKind,
        name: &str,
    ) -> Result<AssetCollectionRecord> {
        let existing = self
            .connection
            .query_row(
                "SELECT id, wellbore_id, asset_kind, name, logical_asset_id, status
                 FROM asset_collections
                 WHERE wellbore_id = ?1 AND asset_kind = ?2 AND name = ?3",
                params![wellbore_id.0, asset_kind.as_str(), name],
                |row| {
                    Ok(AssetCollectionRecord {
                        id: AssetCollectionId(row.get(0)?),
                        wellbore_id: WellboreId(row.get(1)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(2)?)
                            .map_err(sql_validation_error)?,
                        name: row.get(3)?,
                        logical_asset_id: AssetId(row.get(4)?),
                        status: AssetStatus::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        if let Some(collection) = existing {
            return Ok(collection);
        }

        let collection = AssetCollectionRecord {
            id: AssetCollectionId(unique_id("collection")),
            wellbore_id: wellbore_id.clone(),
            asset_kind: asset_kind.clone(),
            name: name.to_string(),
            logical_asset_id: AssetId(unique_id("logical")),
            status: AssetStatus::Bound,
        };
        self.connection.execute(
            "INSERT INTO asset_collections (id, wellbore_id, asset_kind, name, logical_asset_id, status, created_at_unix_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                collection.id.0,
                collection.wellbore_id.0,
                collection.asset_kind.as_str(),
                collection.name,
                collection.logical_asset_id.0,
                collection.status.as_str(),
                now_unix_seconds() as i64,
            ],
        ).map_err(sqlite_error)?;
        Ok(collection)
    }

    fn latest_active_asset_for_collection(
        &self,
        collection_id: &AssetCollectionId,
    ) -> Result<Option<AssetRecord>> {
        let row = self
            .connection
            .query_row(
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE collection_id = ?1 AND status != 'superseded'
                 ORDER BY created_at_unix_seconds DESC
                 LIMIT 1",
                params![collection_id.0],
                |row| {
                    let manifest = serde_json::from_str::<AssetManifest>(&row.get::<_, String>(8)?)
                        .map_err(sql_json_error)?;
                    Ok(AssetRecord {
                        id: AssetId(row.get(0)?),
                        logical_asset_id: AssetId(row.get(1)?),
                        collection_id: AssetCollectionId(row.get(2)?),
                        well_id: WellId(row.get(3)?),
                        wellbore_id: WellboreId(row.get(4)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                        status: AssetStatus::from_str(&row.get::<_, String>(6)?)
                            .map_err(sql_validation_error)?,
                        package_path: self.root.join(row.get::<_, String>(7)?).to_string_lossy().into_owned(),
                        manifest,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        Ok(row)
    }

    fn mark_asset_superseded(&self, asset_id: &AssetId) -> Result<()> {
        self.connection
            .execute(
                "UPDATE assets SET status = 'superseded' WHERE id = ?1",
                params![asset_id.0],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn insert_asset(&self, asset: &AssetRecord, package_rel_path: &Path) -> Result<()> {
        self.connection.execute(
            "INSERT INTO assets
             (id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json, created_at_unix_seconds, source_path, source_fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                asset.id.0,
                asset.logical_asset_id.0,
                asset.collection_id.0,
                asset.well_id.0,
                asset.wellbore_id.0,
                asset.asset_kind.as_str(),
                asset.status.as_str(),
                package_rel_path.to_string_lossy().to_string(),
                serde_json::to_string(&asset.manifest)?,
                now_unix_seconds() as i64,
                asset.manifest.provenance.source_path.clone(),
                asset.manifest.provenance.source_fingerprint.clone(),
            ],
        ).map_err(sqlite_error)?;
        Ok(())
    }

    fn update_asset_manifest(&self, asset: &AssetRecord) -> Result<()> {
        let package_rel_path = Path::new(&asset.package_path)
            .strip_prefix(&self.root)
            .map_err(|_| {
                LasError::Storage(format!(
                    "asset '{}' does not live under project root '{}'",
                    asset.id.0,
                    self.root.display()
                ))
            })?
            .to_string_lossy()
            .to_string();
        self.connection
            .execute(
                "UPDATE assets
                 SET manifest_json = ?2, package_rel_path = ?3, source_path = ?4, source_fingerprint = ?5
                 WHERE id = ?1",
                params![
                    asset.id.0,
                    serde_json::to_string(&asset.manifest)?,
                    package_rel_path,
                    asset.manifest.provenance.source_path.clone(),
                    asset.manifest.provenance.source_fingerprint.clone(),
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn find_matching_well(&self, identifiers: &WellIdentifierSet) -> Result<Option<WellRecord>> {
        if let Some(uwi) = &identifiers.uwi {
            if let Some(well) = self
                .connection
                .query_row(
                    "SELECT id, primary_name, identifiers_json FROM wells WHERE uwi = ?1",
                    params![uwi],
                    |row| {
                        Ok(WellRecord {
                            id: WellId(row.get(0)?),
                            name: row.get(1)?,
                            identifiers: serde_json::from_str::<WellIdentifierSet>(
                                &row.get::<_, String>(2)?,
                            )
                            .map_err(sql_json_error)?,
                        })
                    },
                )
                .optional()
                .map_err(sqlite_error)?
            {
                return Ok(Some(well));
            }
        }

        if let Some(api) = &identifiers.api {
            if let Some(well) = self
                .connection
                .query_row(
                    "SELECT id, primary_name, identifiers_json FROM wells WHERE api = ?1",
                    params![api],
                    |row| {
                        Ok(WellRecord {
                            id: WellId(row.get(0)?),
                            name: row.get(1)?,
                            identifiers: serde_json::from_str::<WellIdentifierSet>(
                                &row.get::<_, String>(2)?,
                            )
                            .map_err(sql_json_error)?,
                        })
                    },
                )
                .optional()
                .map_err(sqlite_error)?
            {
                return Ok(Some(well));
            }
        }

        if let Some(name) = &identifiers.primary_name {
            if let Some(well) = self
                .connection
                .query_row(
                    "SELECT id, primary_name, identifiers_json FROM wells WHERE normalized_name = ?1",
                    params![normalized_text(name)],
                    |row| {
                        Ok(WellRecord {
                            id: WellId(row.get(0)?),
                            name: row.get(1)?,
                            identifiers: serde_json::from_str::<WellIdentifierSet>(
                                &row.get::<_, String>(2)?,
                            )
                            .map_err(sql_json_error)?,
                        })
                    },
                )
                .optional()
                .map_err(sqlite_error)?
            {
                return Ok(Some(well));
            }
        }

        Ok(None)
    }

    fn insert_asset_revision(&self, revision: &AssetRevisionRecord) -> Result<()> {
        self.connection
            .execute(
                "INSERT INTO asset_revisions
                 (id, asset_id, logical_asset_id, asset_kind, parent_revision_id, revision_json, created_at_unix_seconds)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    revision.revision_id.0,
                    revision.asset_id.0,
                    revision.logical_asset_id.0,
                    revision.asset_kind.as_str(),
                    revision
                        .parent_revision_id
                        .as_ref()
                        .map(|value| value.0.clone()),
                    serde_json::to_string(revision)?,
                    revision.created_at_unix_seconds as i64,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn build_asset_revision_from_snapshot(
        &self,
        asset: &AssetRecord,
        parent_revision_id: Option<&AssetRevisionId>,
        diff_summary: AssetDiffSummary,
        staged_snapshot: &StagedAssetSnapshot,
    ) -> Result<AssetRevisionRecord> {
        let created_at_unix_seconds = now_unix_seconds();
        let manifest_path = staged_snapshot.root.join(ASSET_MANIFEST_FILENAME);
        let manifest_bytes = fs::read(&manifest_path)?;
        let metadata_blob = AssetBlobRef {
            relative_path: ASSET_MANIFEST_FILENAME.to_string(),
            media_type: "application/json".to_string(),
            byte_count: manifest_bytes.len() as u64,
            content_hash: stable_project_blob_hash("asset-manifest", &manifest_bytes),
        };
        let data_blob = asset_primary_blob_ref(&asset.manifest, &staged_snapshot.root)?;
        let revision_id = AssetRevisionId(
            revision_token_for_bytes(
                "asset-revision",
                &format!(
                    "{}:{}:{}",
                    metadata_blob.content_hash, data_blob.content_hash, created_at_unix_seconds
                ),
            )
            .0,
        );
        let snapshot_rel_path = project_asset_revision_store_rel_path(&asset.id, &revision_id);
        let snapshot_root = self.root.join(&snapshot_rel_path);
        if let Some(parent) = snapshot_root.parent() {
            fs::create_dir_all(parent)?;
        }
        if snapshot_root.exists() {
            fs::remove_dir_all(&snapshot_root)?;
        }
        fs::rename(&staged_snapshot.root, &snapshot_root)?;
        let change_summary = summarize_asset_diff(&asset.asset_kind, &diff_summary);
        let revision = AssetRevisionRecord {
            revision_id,
            asset_id: asset.id.clone(),
            logical_asset_id: asset.logical_asset_id.clone(),
            asset_kind: asset.asset_kind.clone(),
            parent_revision_id: parent_revision_id.cloned(),
            package_snapshot_rel_path: snapshot_rel_path.to_string_lossy().to_string(),
            created_at_unix_seconds,
            metadata_blob,
            data_blob,
            diff_summary,
            change_summary,
        };
        Ok(revision)
    }

    fn commit_asset_revision(
        &self,
        asset: &AssetRecord,
        revision: &AssetRevisionRecord,
    ) -> Result<()> {
        let previous_head = self.current_asset_revision(&asset.id)?;
        self.materialize_asset_head_from_revision(asset, revision)?;
        if let Err(error) = self.insert_asset_revision(revision) {
            if let Some(previous_revision) = previous_head.as_ref() {
                self.materialize_asset_head_from_revision(asset, previous_revision)?;
            } else {
                clear_project_visible_files(asset)?;
            }
            return Err(error);
        }
        Ok(())
    }

    fn materialize_asset_head_from_revision(
        &self,
        asset: &AssetRecord,
        revision: &AssetRevisionRecord,
    ) -> Result<()> {
        let revision_root = self.root.join(&revision.package_snapshot_rel_path);
        fs::create_dir_all(&asset.package_path)?;
        materialize_project_visible_files(
            &self.root,
            &artifact_mappings_for_asset(asset, &revision_root),
        )
    }
}

fn copy_if_exists(source: &Path, target: &Path) -> Result<()> {
    if source.exists() {
        copy_path(source, target)?;
    }
    Ok(())
}

fn copy_path(source: &Path, target: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_path(&entry.path(), &target.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
    }
    Ok(())
}

fn stage_project_asset_root(root: &Path, asset_id: &AssetId) -> Result<StagedAssetSnapshot> {
    let staging_root = PathBuf::from(root)
        .join(PROJECT_REVISION_STORE_DIRNAME)
        .join(PROJECT_STAGING_DIRNAME)
        .join(&asset_id.0)
        .join(
            revision_token_for_bytes(
                "asset-stage",
                &format!("{}:{}", asset_id.0, now_unix_nanos()),
            )
            .0,
        );
    fs::create_dir_all(&staging_root)?;
    Ok(StagedAssetSnapshot { root: staging_root })
}

fn stage_existing_asset_root(root: &Path, asset: &AssetRecord) -> Result<StagedAssetSnapshot> {
    let staged = stage_project_asset_root(root, &asset.id)?;
    for relative_path in asset_visible_relative_paths(&asset.manifest) {
        copy_if_exists(
            &Path::new(&asset.package_path).join(&relative_path),
            &staged.root.join(relative_path),
        )?;
    }
    Ok(staged)
}

fn project_asset_revision_store_rel_path(
    asset_id: &AssetId,
    revision_id: &AssetRevisionId,
) -> PathBuf {
    PathBuf::from(PROJECT_REVISION_STORE_DIRNAME)
        .join(PROJECT_ASSET_REVISION_STORE_DIRNAME)
        .join(&asset_id.0)
        .join(&revision_id.0)
}

fn stable_project_blob_hash(scope: &str, bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    scope.hash(&mut hasher);
    bytes.hash(&mut hasher);
    format!("{scope}-{:016x}", hasher.finish())
}

fn stable_project_path_hash(scope: &str, path: &Path) -> Result<String> {
    let mut hasher = DefaultHasher::new();
    scope.hash(&mut hasher);
    hash_path_into(path, path, &mut hasher)?;
    Ok(format!("{scope}-{:016x}", hasher.finish()))
}

fn hash_path_into(root: &Path, path: &Path, hasher: &mut DefaultHasher) -> Result<()> {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    relative.hash(hasher);
    if path.is_dir() {
        "dir".hash(hasher);
        let mut entries = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            hash_path_into(root, &entry.path(), hasher)?;
        }
    } else {
        "file".hash(hasher);
        fs::read(path)?.hash(hasher);
    }
    Ok(())
}

fn path_byte_count(path: &Path) -> Result<u64> {
    if path.is_dir() {
        let mut total = 0u64;
        for entry in fs::read_dir(path)? {
            total += path_byte_count(&entry?.path())?;
        }
        Ok(total)
    } else {
        Ok(fs::metadata(path)?.len())
    }
}

fn remove_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn asset_primary_blob_ref(manifest: &AssetManifest, root: &Path) -> Result<AssetBlobRef> {
    let descriptor = manifest
        .bulk_data_descriptors
        .iter()
        .find(|descriptor| descriptor.role != "metadata")
        .or_else(|| manifest.bulk_data_descriptors.first())
        .ok_or_else(|| {
            LasError::Validation("asset manifest is missing bulk data descriptors".to_string())
        })?;
    let data_path = root.join(&descriptor.relative_path);
    Ok(AssetBlobRef {
        relative_path: descriptor.relative_path.clone(),
        media_type: descriptor.media_type.clone(),
        byte_count: path_byte_count(&data_path)?,
        content_hash: stable_project_path_hash("asset-data", &data_path)?,
    })
}

fn asset_visible_relative_paths(manifest: &AssetManifest) -> Vec<String> {
    let mut paths = Vec::with_capacity(manifest.bulk_data_descriptors.len() + 1);
    paths.push("metadata.json".to_string());
    paths.push(ASSET_MANIFEST_FILENAME.to_string());
    for descriptor in &manifest.bulk_data_descriptors {
        if !paths.iter().any(|path| path == &descriptor.relative_path) {
            paths.push(descriptor.relative_path.clone());
        }
    }
    paths
}

fn artifact_mappings_for_asset(
    asset: &AssetRecord,
    revision_root: &Path,
) -> Vec<(PathBuf, PathBuf)> {
    asset_visible_relative_paths(&asset.manifest)
        .into_iter()
        .map(|relative_path| {
            (
                revision_root.join(&relative_path),
                Path::new(&asset.package_path).join(relative_path),
            )
        })
        .collect()
}

fn materialize_project_visible_files(root: &Path, mappings: &[(PathBuf, PathBuf)]) -> Result<()> {
    let backup_root = PathBuf::from(root)
        .join(PROJECT_REVISION_STORE_DIRNAME)
        .join(PROJECT_STAGING_DIRNAME)
        .join(
            revision_token_for_bytes(
                "project-materialize-backup",
                &format!("{}:{}", root.display(), now_unix_nanos()),
            )
            .0,
        );
    fs::create_dir_all(&backup_root)?;

    for (_, destination) in mappings {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            copy_path(
                destination,
                &backup_root.join(
                    destination
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_else(|| String::from("backup")),
                ),
            )?;
        }
    }

    for (source, destination) in mappings {
        let temp_path = destination.with_extension("next");
        remove_path(&temp_path)?;
        copy_path(source, &temp_path)?;
        if destination.exists() {
            remove_path(destination)?;
        }
        if let Err(error) = fs::rename(&temp_path, destination) {
            restore_project_visible_files(&backup_root, mappings)?;
            return Err(LasError::Io(error));
        }
    }

    if backup_root.exists() {
        fs::remove_dir_all(backup_root)?;
    }
    Ok(())
}

fn restore_project_visible_files(root: &Path, mappings: &[(PathBuf, PathBuf)]) -> Result<()> {
    for (_, destination) in mappings {
        let backup_path = root.join(
            destination
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| String::from("backup")),
        );
        if backup_path.exists() {
            if destination.exists() {
                remove_path(destination)?;
            }
            copy_path(&backup_path, destination)?;
        }
    }
    Ok(())
}

fn clear_project_visible_files(asset: &AssetRecord) -> Result<()> {
    for relative_path in asset_visible_relative_paths(&asset.manifest) {
        let path = Path::new(&asset.package_path).join(relative_path);
        if path.exists() {
            remove_path(&path)?;
        }
    }
    Ok(())
}

fn default_asset_diff_summary(asset_kind: &AssetKind) -> AssetDiffSummary {
    match asset_kind {
        AssetKind::Log => AssetDiffSummary::Log(LogAssetDiffSummary::default()),
        AssetKind::Trajectory => {
            AssetDiffSummary::Trajectory(StructuredAssetDiffSummary::default())
        }
        AssetKind::TopSet => AssetDiffSummary::TopSet(StructuredAssetDiffSummary::default()),
        AssetKind::PressureObservation => {
            AssetDiffSummary::PressureObservation(StructuredAssetDiffSummary::default())
        }
        AssetKind::DrillingObservation => {
            AssetDiffSummary::DrillingObservation(StructuredAssetDiffSummary::default())
        }
        AssetKind::SeismicTraceData => {
            AssetDiffSummary::SeismicTraceData(DirectoryAssetDiffSummary::default())
        }
    }
}

fn diff_structured_rows<T: PartialEq>(
    asset_kind: AssetKind,
    previous_rows: &[T],
    current_rows: &[T],
    extent_changed: bool,
) -> AssetDiffSummary {
    let rows_updated = previous_rows
        .iter()
        .zip(current_rows.iter())
        .filter(|(left, right)| left != right)
        .count();
    let summary = StructuredAssetDiffSummary {
        rows_added: current_rows.len().saturating_sub(previous_rows.len()),
        rows_removed: previous_rows.len().saturating_sub(current_rows.len()),
        rows_updated,
        extent_changed,
    };
    match asset_kind {
        AssetKind::Trajectory => AssetDiffSummary::Trajectory(summary),
        AssetKind::TopSet => AssetDiffSummary::TopSet(summary),
        AssetKind::PressureObservation => AssetDiffSummary::PressureObservation(summary),
        AssetKind::DrillingObservation => AssetDiffSummary::DrillingObservation(summary),
        AssetKind::Log => AssetDiffSummary::Log(LogAssetDiffSummary::default()),
        AssetKind::SeismicTraceData => {
            AssetDiffSummary::SeismicTraceData(DirectoryAssetDiffSummary::default())
        }
    }
}

fn semantic_diff_fields(
    previous: &[CurveSemanticDescriptor],
    current: &[CurveSemanticDescriptor],
) -> Vec<String> {
    current
        .iter()
        .filter_map(|descriptor| {
            let previous_descriptor = previous
                .iter()
                .find(|item| item.curve_name == descriptor.curve_name)?;
            (previous_descriptor.semantic_type != descriptor.semantic_type
                || previous_descriptor.source != descriptor.source)
                .then(|| format!("curve_semantics.{}", descriptor.curve_name))
        })
        .collect()
}

fn diff_log_files(previous: &LasFile, current: &LasFile) -> LogAssetDiffSummary {
    let previous_curves = previous
        .curves
        .iter()
        .map(|curve| (curve.mnemonic.clone(), curve))
        .collect::<BTreeMap<_, _>>();
    let current_curves = current
        .curves
        .iter()
        .map(|curve| (curve.mnemonic.clone(), curve))
        .collect::<BTreeMap<_, _>>();
    let curves_added = current_curves
        .keys()
        .filter(|name| !previous_curves.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    let curves_removed = previous_curves
        .keys()
        .filter(|name| !current_curves.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    let modified_curves = current_curves
        .iter()
        .filter_map(|(name, current_curve)| {
            let previous_curve = previous_curves.get(name)?;
            let summary = diff_log_curve_values(name, previous_curve, current_curve);
            (summary.changed_value_count > 0).then_some(summary)
        })
        .collect::<Vec<_>>();

    LogAssetDiffSummary {
        metadata_changed: serde_json::to_string(&package_metadata_for(current, 1).canonical).ok()
            != serde_json::to_string(&package_metadata_for(previous, 1).canonical).ok(),
        row_count_changed: current.row_count() != previous.row_count(),
        curve_count_changed: current.curves.len() != previous.curves.len(),
        curves_added,
        curves_removed,
        modified_curves,
    }
}

fn diff_log_curve_values(
    curve_name: &str,
    previous: &CurveItem,
    current: &CurveItem,
) -> CurveValueChangeSummary {
    let max_len = previous.data.len().max(current.data.len());
    let mut changed_value_count = 0usize;
    let mut first_changed_row = None;
    let mut last_changed_row = None;

    for row_index in 0..max_len {
        let previous_value = previous.data.get(row_index);
        let current_value = current.data.get(row_index);
        if log_values_equal(previous_value, current_value) {
            continue;
        }
        changed_value_count += 1;
        first_changed_row.get_or_insert(row_index);
        last_changed_row = Some(row_index);
    }

    CurveValueChangeSummary {
        curve_name: curve_name.to_string(),
        changed_value_count,
        first_changed_row,
        last_changed_row,
    }
}

fn summarize_asset_diff(asset_kind: &AssetKind, diff: &AssetDiffSummary) -> String {
    match diff {
        AssetDiffSummary::Log(summary) => summarize_log_asset_diff(summary),
        AssetDiffSummary::Trajectory(summary) => {
            summarize_structured_asset_diff("trajectory", summary)
        }
        AssetDiffSummary::TopSet(summary) => summarize_structured_asset_diff("tops", summary),
        AssetDiffSummary::PressureObservation(summary) => {
            summarize_structured_asset_diff("pressure observations", summary)
        }
        AssetDiffSummary::DrillingObservation(summary) => {
            summarize_structured_asset_diff("drilling observations", summary)
        }
        AssetDiffSummary::SeismicTraceData(summary) => {
            summarize_directory_asset_diff("seismic trace data", summary)
        }
        AssetDiffSummary::MetadataOnly { changed_fields } => {
            if changed_fields.is_empty() {
                format!("updated {} metadata", asset_kind.as_str())
            } else {
                format!("updated metadata fields {}", changed_fields.join(", "))
            }
        }
    }
}

fn summarize_log_asset_diff(diff: &LogAssetDiffSummary) -> String {
    let mut parts = Vec::new();
    if diff.metadata_changed {
        parts.push(String::from("metadata updated"));
    }
    if !diff.curves_added.is_empty() {
        parts.push(format!("added curves {}", diff.curves_added.join(", ")));
    }
    if !diff.curves_removed.is_empty() {
        parts.push(format!("removed curves {}", diff.curves_removed.join(", ")));
    }
    if !diff.modified_curves.is_empty() {
        parts.push(format!(
            "updated {} curve value ranges",
            diff.modified_curves.len()
        ));
    }
    if diff.row_count_changed {
        parts.push(String::from("row count changed"));
    }
    if diff.curve_count_changed {
        parts.push(String::from("curve count changed"));
    }
    if parts.is_empty() {
        String::from("initial log asset revision")
    } else {
        parts.join("; ")
    }
}

fn summarize_structured_asset_diff(label: &str, diff: &StructuredAssetDiffSummary) -> String {
    let mut parts = Vec::new();
    if diff.rows_added > 0 {
        parts.push(format!("added {} rows", diff.rows_added));
    }
    if diff.rows_removed > 0 {
        parts.push(format!("removed {} rows", diff.rows_removed));
    }
    if diff.rows_updated > 0 {
        parts.push(format!("updated {} rows", diff.rows_updated));
    }
    if diff.extent_changed {
        parts.push(String::from("extent changed"));
    }
    if parts.is_empty() {
        format!("initial {label} asset revision")
    } else {
        parts.join("; ")
    }
}

fn summarize_directory_asset_diff(label: &str, diff: &DirectoryAssetDiffSummary) -> String {
    let mut parts = Vec::new();
    if diff.entry_count_changed {
        parts.push(String::from("entry count changed"));
    }
    if diff.changed_path_count > 0 {
        parts.push(format!("updated {} paths", diff.changed_path_count));
    }
    if parts.is_empty() {
        format!("initial {label} asset revision")
    } else {
        parts.join("; ")
    }
}

fn log_values_equal(previous: Option<&LasValue>, current: Option<&LasValue>) -> bool {
    match (previous, current) {
        (Some(LasValue::Number(left)), Some(LasValue::Number(right))) => {
            (left.is_nan() && right.is_nan()) || left == right
        }
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

fn initialize_project_schema(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "
        CREATE TABLE IF NOT EXISTS project_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS wells (
            id TEXT PRIMARY KEY,
            primary_name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            uwi TEXT,
            api TEXT,
            identifiers_json TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS wellbores (
            id TEXT PRIMARY KEY,
            well_id TEXT NOT NULL,
            primary_name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            identifiers_json TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS asset_collections (
            id TEXT PRIMARY KEY,
            wellbore_id TEXT NOT NULL,
            asset_kind TEXT NOT NULL,
            name TEXT NOT NULL,
            logical_asset_id TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL,
            UNIQUE(wellbore_id, asset_kind, name)
        );
        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            logical_asset_id TEXT NOT NULL,
            collection_id TEXT NOT NULL,
            well_id TEXT NOT NULL,
            wellbore_id TEXT NOT NULL,
            asset_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            package_rel_path TEXT NOT NULL,
            manifest_json TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL,
            source_path TEXT,
            source_fingerprint TEXT
        );
        CREATE TABLE IF NOT EXISTS asset_revisions (
            id TEXT PRIMARY KEY,
            asset_id TEXT NOT NULL,
            logical_asset_id TEXT NOT NULL,
            asset_kind TEXT NOT NULL,
            parent_revision_id TEXT,
            revision_json TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL
        );
        ",
        )
        .map_err(sqlite_error)?;
    connection
        .execute(
            "INSERT OR REPLACE INTO project_meta (key, value) VALUES ('schema_version', ?1)",
            params![PROJECT_SCHEMA_VERSION],
        )
        .map_err(sqlite_error)?;
    Ok(())
}

fn classify_log_curves_from_file(file: &LasFile) -> Vec<CurveSemanticDescriptor> {
    let package_metadata = package_metadata_for(file, 1);
    package_metadata
        .storage
        .curve_columns
        .iter()
        .map(|curve| CurveSemanticDescriptor {
            curve_name: curve.name.clone(),
            original_mnemonic: curve.original_mnemonic.clone(),
            unit: (!curve.unit.trim().is_empty()).then_some(curve.unit.clone()),
            semantic_type: classify_curve_semantic(
                &curve.alias,
                &curve.original_mnemonic,
                Some(&curve.unit),
                curve.is_index,
            ),
            source: CurveSemanticSource::Derived,
        })
        .collect()
}

fn classify_log_curves_from_package(package_path: &str) -> Result<Vec<CurveSemanticDescriptor>> {
    let package = open_package(package_path)?;
    Ok(classify_log_curves_from_file(package.file()))
}

fn log_curve_data_for_compute(
    file: &LasFile,
    semantics: &[CurveSemanticDescriptor],
) -> Result<Vec<LogCurveData>> {
    let index_curve = file.curve(&file.index.curve_id)?;
    let depths = index_curve.numeric_data().ok_or_else(|| {
        LasError::Validation(format!(
            "index curve '{}' must remain numeric for compute execution",
            file.index.curve_id
        ))
    })?;
    let semantics_by_name = semantics
        .iter()
        .map(|item| (item.curve_name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut result = Vec::new();
    for curve in file.curves.iter() {
        let numeric = curve.numeric_data();
        if numeric.is_none() {
            continue;
        }
        let descriptor = semantics_by_name
            .get(&curve.mnemonic)
            .cloned()
            .unwrap_or_else(|| CurveSemanticDescriptor {
                curve_name: curve.mnemonic.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                unit: (!curve.unit.trim().is_empty()).then_some(curve.unit.clone()),
                semantic_type: classify_curve_semantic(
                    &derive_canonical_alias(&curve.original_mnemonic, &curve.unit),
                    &curve.original_mnemonic,
                    Some(&curve.unit),
                    curve.mnemonic == file.index.curve_id,
                ),
                source: CurveSemanticSource::Derived,
            });
        let values = numeric
            .unwrap()
            .into_iter()
            .map(|value| (!value.is_nan()).then_some(value))
            .collect::<Vec<_>>();
        result.push(LogCurveData {
            curve_name: curve.mnemonic.clone(),
            original_mnemonic: curve.original_mnemonic.clone(),
            unit: (!curve.unit.trim().is_empty()).then_some(curve.unit.clone()),
            semantic_type: descriptor.semantic_type,
            depths: depths.clone(),
            values,
        });
    }
    Ok(result)
}

fn filter_log_curve_for_depth_range(
    curve: &LogCurveData,
    depth_min: Option<f64>,
    depth_max: Option<f64>,
) -> LogCurveData {
    let mut depths = Vec::new();
    let mut values = Vec::new();
    for (depth, value) in curve.depths.iter().zip(curve.values.iter()) {
        if !depth_in_range(*depth, depth_min, depth_max) {
            continue;
        }
        depths.push(*depth);
        values.push(*value);
    }
    LogCurveData {
        curve_name: curve.curve_name.clone(),
        original_mnemonic: curve.original_mnemonic.clone(),
        unit: curve.unit.clone(),
        semantic_type: curve.semantic_type.clone(),
        depths,
        values,
    }
}

fn filter_top_rows_for_depth_range(
    rows: Vec<TopRow>,
    depth_min: Option<f64>,
    depth_max: Option<f64>,
) -> Vec<TopRow> {
    rows.into_iter()
        .filter(|row| depth_in_range(row.top_depth, depth_min, depth_max))
        .collect()
}

fn identity_panel_depth_mapping(depths: Vec<f64>) -> Vec<WellPanelDepthSampleDto> {
    let mut unique = depths
        .into_iter()
        .filter(|depth| depth.is_finite())
        .collect::<Vec<_>>();
    unique.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    unique.dedup_by(|left, right| (*left - *right).abs() < 1e-6);
    unique
        .into_iter()
        .map(|depth| WellPanelDepthSampleDto {
            native_depth: depth,
            panel_depth: depth,
        })
        .collect()
}

fn depth_in_range(depth: f64, depth_min: Option<f64>, depth_max: Option<f64>) -> bool {
    if let Some(min) = depth_min {
        if depth < min {
            return false;
        }
    }
    if let Some(max) = depth_max {
        if depth > max {
            return false;
        }
    }
    true
}

fn depth_query(depth_min: Option<f64>, depth_max: Option<f64>) -> Option<DepthRangeQuery> {
    if depth_min.is_none() && depth_max.is_none() {
        None
    } else {
        Some(DepthRangeQuery {
            depth_min,
            depth_max,
        })
    }
}

enum StructuredComputedRows {
    Trajectory(Vec<TrajectoryRow>),
    TopSet(Vec<TopRow>),
    Pressure(Vec<PressureObservationRow>),
    Drilling(Vec<DrillingObservationRow>),
}

fn trajectory_rows_for_compute(rows: &[TrajectoryRow]) -> Vec<TrajectoryDataRow> {
    rows.iter()
        .cloned()
        .map(|row| TrajectoryDataRow {
            measured_depth: row.measured_depth,
            true_vertical_depth: row.true_vertical_depth,
            azimuth_deg: row.azimuth_deg,
            inclination_deg: row.inclination_deg,
            northing_offset: row.northing_offset,
            easting_offset: row.easting_offset,
        })
        .collect()
}

fn trajectory_rows_from_compute(rows: &[TrajectoryDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::Trajectory(
        rows.iter()
            .cloned()
            .map(|row| TrajectoryRow {
                measured_depth: row.measured_depth,
                true_vertical_depth: row.true_vertical_depth,
                azimuth_deg: row.azimuth_deg,
                inclination_deg: row.inclination_deg,
                northing_offset: row.northing_offset,
                easting_offset: row.easting_offset,
            })
            .collect(),
    )
}

fn top_rows_for_compute(rows: &[TopRow]) -> Vec<TopDataRow> {
    rows.iter()
        .cloned()
        .map(|row| TopDataRow {
            name: row.name,
            top_depth: row.top_depth,
            base_depth: row.base_depth,
            source: row.source,
            depth_reference: row.depth_reference,
        })
        .collect()
}

fn top_rows_from_compute(rows: &[TopDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::TopSet(
        rows.iter()
            .cloned()
            .map(|row| TopRow {
                name: row.name,
                top_depth: row.top_depth,
                base_depth: row.base_depth,
                source: row.source,
                depth_reference: row.depth_reference,
            })
            .collect(),
    )
}

fn pressure_rows_for_compute(rows: &[PressureObservationRow]) -> Vec<PressureObservationDataRow> {
    rows.iter()
        .cloned()
        .map(|row| PressureObservationDataRow {
            measured_depth: row.measured_depth,
            pressure: row.pressure,
            phase: row.phase,
            test_kind: row.test_kind,
            timestamp: row.timestamp,
        })
        .collect()
}

fn pressure_rows_from_compute(rows: &[PressureObservationDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::Pressure(
        rows.iter()
            .cloned()
            .map(|row| PressureObservationRow {
                measured_depth: row.measured_depth,
                pressure: row.pressure,
                phase: row.phase,
                test_kind: row.test_kind,
                timestamp: row.timestamp,
            })
            .collect(),
    )
}

fn drilling_rows_for_compute(rows: &[DrillingObservationRow]) -> Vec<DrillingObservationDataRow> {
    rows.iter()
        .cloned()
        .map(|row| DrillingObservationDataRow {
            measured_depth: row.measured_depth,
            event_kind: row.event_kind,
            value: row.value,
            unit: row.unit,
            timestamp: row.timestamp,
            comment: row.comment,
        })
        .collect()
}

fn drilling_rows_from_compute(rows: &[DrillingObservationDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::Drilling(
        rows.iter()
            .cloned()
            .map(|row| DrillingObservationRow {
                measured_depth: row.measured_depth,
                event_kind: row.event_kind,
                value: row.value,
                unit: row.unit,
                timestamp: row.timestamp,
                comment: row.comment,
            })
            .collect(),
    )
}

fn build_derived_log_file(
    source_file: &LasFile,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    computed_curve: &ophiolite_compute::ComputedCurve,
    execution: &ComputeExecutionManifest,
) -> LasFile {
    let mut derived = source_file.clone();
    let index_curve = source_file
        .curve(&source_file.index.curve_id)
        .cloned()
        .unwrap();
    let curve_item = CurveItem::new(
        computed_curve.curve_name.clone(),
        computed_curve.unit.clone().unwrap_or_default(),
        LasValue::Empty,
        computed_curve.description.clone().unwrap_or_default(),
        computed_curve
            .values
            .iter()
            .map(|value| match value {
                Some(number) => LasValue::Number(*number),
                None => LasValue::Empty,
            })
            .collect(),
    );
    derived.curves = SectionItems::from_items(
        vec![index_curve, curve_item],
        source_file.curves.mnemonic_case,
    );
    derived.summary.source_path = source_asset.package_path.clone();
    derived.summary.original_filename = format!(
        "{}-{}.las",
        collection.name.replace(' ', "_"),
        storage_asset_id.0
    );
    derived.summary.source_fingerprint =
        revision_token_for_bytes("compute", &execution.function_id).0;
    derived.summary.curve_count = derived.curves.len();
    derived.summary.row_count = derived.row_count();
    derived.summary.issue_count = derived.issues.len();
    derived.provenance = Provenance {
        source_path: source_asset.package_path.clone(),
        original_filename: derived.summary.original_filename.clone(),
        source_fingerprint: derived.summary.source_fingerprint.clone(),
        imported_at_unix_seconds: execution.executed_at_unix_seconds,
    };
    derived
}

fn log_asset_manifest(
    file: &LasFile,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
) -> AssetManifest {
    let metadata = package_metadata_for(file, 1);
    let imported_at = now_unix_seconds();
    AssetManifest {
        asset_kind: AssetKind::Log,
        asset_schema_version: "0.1.0".to_string(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: file.provenance.source_path.clone(),
            original_filename: file.provenance.original_filename.clone(),
            source_fingerprint: file.provenance.source_fingerprint.clone(),
        }],
        provenance: file.provenance.clone(),
        diagnostics: file.issues.clone(),
        extents: AssetExtent {
            index_kind: Some(file.index.kind.clone()),
            start: metadata.canonical.well.start,
            stop: metadata.canonical.well.stop,
            row_count: Some(file.row_count()),
        },
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: "curves.parquet".to_string(),
                media_type: "application/vnd.apache.parquet".to_string(),
                role: "curve_samples".to_string(),
            },
        ],
        reference_metadata: AssetReferenceMetadata {
            identifiers: identifiers_from_well_info(&metadata.canonical.well),
            coordinate_reference: None,
            vertical_datum: None,
            depth_reference: DepthReference::MeasuredDepth,
            unit_system: UnitSystem {
                depth_unit: metadata.storage.index_unit.clone(),
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: classify_log_curves_from_file(file),
        compute_manifest: None,
    }
}

fn structured_asset_manifest(
    source_path: &Path,
    metadata: &AssetTableMetadata,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    asset_kind: AssetKind,
    extent: AssetExtent,
    identifiers: WellIdentifierSet,
    supersedes: Option<AssetId>,
) -> Result<AssetManifest> {
    let imported_at = now_unix_seconds();
    let source_bytes = fs::read(source_path)?;
    let fingerprint = source_fingerprint(&source_bytes);
    let provenance = Provenance::from_path(source_path, fingerprint.clone(), imported_at);
    Ok(AssetManifest {
        asset_kind: asset_kind.clone(),
        asset_schema_version: metadata.schema_version.clone(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
        }],
        provenance,
        diagnostics: Vec::new(),
        extents: extent,
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: data_filename().to_string(),
                media_type: "application/vnd.apache.parquet".to_string(),
                role: "bulk_data".to_string(),
            },
        ],
        reference_metadata: AssetReferenceMetadata {
            identifiers,
            coordinate_reference: None,
            vertical_datum: vertical_datum_for_kind(&asset_kind),
            depth_reference: depth_reference_for_kind(&asset_kind),
            unit_system: UnitSystem {
                depth_unit: None,
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: Vec::new(),
        compute_manifest: None,
    })
}

fn seismic_asset_manifest(
    source_root: &Path,
    metadata: &SeismicAssetMetadata,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    asset_kind: AssetKind,
    identifiers: WellIdentifierSet,
    supersedes: Option<AssetId>,
) -> Result<AssetManifest> {
    let imported_at = now_unix_seconds();
    let source_fingerprint = stable_project_path_hash("seismic-source", source_root)?;
    let provenance = Provenance {
        source_path: source_root.display().to_string(),
        original_filename: source_root
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| asset_kind.as_str().to_string()),
        source_fingerprint,
        imported_at_unix_seconds: imported_at,
    };
    Ok(AssetManifest {
        asset_kind,
        asset_schema_version: "0.1.0".to_string(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
        }],
        provenance,
        diagnostics: Vec::new(),
        extents: seismic_asset_extent(metadata),
        bulk_data_descriptors: vec![BulkDataDescriptor {
            relative_path: "store".to_string(),
            media_type: "application/vnd.ophiolite.tbvol".to_string(),
            role: "seismic_store".to_string(),
        }],
        reference_metadata: AssetReferenceMetadata {
            identifiers,
            coordinate_reference: None,
            vertical_datum: None,
            depth_reference: DepthReference::Unknown,
            unit_system: UnitSystem {
                depth_unit: None,
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: Vec::new(),
        compute_manifest: None,
    })
}

fn derived_log_asset_manifest(
    file: &LasFile,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
    computed_curve: &ophiolite_compute::ComputedCurve,
    execution: &ComputeExecutionManifest,
) -> AssetManifest {
    let mut manifest = log_asset_manifest(
        file,
        &source_asset.well_id,
        &source_asset.wellbore_id,
        &collection.id,
        &collection.logical_asset_id,
        storage_asset_id,
        supersedes,
    );
    manifest.asset_schema_version = "0.2.0".to_string();
    manifest.source_artifacts = source_asset.manifest.source_artifacts.clone();
    manifest.reference_metadata = source_asset.manifest.reference_metadata.clone();
    manifest.derived_from = Some(source_asset.logical_asset_id.clone());
    manifest.curve_semantics = vec![
        CurveSemanticDescriptor {
            curve_name: file.index.curve_id.clone(),
            original_mnemonic: file.index.raw_mnemonic.clone(),
            unit: (!file.index.unit.trim().is_empty()).then_some(file.index.unit.clone()),
            semantic_type: classify_curve_semantic(
                &derive_canonical_alias(&file.index.raw_mnemonic, &file.index.unit),
                &file.index.raw_mnemonic,
                Some(&file.index.unit),
                true,
            ),
            source: CurveSemanticSource::Derived,
        },
        CurveSemanticDescriptor {
            curve_name: computed_curve.curve_name.clone(),
            original_mnemonic: computed_curve.original_mnemonic.clone(),
            unit: computed_curve.unit.clone(),
            semantic_type: computed_curve.semantic_type.clone(),
            source: CurveSemanticSource::Computed,
        },
    ];
    manifest.compute_manifest = Some(execution.clone());
    manifest
}

fn write_structured_compute_rows(
    package_root: &Path,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
    rows: &StructuredComputedRows,
    execution: &ComputeExecutionManifest,
    asset_kind: AssetKind,
) -> Result<AssetManifest> {
    match rows {
        StructuredComputedRows::Trajectory(rows) => {
            write_trajectory_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                trajectory_metadata(rows),
                structured_asset_extent(AssetKind::Trajectory, trajectory_extent(rows)),
            )
        }
        StructuredComputedRows::TopSet(rows) => {
            write_tops_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                tops_metadata(rows),
                structured_asset_extent(AssetKind::TopSet, tops_extent(rows)),
            )
        }
        StructuredComputedRows::Pressure(rows) => {
            write_pressure_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                pressure_metadata(rows),
                structured_asset_extent(AssetKind::PressureObservation, pressure_extent(rows)),
            )
        }
        StructuredComputedRows::Drilling(rows) => {
            write_drilling_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                drilling_metadata(rows),
                structured_asset_extent(AssetKind::DrillingObservation, drilling_extent(rows)),
            )
        }
    }
}

fn derived_structured_asset_manifest(
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
    execution: &ComputeExecutionManifest,
    asset_kind: AssetKind,
    metadata: AssetTableMetadata,
    extent: AssetExtent,
) -> Result<AssetManifest> {
    let imported_at = execution.executed_at_unix_seconds;
    let mut manifest = AssetManifest {
        asset_kind: asset_kind.clone(),
        asset_schema_version: metadata.schema_version.clone(),
        logical_asset_id: collection.logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: source_asset.well_id.clone(),
        wellbore_id: source_asset.wellbore_id.clone(),
        asset_collection_id: collection.id.clone(),
        source_artifacts: source_asset.manifest.source_artifacts.clone(),
        provenance: Provenance {
            source_path: source_asset.package_path.clone(),
            original_filename: format!("derived-{}", execution.function_id),
            source_fingerprint: revision_token_for_bytes("compute", &execution.function_id).0,
            imported_at_unix_seconds: imported_at,
        },
        diagnostics: Vec::new(),
        extents: extent,
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: data_filename().to_string(),
                media_type: "application/vnd.apache.parquet".to_string(),
                role: "bulk_data".to_string(),
            },
        ],
        reference_metadata: source_asset.manifest.reference_metadata.clone(),
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: Some(source_asset.logical_asset_id.clone()),
        curve_semantics: Vec::new(),
        compute_manifest: Some(execution.clone()),
    };
    manifest.source_artifacts = source_asset.manifest.source_artifacts.clone();
    Ok(manifest)
}

fn write_asset_manifest(root: &Path, manifest: &AssetManifest) -> Result<()> {
    fs::write(
        root.join(ASSET_MANIFEST_FILENAME),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn identifiers_from_well_info(info: &WellInfo) -> WellIdentifierSet {
    WellIdentifierSet {
        primary_name: info.well.clone(),
        uwi: info.uwi.clone(),
        api: info.api.clone(),
        operator_aliases: info
            .company
            .clone()
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect(),
    }
}

fn identifiers_from_binding(binding: &AssetBindingInput) -> WellIdentifierSet {
    WellIdentifierSet {
        primary_name: Some(binding.well_name.clone()),
        uwi: binding.uwi.clone(),
        api: binding.api.clone(),
        operator_aliases: binding.operator_aliases.clone(),
    }
}

fn structured_asset_extent(
    asset_kind: AssetKind,
    extent: (Option<f64>, Option<f64>, Option<usize>),
) -> AssetExtent {
    AssetExtent {
        index_kind: match asset_kind {
            AssetKind::Trajectory
            | AssetKind::TopSet
            | AssetKind::PressureObservation
            | AssetKind::DrillingObservation => Some(IndexKind::Depth),
            AssetKind::Log => Some(IndexKind::Depth),
            AssetKind::SeismicTraceData => Some(IndexKind::Time),
        },
        start: extent.0,
        stop: extent.1,
        row_count: extent.2,
    }
}

fn seismic_asset_extent(metadata: &SeismicAssetMetadata) -> AssetExtent {
    let sample_axis = &metadata.store.volume.axes.sample_axis_ms;
    AssetExtent {
        index_kind: Some(IndexKind::Time),
        start: sample_axis.first().copied().map(f64::from),
        stop: sample_axis.last().copied().map(f64::from),
        row_count: Some(metadata.store.volume.shape[0] * metadata.store.volume.shape[1]),
    }
}

fn require_asset_kind(asset: &AssetRecord, expected: AssetKind) -> Result<()> {
    if asset.asset_kind != expected {
        return Err(LasError::Validation(format!(
            "asset '{}' is {}, not {}",
            asset.id.0,
            asset.asset_kind.as_str(),
            expected.as_str()
        )));
    }
    Ok(())
}

fn asset_covers_depth_range(asset: &AssetRecord, depth_min: f64, depth_max: f64) -> bool {
    let start = asset.manifest.extents.start;
    let stop = asset.manifest.extents.stop;
    match (start, stop) {
        (Some(start), Some(stop)) => start <= depth_max && stop >= depth_min,
        _ => false,
    }
}

fn unique_id(prefix: &str) -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{prefix}_{nanos}_{counter}")
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn normalized_text(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn optional_db_text(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn sqlite_error(error: rusqlite::Error) -> LasError {
    LasError::Storage(format!("project catalog error: {error}"))
}

fn sql_json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn sql_validation_error(error: LasError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::other(error.to_string())),
    )
}

fn source_fingerprint(bytes: &[u8]) -> String {
    let checksum = bytes.iter().fold(0u64, |acc, byte| {
        acc.wrapping_mul(16777619).wrapping_add(u64::from(*byte))
    });
    revision_token_for_bytes("source", &format!("{}:{checksum}", bytes.len())).0
}
