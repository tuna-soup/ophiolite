use arrow_array::Array;
use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use lithos_core::{
    AssetSummaryDto, CanonicalMetadata, CurveCatalogDto, CurveCatalogEntryDto, CurveColumnMetadata,
    CurveEditRequest, CurveInfo, CurveItem, CurveStorageKind, CurveTable, CurveWindowColumnDto,
    CurveWindowDto, CurveWindowRequest, DTO_CONTRACT_VERSION, DepthWindowRequest, DirtyStateDto,
    HeaderItem, IndexInfo, LasError, LasFile, LasFileSummary, LasValue, MetadataDto,
    MetadataSectionDto, MetadataUpdateRequest, PackageId, PackageMetadata, ParameterInfo, Result,
    RevisionToken, SavePackageResultDto, SaveSessionResponseDto, SectionItems, SessionId,
    SessionMetadataDto, SessionSummaryDto, SessionWindowDto, ValidationReportDto, VersionInfo,
    WellInfo, apply_curve_edit, apply_metadata_update, asset_summary_dto, curve_catalog_dto,
    curve_catalog_result_dto, curve_window_dto, depth_window_request_for_values, dirty_state_dto,
    metadata_dto, package_id_for_path, package_metadata_for, package_validation_report,
    parse_package_metadata, revision_token_for_bytes, session_metadata_dto, session_summary_dto,
    session_window_dto, validate_edit_state, validate_package_metadata, validation_report_dto,
};
use lithos_table::CurveColumnDescriptor;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::{
    ArrowReaderMetadata, ArrowReaderOptions, ParquetRecordBatchReaderBuilder, RowSelection,
};
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::metadata::SortingColumn;
use parquet::file::properties::{EnabledStatistics, WriterProperties};
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::iter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

const PACKAGE_VERSION: u32 = 1;
const METADATA_FILENAME: &str = "metadata.json";
const CURVES_FILENAME: &str = "curves.parquet";
const CURVES_ROW_GROUP_ROW_COUNT: usize = 16_384;
const CURVES_DATA_PAGE_ROW_COUNT_LIMIT: usize = 4_096;

#[derive(Debug, Clone)]
pub struct PackageSession {
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    dirty: bool,
    root: PathBuf,
    file: LasFile,
    table: CurveTable,
}

pub type StoredLasFile = PackageSession;

#[derive(Debug, Default)]
pub struct PackageSessionStore {
    sessions: BTreeMap<String, PackageSession>,
    package_sessions: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct LazyPackageSession {
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    dirty: bool,
    root: PathBuf,
    metadata: PackageMetadata,
    reader_metadata: ArrowReaderMetadata,
}

#[derive(Debug, Clone)]
enum BackendPackageSession {
    Lazy(LazyPackageSession),
    Materialized(PackageSession),
}

#[derive(Debug, Default)]
pub struct PackageBackendSessionStore {
    sessions: BTreeMap<String, BackendPackageSession>,
    package_sessions: BTreeMap<String, String>,
}

impl PackageSessionStore {
    pub fn open_shared(&mut self, path: impl AsRef<Path>) -> Result<SessionSummaryDto> {
        let key = package_path_key(path.as_ref());
        if let Some(session_id) = self.package_sessions.get(&key) {
            if let Some(session) = self.sessions.get(session_id) {
                return Ok(session.session_summary());
            }
        }

        let session = open_package(path)?;
        let session_id = session.session_id.0.clone();
        self.package_sessions.insert(key, session_id.clone());
        let summary = session.session_summary();
        self.sessions.insert(session_id, session);
        Ok(summary)
    }

    pub fn get(&self, session_id: &SessionId) -> Option<&PackageSession> {
        self.sessions.get(&session_id.0)
    }

    pub fn get_mut(&mut self, session_id: &SessionId) -> Option<&mut PackageSession> {
        self.sessions.get_mut(&session_id.0)
    }

    pub fn close(&mut self, session_id: &SessionId) -> Option<PackageSession> {
        let session = self.sessions.remove(&session_id.0)?;
        self.package_sessions
            .retain(|_, existing| existing != &session_id.0);
        Some(session)
    }

    pub fn rebind_path(&mut self, session_id: &SessionId, old_root: &Path, new_root: &Path) {
        let old_key = package_path_key(old_root);
        let new_key = package_path_key(new_root);
        self.package_sessions.remove(&old_key);
        self.package_sessions.insert(new_key, session_id.0.clone());
    }
}

impl PackageBackendSessionStore {
    pub fn open_shared(&mut self, path: impl AsRef<Path>) -> Result<SessionSummaryDto> {
        let key = package_path_key(path.as_ref());
        if let Some(session_id) = self.package_sessions.get(&key) {
            if let Some(session) = self.sessions.get(session_id) {
                return Ok(session.session_summary());
            }
        }

        let session = open_backend_session(path)?;
        let session_id = session.session_id().0.clone();
        self.package_sessions.insert(key, session_id.clone());
        let summary = session.session_summary();
        self.sessions.insert(session_id, session);
        Ok(summary)
    }

    pub fn session_summary(&self, session_id: &SessionId) -> Result<SessionSummaryDto> {
        Ok(self.session(session_id)?.session_summary())
    }

    pub fn session_metadata(&self, session_id: &SessionId) -> Result<SessionMetadataDto> {
        Ok(self.session(session_id)?.session_metadata())
    }

    pub fn session_curve_catalog(&self, session_id: &SessionId) -> Result<CurveCatalogDto> {
        Ok(self.session(session_id)?.curve_catalog())
    }

    pub fn read_curve_window(
        &self,
        session_id: &SessionId,
        request: &CurveWindowRequest,
    ) -> Result<SessionWindowDto> {
        self.session(session_id)?.read_window(request)
    }

    pub fn read_depth_window(
        &self,
        session_id: &SessionId,
        request: &DepthWindowRequest,
    ) -> Result<SessionWindowDto> {
        self.session(session_id)?.read_depth_window(request)
    }

    pub fn dirty_state(&self, session_id: &SessionId) -> Result<DirtyStateDto> {
        Ok(self.session(session_id)?.dirty_state())
    }

    pub fn apply_metadata_edit(
        &mut self,
        session_id: &SessionId,
        request: &MetadataUpdateRequest,
    ) -> Result<SessionSummaryDto> {
        let session = self.session_mut(session_id)?;
        session.apply_metadata_update(request)?;
        Ok(session.session_summary())
    }

    pub fn apply_curve_edit(
        &mut self,
        session_id: &SessionId,
        request: &CurveEditRequest,
    ) -> Result<SessionSummaryDto> {
        let session = self.session_mut(session_id)?;
        session.apply_curve_edit(request)?;
        Ok(session.session_summary())
    }

    pub fn save_session(&mut self, session_id: &SessionId) -> Result<SaveSessionResponseDto> {
        self.session_mut(session_id)?.save_checked()
    }

    pub fn save_session_as(
        &mut self,
        session_id: &SessionId,
        output_dir: impl AsRef<Path>,
    ) -> Result<SaveSessionResponseDto> {
        let old_root = self.session(session_id)?.root().to_path_buf();
        let response = self
            .session_mut(session_id)?
            .save_as_checked_in_place(output_dir.as_ref())?;
        let SaveSessionResponseDto::Saved(saved) = &response;
        self.rebind_path(session_id, &old_root, Path::new(&saved.root));
        Ok(response)
    }

    pub fn close(&mut self, session_id: &SessionId) -> Option<PackageId> {
        let session = self.sessions.remove(&session_id.0)?;
        self.package_sessions
            .retain(|_, existing| existing != &session_id.0);
        Some(session.package_id().clone())
    }

    pub fn rebind_path(&mut self, session_id: &SessionId, old_root: &Path, new_root: &Path) {
        let old_key = package_path_key(old_root);
        let new_key = package_path_key(new_root);
        self.package_sessions.remove(&old_key);
        self.package_sessions.insert(new_key, session_id.0.clone());
    }

    fn session(&self, session_id: &SessionId) -> Result<&BackendPackageSession> {
        self.sessions
            .get(&session_id.0)
            .ok_or_else(|| LasError::Validation(format!("session '{}' not found", session_id.0)))
    }

    fn session_mut(&mut self, session_id: &SessionId) -> Result<&mut BackendPackageSession> {
        self.sessions
            .get_mut(&session_id.0)
            .ok_or_else(|| LasError::Validation(format!("session '{}' not found", session_id.0)))
    }
}

impl BackendPackageSession {
    fn session_id(&self) -> &SessionId {
        match self {
            Self::Lazy(session) => &session.session_id,
            Self::Materialized(session) => session.session_id(),
        }
    }

    fn package_id(&self) -> &PackageId {
        match self {
            Self::Lazy(session) => &session.package_id,
            Self::Materialized(session) => session.package_id(),
        }
    }

    fn root(&self) -> &Path {
        match self {
            Self::Lazy(session) => &session.root,
            Self::Materialized(session) => session.root(),
        }
    }

    fn session_summary(&self) -> SessionSummaryDto {
        match self {
            Self::Lazy(session) => session_summary_dto(
                session.package_id.clone(),
                session.session_id.clone(),
                session.revision.clone(),
                session.root.display().to_string(),
                session.dirty,
                asset_summary_from_package_metadata(&session.metadata),
            ),
            Self::Materialized(session) => session.session_summary(),
        }
    }

    fn session_metadata(&self) -> SessionMetadataDto {
        match self {
            Self::Lazy(session) => session_metadata_dto(
                session.package_id.clone(),
                session.session_id.clone(),
                session.revision.clone(),
                session.root.display().to_string(),
                metadata_dto_from_package_metadata(&session.metadata),
            ),
            Self::Materialized(session) => session_metadata_dto(
                session.package_id().clone(),
                session.session_id().clone(),
                session.revision().clone(),
                session.root().display().to_string(),
                session.metadata_dto(),
            ),
        }
    }

    fn curve_catalog(&self) -> CurveCatalogDto {
        match self {
            Self::Lazy(session) => curve_catalog_result_dto(
                session.package_id.clone(),
                session.session_id.clone(),
                session.revision.clone(),
                session.root.display().to_string(),
                curve_catalog_from_package_metadata(&session.metadata),
            ),
            Self::Materialized(session) => curve_catalog_result_dto(
                session.package_id().clone(),
                session.session_id().clone(),
                session.revision().clone(),
                session.root().display().to_string(),
                session.curve_catalog(),
            ),
        }
    }

    fn read_window(&self, request: &CurveWindowRequest) -> Result<SessionWindowDto> {
        match self {
            Self::Lazy(session) => {
                ensure_lazy_session_current(session)?;
                Ok(session_window_dto(
                    session.package_id.clone(),
                    session.session_id.clone(),
                    session.revision.clone(),
                    session.root.display().to_string(),
                    read_lazy_curve_window(session, request)?,
                ))
            }
            Self::Materialized(session) => Ok(session_window_dto(
                session.package_id().clone(),
                session.session_id().clone(),
                session.revision().clone(),
                session.root().display().to_string(),
                session.read_window(request)?,
            )),
        }
    }

    fn read_depth_window(&self, request: &DepthWindowRequest) -> Result<SessionWindowDto> {
        match self {
            Self::Lazy(session) => {
                ensure_lazy_session_current(session)?;
                Ok(session_window_dto(
                    session.package_id.clone(),
                    session.session_id.clone(),
                    session.revision.clone(),
                    session.root.display().to_string(),
                    read_lazy_depth_window(session, request)?,
                ))
            }
            Self::Materialized(session) => Ok(session_window_dto(
                session.package_id().clone(),
                session.session_id().clone(),
                session.revision().clone(),
                session.root().display().to_string(),
                session.read_depth_window(request)?,
            )),
        }
    }

    fn dirty_state(&self) -> DirtyStateDto {
        match self {
            Self::Lazy(session) => dirty_state_dto(
                session.package_id.clone(),
                session.session_id.clone(),
                session.revision.clone(),
                session.dirty,
            ),
            Self::Materialized(session) => session.dirty_state(),
        }
    }

    fn apply_metadata_update(&mut self, request: &MetadataUpdateRequest) -> Result<()> {
        match self {
            Self::Lazy(session) => {
                ensure_lazy_session_current(session)?;
                let mut candidate = session.metadata.clone();
                apply_metadata_update_to_package_metadata(&mut candidate, request)?;
                session.metadata = candidate;
                session.dirty = true;
                Ok(())
            }
            Self::Materialized(session) => session.apply_metadata_update(request),
        }
    }

    fn apply_curve_edit(&mut self, request: &CurveEditRequest) -> Result<()> {
        self.materialize_for_edit()?.apply_curve_edit(request)
    }

    fn save_checked(&mut self) -> Result<SaveSessionResponseDto> {
        if let Self::Lazy(session) = self {
            if !session.dirty {
                return Ok(SaveSessionResponseDto::Saved(SavePackageResultDto {
                    dto_contract_version: String::from(DTO_CONTRACT_VERSION),
                    package_id: session.package_id.clone(),
                    session_id: session.session_id.clone(),
                    revision: session.revision.clone(),
                    root: session.root.display().to_string(),
                    overwritten: false,
                    dirty_cleared: true,
                    summary: asset_summary_from_package_metadata(&session.metadata),
                }));
            }

            return Ok(SaveSessionResponseDto::Saved(
                save_lazy_metadata_session_in_place(session)?,
            ));
        }

        Ok(SaveSessionResponseDto::Saved(
            self.materialize_for_edit()?.save_checked()?,
        ))
    }

    fn save_as_checked_in_place(&mut self, output_dir: &Path) -> Result<SaveSessionResponseDto> {
        if let Self::Lazy(session) = self {
            return Ok(SaveSessionResponseDto::Saved(
                save_lazy_session_as_in_place(session, output_dir)?,
            ));
        }

        let saved = self.materialize_for_edit()?.save_as_in_place(output_dir)?;
        Ok(SaveSessionResponseDto::Saved(saved))
    }

    fn materialize_for_edit(&mut self) -> Result<&mut PackageSession> {
        if let Self::Lazy(session) = self {
            ensure_lazy_session_current(session)?;
            let materialized = materialize_package_session_from_lazy(session)?;
            *self = Self::Materialized(materialized);
        }

        match self {
            Self::Materialized(session) => Ok(session),
            Self::Lazy(_) => unreachable!("lazy session must materialize before edit"),
        }
    }
}

impl PackageSession {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        open_package(path)
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn revision(&self) -> &RevisionToken {
        &self.revision
    }

    pub fn file(&self) -> &LasFile {
        &self.file
    }

    pub fn summary(&self) -> &LasFileSummary {
        &self.file.summary
    }

    pub fn read_curve(&self, mnemonic: &str) -> Option<&[LasValue]> {
        self.file.curve_data(mnemonic)
    }

    pub fn data(&self) -> &CurveTable {
        &self.table
    }

    pub fn summary_dto(&self) -> AssetSummaryDto {
        asset_summary_dto(&self.file)
    }

    pub fn session_summary(&self) -> SessionSummaryDto {
        session_summary_dto(
            self.package_id.clone(),
            self.session_id.clone(),
            self.revision.clone(),
            self.root.display().to_string(),
            self.dirty,
            self.summary_dto(),
        )
    }

    pub fn dirty_state(&self) -> DirtyStateDto {
        dirty_state_dto(
            self.package_id.clone(),
            self.session_id.clone(),
            self.revision.clone(),
            self.dirty,
        )
    }

    pub fn metadata_dto(&self) -> MetadataDto {
        metadata_dto(&self.file)
    }

    pub fn curve_catalog(&self) -> Vec<CurveCatalogEntryDto> {
        curve_catalog_dto(&self.file)
    }

    pub fn read_window(&self, request: &CurveWindowRequest) -> Result<CurveWindowDto> {
        curve_window_dto(&self.file, request)
    }

    pub fn read_depth_window(&self, request: &DepthWindowRequest) -> Result<CurveWindowDto> {
        let index_values = self
            .file
            .curve_data(&self.file.index.curve_id)
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "index curve '{}' not found in LAS file",
                    self.file.index.curve_id
                ))
            })?;
        let row_window =
            depth_window_request_for_values(&self.file.index.curve_id, index_values, request)?;
        curve_window_dto(&self.file, &row_window)
    }

    pub fn validation_report(&self) -> ValidationReportDto {
        validation_report_dto(&self.file)
    }

    pub fn apply_metadata_update(&mut self, request: &MetadataUpdateRequest) -> Result<()> {
        apply_metadata_update(&mut self.file, request)?;
        self.table = self.file.data();
        self.dirty = true;
        Ok(())
    }

    pub fn apply_curve_edit(&mut self, request: &CurveEditRequest) -> Result<()> {
        apply_curve_edit(&mut self.file, request)?;
        self.table = self.file.data();
        self.dirty = true;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        self.save_checked().map(|_| ())
    }

    pub fn save_checked(&mut self) -> Result<SavePackageResultDto> {
        let session_id = self.session_id.clone();
        // Only replace the in-memory session once the rewritten package can be reopened
        // coherently; failed writes must not partially mutate the current session state.
        let reopened = write_package_internal(&self.file, &self.root, true)?;
        self.replace_from_saved(reopened, session_id.clone());
        let result = SavePackageResultDto {
            dto_contract_version: String::from(DTO_CONTRACT_VERSION),
            package_id: self.package_id.clone(),
            session_id,
            revision: self.revision.clone(),
            root: self.root.display().to_string(),
            overwritten: true,
            dirty_cleared: true,
            summary: self.summary_dto(),
        };
        Ok(result)
    }

    pub fn save_with_result(&mut self) -> Result<SavePackageResultDto> {
        self.save_checked()
    }

    pub fn save_as(&self, output_dir: impl AsRef<Path>) -> Result<PackageSession> {
        write_package_internal(&self.file, output_dir.as_ref(), false)
    }

    pub fn save_as_in_place(
        &mut self,
        output_dir: impl AsRef<Path>,
    ) -> Result<SavePackageResultDto> {
        let session_id = self.session_id.clone();
        // Rebind the current session only after the newly written package reopens cleanly.
        let reopened = write_package_internal(&self.file, output_dir.as_ref(), false)?;
        self.replace_from_saved(reopened, session_id.clone());
        Ok(SavePackageResultDto {
            dto_contract_version: String::from(DTO_CONTRACT_VERSION),
            package_id: self.package_id.clone(),
            session_id,
            revision: self.revision.clone(),
            root: self.root.display().to_string(),
            overwritten: false,
            dirty_cleared: true,
            summary: self.summary_dto(),
        })
    }

    pub fn save_as_with_result(
        &self,
        output_dir: impl AsRef<Path>,
    ) -> Result<SavePackageResultDto> {
        let reopened = write_package_internal(&self.file, output_dir.as_ref(), false)?;
        Ok(SavePackageResultDto {
            dto_contract_version: String::from(DTO_CONTRACT_VERSION),
            package_id: reopened.package_id.clone(),
            session_id: reopened.session_id.clone(),
            revision: reopened.revision.clone(),
            root: reopened.root.display().to_string(),
            overwritten: false,
            dirty_cleared: true,
            summary: reopened.summary_dto(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn replace_from_saved(&mut self, mut reopened: PackageSession, session_id: SessionId) {
        reopened.session_id = session_id;
        reopened.dirty = false;
        *self = reopened;
    }
}

fn open_backend_session(path: impl AsRef<Path>) -> Result<BackendPackageSession> {
    let root = path.as_ref().to_path_buf();
    debug!(package = %root.display(), "opening lazy LAS backend session");

    let metadata = read_package_metadata(&root)?;
    let revision = package_revision(&root)?;
    let reader_metadata = load_parquet_reader_metadata(curves_path(&root))?;
    let root_key = package_path_key(&root);

    Ok(BackendPackageSession::Lazy(LazyPackageSession {
        package_id: package_id_for_path(&root_key),
        session_id: new_session_id(&root),
        revision,
        dirty: false,
        root,
        metadata,
        reader_metadata,
    }))
}

fn materialize_package_session_from_lazy(session: &LazyPackageSession) -> Result<PackageSession> {
    let batch = read_parquet_batch_with_metadata(
        curves_path(&session.root),
        session.reader_metadata.clone(),
        session.metadata.document.summary.row_count,
    )?;
    package_session_from_record_batch(
        session.root.clone(),
        session.package_id.clone(),
        session.session_id.clone(),
        session.revision.clone(),
        session.dirty,
        session.metadata.clone(),
        batch,
    )
}

fn package_session_from_record_batch(
    root: PathBuf,
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    dirty: bool,
    metadata: PackageMetadata,
    batch: RecordBatch,
) -> Result<PackageSession> {
    let descriptors = metadata
        .storage
        .curve_columns
        .iter()
        .map(|curve| CurveColumnDescriptor {
            name: curve.name.clone(),
            storage_kind: curve.storage_kind,
        })
        .collect::<Vec<_>>();
    let table = table_from_record_batch(&batch, &descriptors)?;
    let curves = materialize_curves(&metadata.storage.curve_columns, &table)?;

    Ok(PackageSession {
        package_id,
        session_id,
        revision,
        dirty,
        root,
        file: las_file_from_package_metadata(metadata, curves),
        table,
    })
}

fn las_file_from_package_metadata(metadata: PackageMetadata, curves: Vec<CurveItem>) -> LasFile {
    LasFile {
        summary: metadata.document.summary,
        provenance: metadata.document.provenance,
        encoding: metadata.document.encoding,
        index: metadata.storage.index,
        version: metadata.raw.version,
        well: metadata.raw.well,
        params: metadata.raw.params,
        curves: SectionItems::from_items(curves, metadata.raw.curve_mnemonic_case),
        other: metadata.raw.other,
        extra_sections: metadata.raw.extra_sections,
        issues: metadata.diagnostics.issues,
        index_unit: metadata.storage.index_unit,
    }
}

pub fn open_package(path: impl AsRef<Path>) -> Result<PackageSession> {
    let root = path.as_ref().to_path_buf();
    debug!(package = %root.display(), "opening LAS package");

    let metadata = read_package_metadata(&root)?;
    let revision = package_revision(&root)?;
    let session_id = new_session_id(&root);

    let root_key = package_path_key(&root);
    let batch = read_parquet_batch(curves_path(&root), metadata.document.summary.row_count)?;
    package_session_from_record_batch(
        root,
        package_id_for_path(&root_key),
        session_id,
        revision,
        false,
        metadata,
        batch,
    )
}

pub fn open_package_summary(path: impl AsRef<Path>) -> Result<AssetSummaryDto> {
    let metadata = read_package_metadata(path.as_ref())?;
    Ok(asset_summary_from_package_metadata(&metadata))
}

pub fn open_package_metadata(path: impl AsRef<Path>) -> Result<MetadataDto> {
    let metadata = read_package_metadata(path.as_ref())?;
    Ok(metadata_dto_from_package_metadata(&metadata))
}

pub fn validate_package(path: impl AsRef<Path>) -> Result<ValidationReportDto> {
    let package = open_package(path)?;
    let base = package.validation_report();
    Ok(package_validation_report(base.errors))
}

pub fn write_package(file: &LasFile, output_dir: impl AsRef<Path>) -> Result<PackageSession> {
    write_package_internal(file, output_dir.as_ref(), false)
}

pub fn write_package_overwrite(
    file: &LasFile,
    output_dir: impl AsRef<Path>,
) -> Result<PackageSession> {
    write_package_internal(file, output_dir.as_ref(), true)
}

pub fn write_bundle(file: &LasFile, output_dir: impl AsRef<Path>) -> Result<PackageSession> {
    write_package(file, output_dir)
}

fn write_package_internal(
    file: &LasFile,
    output_dir: &Path,
    overwrite: bool,
) -> Result<PackageSession> {
    validate_edit_state(file)?;
    if output_dir.exists() {
        if overwrite {
            fs::remove_dir_all(output_dir)?;
        } else {
            return Err(LasError::Storage(format!(
                "output directory '{}' already exists",
                output_dir.display()
            )));
        }
    }

    debug!(package = %output_dir.display(), "writing LAS package");
    fs::create_dir_all(output_dir)?;

    let table = file.data();
    let metadata = package_metadata_for(file, PACKAGE_VERSION);

    fs::write(
        metadata_path(output_dir),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    write_parquet_batch(curves_path(output_dir), &table, Some(&file.index.curve_id))?;

    open_package(output_dir)
}

fn metadata_path(root: &Path) -> PathBuf {
    root.join(METADATA_FILENAME)
}

fn curves_path(root: &Path) -> PathBuf {
    root.join(CURVES_FILENAME)
}

fn read_package_metadata(root: &Path) -> Result<PackageMetadata> {
    let metadata_text = fs::read_to_string(metadata_path(root))?;
    let metadata = parse_package_metadata(&metadata_text)?;
    if metadata.package_version() != PACKAGE_VERSION {
        return Err(LasError::Storage(format!(
            "unsupported package version {}",
            metadata.package_version()
        )));
    }
    validate_package_metadata(&metadata)
        .map_err(|err| LasError::Storage(format!("invalid package metadata: {err}")))?;
    Ok(metadata)
}

fn load_parquet_reader_metadata(path: PathBuf) -> Result<ArrowReaderMetadata> {
    let file = File::open(path)?;
    ArrowReaderMetadata::load(
        &file,
        ArrowReaderOptions::new().with_page_index_policy(true.into()),
    )
    .map_err(|err| LasError::Storage(err.to_string()))
}

fn ensure_lazy_session_current(session: &LazyPackageSession) -> Result<()> {
    let current_revision = package_revision(&session.root)?;
    if current_revision != session.revision {
        return Err(LasError::Validation(format!(
            "package '{}' changed since session '{}' was opened; reopen the session",
            session.root.display(),
            session.session_id.0
        )));
    }
    Ok(())
}

fn asset_summary_from_package_metadata(metadata: &PackageMetadata) -> AssetSummaryDto {
    AssetSummaryDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        summary: metadata.document.summary.clone(),
        encoding: metadata.document.encoding.clone(),
        index: metadata.canonical.index.clone(),
    }
}

fn metadata_dto_from_package_metadata(metadata: &PackageMetadata) -> MetadataDto {
    MetadataDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        metadata: metadata.canonical.clone(),
        issues: metadata.diagnostics.issues.clone(),
        extra_sections: metadata.raw.extra_sections.clone(),
    }
}

fn curve_catalog_from_package_metadata(metadata: &PackageMetadata) -> Vec<CurveCatalogEntryDto> {
    metadata
        .storage
        .curve_columns
        .iter()
        .map(|curve| CurveCatalogEntryDto {
            curve_id: revision_token_for_bytes("curve", &curve.name).0,
            name: curve.name.clone(),
            canonical_name: curve.canonical_name.clone(),
            original_mnemonic: curve.original_mnemonic.clone(),
            unit: optional_string(&curve.unit),
            description: optional_string(&curve.description),
            row_count: curve.row_count,
            nullable: curve.nullable,
            storage_kind: curve.storage_kind,
            alias: curve.alias.clone(),
            is_index: curve.is_index,
        })
        .collect()
}

fn read_lazy_curve_window(
    session: &LazyPackageSession,
    request: &CurveWindowRequest,
) -> Result<CurveWindowDto> {
    if request.curve_names.is_empty() {
        return Err(LasError::Validation(String::from(
            "curve window request must include at least one curve",
        )));
    }

    let total_rows = session.metadata.document.summary.row_count;
    let safe_start = request.start_row.min(total_rows);
    let safe_end = request
        .start_row
        .saturating_add(request.row_count)
        .min(total_rows);
    let selected_columns = request
        .curve_names
        .iter()
        .map(|name| {
            session
                .metadata
                .storage
                .curve_columns
                .iter()
                .find(|curve| curve.name == *name)
                .cloned()
                .ok_or_else(|| {
                    LasError::Validation(format!("curve '{name}' not found in LAS file"))
                })
        })
        .collect::<Result<Vec<_>>>()?;

    if safe_start >= safe_end {
        return Ok(CurveWindowDto {
            dto_contract_version: String::from(DTO_CONTRACT_VERSION),
            start_row: request.start_row,
            row_count: 0,
            columns: selected_columns
                .iter()
                .map(|curve| empty_curve_window_column(curve))
                .collect(),
        });
    }

    let projected_indices = selected_columns
        .iter()
        .map(|curve| {
            session
                .metadata
                .storage
                .curve_columns
                .iter()
                .position(|column| column.name == curve.name)
                .ok_or_else(|| {
                    LasError::Storage(format!(
                        "column '{}' missing from package storage descriptors",
                        curve.name
                    ))
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let selection =
        RowSelection::from_consecutive_ranges(iter::once(safe_start..safe_end), total_rows);
    let batch = read_projected_parquet_batch(
        curves_path(&session.root),
        session.reader_metadata.clone(),
        projected_indices,
        selection,
        request.row_count.max(1),
    )?;
    let descriptors = selected_columns
        .iter()
        .map(|curve| CurveColumnDescriptor {
            name: curve.name.clone(),
            storage_kind: curve.storage_kind,
        })
        .collect::<Vec<_>>();
    let table = table_from_record_batch(&batch, &descriptors)?;

    Ok(CurveWindowDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        start_row: request.start_row,
        row_count: table.row_count(),
        columns: selected_columns
            .iter()
            .map(|curve| CurveWindowColumnDto {
                curve_id: revision_token_for_bytes("curve", &curve.name).0,
                name: curve.name.clone(),
                canonical_name: curve.canonical_name.clone(),
                is_index: curve.is_index,
                storage_kind: curve.storage_kind,
                values: table
                    .column(&curve.name)
                    .map(|column| column.values().to_vec())
                    .unwrap_or_default(),
            })
            .collect(),
    })
}

fn read_lazy_depth_window(
    session: &LazyPackageSession,
    request: &DepthWindowRequest,
) -> Result<CurveWindowDto> {
    let index_column = lazy_index_column(session)?;
    let row_window = match regular_depth_window_request_from_metadata(&session.metadata, request)? {
        Some(window) => window,
        None => {
            let index_values = read_lazy_numeric_column(session, &index_column.name)?;
            depth_window_request_for_values(&index_column.name, &index_values, request)?
        }
    };
    read_lazy_curve_window(session, &row_window)
}

fn lazy_index_column(session: &LazyPackageSession) -> Result<&CurveColumnMetadata> {
    session
        .metadata
        .storage
        .curve_columns
        .iter()
        .find(|curve| curve.is_index)
        .ok_or_else(|| {
            LasError::Validation(String::from(
                "package storage metadata must mark exactly one index column",
            ))
        })
}

fn read_lazy_numeric_column(
    session: &LazyPackageSession,
    column_name: &str,
) -> Result<Vec<LasValue>> {
    let total_rows = session.metadata.document.summary.row_count;
    let projected_index = session
        .metadata
        .storage
        .curve_columns
        .iter()
        .position(|curve| curve.name == column_name)
        .ok_or_else(|| {
            LasError::Storage(format!(
                "column '{column_name}' missing from package storage descriptors"
            ))
        })?;
    let curve = session
        .metadata
        .storage
        .curve_columns
        .get(projected_index)
        .ok_or_else(|| {
            LasError::Storage(format!(
                "column '{column_name}' missing from package storage descriptors"
            ))
        })?;
    let selection = RowSelection::from_consecutive_ranges(iter::once(0..total_rows), total_rows);
    let batch = read_projected_parquet_batch(
        curves_path(&session.root),
        session.reader_metadata.clone(),
        vec![projected_index],
        selection,
        total_rows.max(1),
    )?;
    let table = table_from_record_batch(
        &batch,
        &[CurveColumnDescriptor {
            name: curve.name.clone(),
            storage_kind: curve.storage_kind,
        }],
    )?;
    table
        .column(column_name)
        .map(|column| column.values().to_vec())
        .ok_or_else(|| {
            LasError::Storage(format!(
                "column '{column_name}' missing from projected parquet batch"
            ))
        })
}

fn regular_depth_window_request_from_metadata(
    metadata: &PackageMetadata,
    request: &DepthWindowRequest,
) -> Result<Option<CurveWindowRequest>> {
    if metadata.canonical.index.kind != lithos_core::IndexKind::Depth {
        return Ok(None);
    }

    let start = match metadata.canonical.well.start {
        Some(value) if value.is_finite() => value,
        _ => return Ok(None),
    };
    let step = match metadata.canonical.well.step {
        Some(value) if value.is_finite() && value != 0.0 => value,
        _ => return Ok(None),
    };
    let total_rows = metadata.document.summary.row_count;
    if total_rows == 0 {
        return Ok(Some(CurveWindowRequest {
            curve_names: request.curve_names.clone(),
            start_row: 0,
            row_count: 0,
        }));
    }

    let (start_row, row_count) = regular_depth_bounds_to_row_window(
        start,
        step,
        total_rows,
        request.depth_min,
        request.depth_max,
    )?;
    Ok(Some(CurveWindowRequest {
        curve_names: request.curve_names.clone(),
        start_row,
        row_count,
    }))
}

fn regular_depth_bounds_to_row_window(
    start: f64,
    step: f64,
    total_rows: usize,
    depth_min: f64,
    depth_max: f64,
) -> Result<(usize, usize)> {
    if depth_min > depth_max {
        return Err(LasError::Validation(String::from(
            "depth window requires depth_min <= depth_max",
        )));
    }

    let epsilon = step.abs() * 1e-9;
    let (start_row, end_row) = if step > 0.0 {
        let first = ((depth_min - start - epsilon) / step).ceil();
        let last = ((depth_max - start + epsilon) / step).floor();
        (first as isize, last as isize + 1)
    } else {
        let magnitude = step.abs();
        let first = ((start - depth_max - epsilon) / magnitude).ceil();
        let last = ((start - depth_min + epsilon) / magnitude).floor();
        (first as isize, last as isize + 1)
    };

    let safe_start = start_row.clamp(0, total_rows as isize) as usize;
    let safe_end = end_row.clamp(0, total_rows as isize) as usize;
    Ok((safe_start, safe_end.saturating_sub(safe_start)))
}

fn apply_metadata_update_to_package_metadata(
    metadata: &mut PackageMetadata,
    request: &MetadataUpdateRequest,
) -> Result<()> {
    let mut candidate = metadata.clone();
    for item in &request.items {
        let header = HeaderItem::new(
            item.mnemonic.clone(),
            item.unit.clone(),
            item.value.clone(),
            item.description.clone(),
        );
        match item.section {
            MetadataSectionDto::Version => candidate.raw.version.set_item(&item.mnemonic, header),
            MetadataSectionDto::Well => candidate.raw.well.set_item(&item.mnemonic, header),
            MetadataSectionDto::Parameters => candidate.raw.params.set_item(&item.mnemonic, header),
        }
    }

    if let Some(other) = &request.other {
        candidate.raw.other = other.clone();
    }

    refresh_package_metadata_after_metadata_edit(&mut candidate)?;
    *metadata = candidate;
    Ok(())
}

fn refresh_package_metadata_after_metadata_edit(metadata: &mut PackageMetadata) -> Result<()> {
    metadata.document.summary.las_version = metadata_version_string(&metadata.raw.version);
    metadata.document.summary.wrap_mode = metadata_wrap_mode(&metadata.raw.version);
    metadata.document.summary.delimiter =
        metadata_delimiter(&metadata.raw.version, &metadata.document.summary.delimiter);
    metadata.document.summary.curve_count = metadata.storage.curve_columns.len();
    metadata.document.summary.issue_count = metadata.diagnostics.issues.len();
    metadata.canonical = canonical_metadata_from_package(metadata)?;
    validate_package_metadata(metadata)
}

fn canonical_metadata_from_package(metadata: &PackageMetadata) -> Result<CanonicalMetadata> {
    let index_column = metadata
        .storage
        .curve_columns
        .iter()
        .find(|column| column.is_index)
        .ok_or_else(|| {
            LasError::Validation(String::from(
                "package storage metadata must mark exactly one index column",
            ))
        })?;

    Ok(CanonicalMetadata {
        version: VersionInfo {
            vers: header_display_string(metadata.raw.version.get("VERS")),
            wrap: header_display_string(metadata.raw.version.get("WRAP")),
            delimiter: optional_string(&metadata.document.summary.delimiter),
        },
        well: WellInfo {
            well: header_display_string(metadata.raw.well.get("WELL")),
            company: header_display_string(metadata.raw.well.get("COMP")),
            field: header_display_string(metadata.raw.well.get("FLD")),
            location: header_display_string(metadata.raw.well.get("LOC")),
            province: header_display_string(metadata.raw.well.get("PROV")),
            service_company: header_display_string(metadata.raw.well.get("SRVC")),
            date: header_display_string(metadata.raw.well.get("DATE")),
            uwi: header_display_string(metadata.raw.well.get("UWI")),
            api: header_display_string(metadata.raw.well.get("API")),
            start: header_numeric_value(metadata.raw.well.get("STRT")),
            stop: header_numeric_value(metadata.raw.well.get("STOP")),
            step: header_numeric_value(metadata.raw.well.get("STEP")),
            null_value: header_numeric_value(metadata.raw.well.get("NULL")),
        },
        index: IndexInfo {
            name: metadata.storage.index.curve_id.clone(),
            original_mnemonic: metadata.storage.index.raw_mnemonic.clone(),
            canonical_name: String::from("index"),
            unit: optional_string(&index_column.unit)
                .or_else(|| metadata.storage.index_unit.clone()),
            kind: metadata.storage.index.kind.clone(),
            row_count: index_column.row_count,
            nullable: index_column.nullable,
            storage_kind: index_column.storage_kind,
            alias: index_column.alias.clone(),
        },
        curves: metadata
            .storage
            .curve_columns
            .iter()
            .map(|curve| CurveInfo {
                name: curve.name.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                canonical_name: curve.canonical_name.clone(),
                unit: optional_string(&curve.unit),
                description: optional_string(&curve.description),
                header_value: las_value_option(&curve.header_value),
                nullable: curve.nullable,
                storage_kind: curve.storage_kind,
                row_count: curve.row_count,
                alias: curve.alias.clone(),
            })
            .collect(),
        parameters: metadata
            .raw
            .params
            .iter()
            .map(|param| ParameterInfo {
                name: param.mnemonic.clone(),
                original_mnemonic: param.original_mnemonic.clone(),
                unit: optional_string(&param.unit),
                value: las_value_option(&param.value),
                description: optional_string(&param.description),
            })
            .collect(),
        other: optional_string(&metadata.raw.other),
        issue_count: metadata.diagnostics.issues.len(),
    })
}

fn save_lazy_metadata_session_in_place(
    session: &mut LazyPackageSession,
) -> Result<SavePackageResultDto> {
    validate_package_metadata(&session.metadata)?;
    write_package_metadata_file(&session.root, &session.metadata)?;
    let reopened = read_package_metadata(&session.root)?;
    session.metadata = reopened;
    session.revision = package_revision(&session.root)?;
    session.dirty = false;
    Ok(SavePackageResultDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id: session.package_id.clone(),
        session_id: session.session_id.clone(),
        revision: session.revision.clone(),
        root: session.root.display().to_string(),
        overwritten: true,
        dirty_cleared: true,
        summary: asset_summary_from_package_metadata(&session.metadata),
    })
}

fn save_lazy_session_as_in_place(
    session: &mut LazyPackageSession,
    output_dir: &Path,
) -> Result<SavePackageResultDto> {
    validate_package_metadata(&session.metadata)?;
    if output_dir.exists() {
        return Err(LasError::Storage(format!(
            "output directory '{}' already exists",
            output_dir.display()
        )));
    }

    debug!(
        package = %output_dir.display(),
        "writing lazy LAS package copy without sample materialization"
    );
    fs::create_dir_all(output_dir)?;
    write_package_metadata_file(output_dir, &session.metadata)?;
    fs::copy(curves_path(&session.root), curves_path(output_dir))?;

    let reopened = read_package_metadata(output_dir)?;
    let reader_metadata = load_parquet_reader_metadata(curves_path(output_dir))?;
    let new_root = output_dir.to_path_buf();
    let new_root_key = package_path_key(&new_root);
    session.package_id = package_id_for_path(&new_root_key);
    session.revision = package_revision(&new_root)?;
    session.root = new_root;
    session.metadata = reopened;
    session.reader_metadata = reader_metadata;
    session.dirty = false;

    Ok(SavePackageResultDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id: session.package_id.clone(),
        session_id: session.session_id.clone(),
        revision: session.revision.clone(),
        root: session.root.display().to_string(),
        overwritten: false,
        dirty_cleared: true,
        summary: asset_summary_from_package_metadata(&session.metadata),
    })
}

fn write_package_metadata_file(root: &Path, metadata: &PackageMetadata) -> Result<()> {
    fs::write(metadata_path(root), serde_json::to_string_pretty(metadata)?)?;
    Ok(())
}

fn metadata_version_string(version: &SectionItems<HeaderItem>) -> String {
    version
        .get("VERS")
        .and_then(|item| item.value.as_f64())
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("unknown"))
}

fn metadata_wrap_mode(version: &SectionItems<HeaderItem>) -> String {
    version
        .get("WRAP")
        .map(|item| item.value.display_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from("NO"))
}

fn metadata_delimiter(version: &SectionItems<HeaderItem>, current: &str) -> String {
    version
        .get("DLM")
        .map(|item| item.value.display_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| current.to_string())
}

fn header_display_string(item: Option<&HeaderItem>) -> Option<String> {
    item.and_then(|item| las_value_option(&item.value))
}

fn header_numeric_value(item: Option<&HeaderItem>) -> Option<f64> {
    item.and_then(|item| item.value.as_f64())
}

fn las_value_option(value: &LasValue) -> Option<String> {
    match value {
        LasValue::Empty => None,
        _ => Some(value.display_string()),
    }
}

fn package_revision(root: &Path) -> Result<RevisionToken> {
    let metadata_text = fs::read_to_string(metadata_path(root))?;
    let parquet = fs::metadata(curves_path(root))?;
    let modified = parquet
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().to_string())
        .unwrap_or_else(|| String::from("0"));
    let payload = format!("{}:{}:{}", metadata_text, parquet.len(), modified);
    Ok(revision_token_for_bytes("package-revision", &payload))
}

fn new_session_id(root: &Path) -> SessionId {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos().to_string())
        .unwrap_or_else(|_| String::from("0"));
    let RevisionToken(token) =
        revision_token_for_bytes("session", &format!("{}:{now}", root.display()));
    SessionId(token)
}

fn package_path_key(root: &Path) -> String {
    root.canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .display()
        .to_string()
}

fn write_parquet_batch(
    path: PathBuf,
    table: &CurveTable,
    index_curve_name: Option<&str>,
) -> Result<()> {
    let batch = table_to_record_batch(table)?;
    let file = File::create(path)?;
    let props = curve_writer_properties(table, index_curve_name);
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))
        .map_err(|err| LasError::Storage(err.to_string()))?;
    writer
        .write(&batch)
        .map_err(|err| LasError::Storage(err.to_string()))?;
    writer
        .close()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    Ok(())
}

fn curve_writer_properties(table: &CurveTable, index_curve_name: Option<&str>) -> WriterProperties {
    let sorting_columns =
        index_curve_name.and_then(|index_name| sorting_columns_for_index(table, index_name));

    WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_write_page_header_statistics(false)
        .set_offset_index_disabled(false)
        .set_max_row_group_row_count(Some(CURVES_ROW_GROUP_ROW_COUNT))
        .set_data_page_row_count_limit(CURVES_DATA_PAGE_ROW_COUNT_LIMIT)
        .set_sorting_columns(sorting_columns)
        .build()
}

fn sorting_columns_for_index(
    table: &CurveTable,
    index_curve_name: &str,
) -> Option<Vec<SortingColumn>> {
    let column_index = table
        .column_names()
        .iter()
        .position(|name| name == index_curve_name)?;
    let numeric_values = table.column(index_curve_name)?.numeric_values()?;
    if numeric_values.is_empty() {
        return Some(vec![SortingColumn {
            column_idx: column_index as i32,
            descending: false,
            nulls_first: false,
        }]);
    }

    let non_decreasing = numeric_values.windows(2).all(|pair| pair[0] <= pair[1]);
    if non_decreasing {
        return Some(vec![SortingColumn {
            column_idx: column_index as i32,
            descending: false,
            nulls_first: false,
        }]);
    }

    let non_increasing = numeric_values.windows(2).all(|pair| pair[0] >= pair[1]);
    if non_increasing {
        return Some(vec![SortingColumn {
            column_idx: column_index as i32,
            descending: true,
            nulls_first: false,
        }]);
    }

    None
}

fn read_parquet_batch(path: PathBuf, row_count: usize) -> Result<RecordBatch> {
    let file = File::open(path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|err| LasError::Storage(err.to_string()))?
        .with_batch_size(row_count.max(1))
        .build()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    let mut batches = reader
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    match batches.len() {
        0 => Err(LasError::Storage(String::from(
            "package parquet data did not contain any record batches",
        ))),
        1 => Ok(batches.remove(0)),
        _ => merge_batches(batches),
    }
}

fn read_parquet_batch_with_metadata(
    path: PathBuf,
    reader_metadata: ArrowReaderMetadata,
    row_count: usize,
) -> Result<RecordBatch> {
    let file = File::open(path)?;
    let reader = ParquetRecordBatchReaderBuilder::new_with_metadata(file, reader_metadata)
        .with_batch_size(row_count.max(1))
        .build()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    let mut batches = reader
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    match batches.len() {
        0 => Err(LasError::Storage(String::from(
            "package parquet data did not contain any record batches",
        ))),
        1 => Ok(batches.remove(0)),
        _ => merge_batches(batches),
    }
}

fn read_projected_parquet_batch(
    path: PathBuf,
    reader_metadata: ArrowReaderMetadata,
    projected_indices: Vec<usize>,
    selection: RowSelection,
    batch_size: usize,
) -> Result<RecordBatch> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::new_with_metadata(file, reader_metadata);
    let mask = ProjectionMask::roots(builder.parquet_schema(), projected_indices);
    let reader = builder
        .with_projection(mask)
        .with_row_selection(selection)
        .with_batch_size(batch_size.max(1))
        .build()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    let mut batches = reader
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    match batches.len() {
        0 => Err(LasError::Storage(String::from(
            "projected parquet read did not return any record batches",
        ))),
        1 => Ok(batches.remove(0)),
        _ => merge_batches(batches),
    }
}

fn merge_batches(batches: Vec<RecordBatch>) -> Result<RecordBatch> {
    let first = batches
        .first()
        .ok_or_else(|| LasError::Storage(String::from("no batches to merge")))?;
    let schema = first.schema();
    let mut merged = Vec::with_capacity(schema.fields().len());

    for column_index in 0..schema.fields().len() {
        match schema.field(column_index).data_type() {
            DataType::Float64 => {
                let mut values = Vec::new();
                for batch in &batches {
                    let array = batch
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<Float64Array>()
                        .ok_or_else(|| {
                            LasError::Storage(String::from(
                                "expected Float64Array while merging parquet batches",
                            ))
                        })?;
                    values.extend((0..array.len()).map(|row| {
                        if array.is_null(row) {
                            None
                        } else {
                            Some(array.value(row))
                        }
                    }));
                }
                merged.push(Arc::new(Float64Array::from(values)) as ArrayRef);
            }
            DataType::Utf8 => {
                let mut values = Vec::new();
                for batch in &batches {
                    let array = batch
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .ok_or_else(|| {
                            LasError::Storage(String::from(
                                "expected StringArray while merging parquet batches",
                            ))
                        })?;
                    values.extend((0..array.len()).map(|row| {
                        if array.is_null(row) {
                            None
                        } else {
                            Some(array.value(row).to_string())
                        }
                    }));
                }
                merged.push(Arc::new(StringArray::from_iter(values)) as ArrayRef);
            }
            other => {
                return Err(LasError::Storage(format!(
                    "unsupported parquet column type during merge: {other:?}"
                )));
            }
        }
    }

    RecordBatch::try_new(schema, merged).map_err(|err| LasError::Storage(err.to_string()))
}

fn materialize_curves(
    curves: &[CurveColumnMetadata],
    table: &CurveTable,
) -> Result<Vec<CurveItem>> {
    curves
        .iter()
        .map(|curve| {
            let values = table
                .column(&curve.name)
                .ok_or_else(|| {
                    LasError::Storage(format!(
                        "column '{}' missing from package table",
                        curve.name
                    ))
                })?
                .values()
                .to_vec();
            Ok(CurveItem {
                mnemonic: curve.name.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                unit: curve.unit.clone(),
                value: curve.header_value.clone(),
                description: curve.description.clone(),
                data: values,
            })
        })
        .collect()
}

fn table_to_record_batch(table: &CurveTable) -> Result<RecordBatch> {
    let descriptors = table.descriptors();
    let schema = Arc::new(Schema::new(
        descriptors
            .iter()
            .map(|column| {
                let data_type = match column.storage_kind {
                    CurveStorageKind::Numeric => DataType::Float64,
                    CurveStorageKind::Text | CurveStorageKind::Mixed => DataType::Utf8,
                };
                Field::new(&column.name, data_type, true)
            })
            .collect::<Vec<_>>(),
    ));

    let arrays = table
        .column_names()
        .iter()
        .map(|name| {
            let column = table
                .column(name)
                .ok_or_else(|| LasError::Storage(format!("column '{name}' missing from table")))?;
            match column.storage_kind() {
                CurveStorageKind::Numeric => {
                    let values = column
                        .values()
                        .iter()
                        .map(|value| match value {
                            LasValue::Number(number) => Some(*number),
                            LasValue::Empty => None,
                            LasValue::Text(text) => text.parse::<f64>().ok(),
                        })
                        .collect::<Vec<_>>();
                    Ok(Arc::new(Float64Array::from(values)) as ArrayRef)
                }
                CurveStorageKind::Text | CurveStorageKind::Mixed => {
                    let values = column.values().iter().map(|value| match value {
                        LasValue::Number(number) => Some(number.to_string()),
                        LasValue::Text(text) => Some(text.clone()),
                        LasValue::Empty => None,
                    });
                    Ok(Arc::new(StringArray::from_iter(values)) as ArrayRef)
                }
            }
        })
        .collect::<Result<Vec<_>>>()?;

    RecordBatch::try_new(schema, arrays).map_err(|err| LasError::Storage(err.to_string()))
}

fn table_from_record_batch(
    batch: &RecordBatch,
    descriptors: &[CurveColumnDescriptor],
) -> Result<CurveTable> {
    let columns = descriptors
        .iter()
        .map(|descriptor| {
            let index = batch.schema().index_of(&descriptor.name).map_err(|err| {
                LasError::Storage(format!(
                    "column '{}' missing from parquet data: {err}",
                    descriptor.name
                ))
            })?;
            let values = values_from_array(batch.column(index).as_ref(), descriptor.storage_kind)?;
            Ok(CurveItem {
                mnemonic: descriptor.name.clone(),
                original_mnemonic: descriptor.name.clone(),
                unit: String::new(),
                value: LasValue::Empty,
                description: String::new(),
                data: values,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(CurveTable::from_curves(&columns))
}

fn values_from_array(array: &dyn Array, storage_kind: CurveStorageKind) -> Result<Vec<LasValue>> {
    match storage_kind {
        CurveStorageKind::Numeric => {
            let numbers = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .ok_or_else(|| {
                    LasError::Storage(String::from(
                        "expected Float64Array for numeric parquet column",
                    ))
                })?;
            Ok((0..numbers.len())
                .map(|index| {
                    if numbers.is_null(index) {
                        LasValue::Empty
                    } else {
                        LasValue::Number(numbers.value(index))
                    }
                })
                .collect())
        }
        CurveStorageKind::Text | CurveStorageKind::Mixed => {
            let strings = array
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| {
                    LasError::Storage(String::from(
                        "expected StringArray for text or mixed parquet column",
                    ))
                })?;
            Ok((0..strings.len())
                .map(|index| {
                    if strings.is_null(index) {
                        LasValue::Empty
                    } else {
                        let value = strings.value(index).to_string();
                        if storage_kind == CurveStorageKind::Mixed {
                            value
                                .parse::<f64>()
                                .map(LasValue::Number)
                                .unwrap_or_else(|_| LasValue::Text(value))
                        } else {
                            LasValue::Text(value)
                        }
                    }
                })
                .collect())
        }
    }
}

fn empty_curve_window_column(curve: &CurveColumnMetadata) -> CurveWindowColumnDto {
    CurveWindowColumnDto {
        curve_id: revision_token_for_bytes("curve", &curve.name).0,
        name: curve.name.clone(),
        canonical_name: curve.canonical_name.clone(),
        is_index: curve.is_index,
        storage_kind: curve.storage_kind,
        values: Vec::new(),
    }
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
