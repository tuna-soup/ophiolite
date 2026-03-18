use crate::project_assets::{
    data_filename, depth_reference_for_kind, drilling_extent, drilling_metadata,
    parse_drilling_csv, parse_pressure_csv, parse_tops_csv, parse_trajectory_csv, pressure_extent,
    pressure_metadata, read_drilling_rows, read_pressure_rows, read_tops_rows,
    read_trajectory_rows, tops_extent, tops_metadata, trajectory_extent, trajectory_metadata,
    vertical_datum_for_kind, write_drilling_package, write_pressure_package, write_tops_package,
    write_trajectory_package,
};
use crate::{
    AssetBindingInput, AssetTableMetadata, DepthRangeQuery, DrillingObservationRow, IndexKind,
    IngestIssue, LasError, LasFile, PressureObservationRow, Provenance, Result, TopRow,
    TrajectoryRow, WellInfo, package_metadata_for, read_path, revision_token_for_bytes,
    write_package_overwrite,
};
use lithos_compute::{
    ComputeCatalog, ComputeExecutionManifest, ComputeParameterValue, ComputeRegistry,
    CurveSemanticDescriptor, CurveSemanticSource, CurveSemanticType, LogCurveData,
    classify_curve_semantic,
};
use lithos_core::{CurveItem, LasValue, SectionItems, derive_canonical_alias};
use lithos_package::open_package;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PROJECT_SCHEMA_VERSION: &str = "0.1.0";
const PROJECT_MANIFEST_FILENAME: &str = "lithos-project.json";
const PROJECT_CATALOG_FILENAME: &str = "catalog.sqlite";
const ASSET_MANIFEST_FILENAME: &str = "asset_manifest.json";

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LithosProjectManifest {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetKind {
    Log,
    Trajectory,
    TopSet,
    PressureObservation,
    DrillingObservation,
}

impl AssetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Trajectory => "trajectory",
            Self::TopSet => "top_set",
            Self::PressureObservation => "pressure_observation",
            Self::DrillingObservation => "drilling_observation",
        }
    }

    fn asset_dir_name(&self) -> &'static str {
        match self {
            Self::Log => "logs",
            Self::Trajectory => "trajectory",
            Self::TopSet => "tops",
            Self::PressureObservation => "pressure",
            Self::DrillingObservation => "drilling",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "log" => Ok(Self::Log),
            "trajectory" => Ok(Self::Trajectory),
            "top_set" => Ok(Self::TopSet),
            "pressure_observation" => Ok(Self::PressureObservation),
            "drilling_observation" => Ok(Self::DrillingObservation),
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

pub struct LithosProject {
    root: PathBuf,
    catalog_path: PathBuf,
    connection: Connection,
}

impl LithosProject {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(root.join("assets"))?;
        let manifest = LithosProjectManifest {
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
        let _: LithosProjectManifest = serde_json::from_str(&fs::read_to_string(manifest_path)?)?;
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
        write_package_overwrite(&file, &package_root)?;
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
        write_asset_manifest(&package_root, &manifest)?;
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
        write_asset_manifest(Path::new(&asset.package_path), &asset.manifest)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn list_compute_catalog(&self, asset_id: &AssetId) -> Result<ComputeCatalog> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
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
        Ok(ComputeRegistry::new().catalog_for_log_asset(&semantics, &numeric_curve_names))
    }

    pub fn run_compute(
        &mut self,
        request: &ProjectComputeRunRequest,
    ) -> Result<ProjectComputeRunResult> {
        let source_asset = self.asset_by_id(&request.source_asset_id)?;
        require_asset_kind(&source_asset, AssetKind::Log)?;
        let source_package = open_package(&source_asset.package_path)?;
        let source_file = source_package.file();
        let semantics = if source_asset.manifest.curve_semantics.is_empty() {
            classify_log_curves_from_package(&source_asset.package_path)?
        } else {
            source_asset.manifest.curve_semantics.clone()
        };
        let log_curves = log_curve_data_for_compute(source_file, &semantics)?;
        let (mut execution, computed_curve) = ComputeRegistry::new().run_log_compute(
            &request.function_id,
            &log_curves,
            &request.curve_bindings,
            &request.parameters,
            request.output_mnemonic.as_deref(),
        )?;
        execution.source_asset_id = source_asset.id.0.clone();
        execution.source_logical_asset_id = source_asset.logical_asset_id.0.clone();
        execution.executed_at_unix_seconds = now_unix_seconds();

        let source_collection = self.collection_by_id(&source_asset.collection_id)?;
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
        let derived_file = build_derived_log_file(
            source_file,
            &source_asset,
            &collection,
            &storage_asset_id,
            &computed_curve,
            &execution,
        );
        write_package_overwrite(&derived_file, &package_root)?;

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
        write_asset_manifest(&package_root, &manifest)?;
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
        self.insert_asset(&asset, &package_rel_path)?;
        Ok(ProjectComputeRunResult {
            collection,
            asset,
            execution,
        })
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
            .join(format!("{}.lithos-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        writer(&package_root)?;
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
        write_asset_manifest(&package_root, &manifest)?;
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

fn build_derived_log_file(
    source_file: &LasFile,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    computed_curve: &lithos_compute::ComputedCurve,
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

fn derived_log_asset_manifest(
    file: &LasFile,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
    computed_curve: &lithos_compute::ComputedCurve,
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
        },
        start: extent.0,
        stop: extent.1,
        row_count: extent.2,
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
