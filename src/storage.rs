use arrow_array::Array;
use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use lithos_core::{
    AssetSummaryDto, CurveCatalogDto, CurveCatalogEntryDto, CurveColumnMetadata, CurveEditRequest,
    CurveItem, CurveStorageKind, CurveTable, CurveWindowColumnDto, CurveWindowDto,
    CurveWindowRequest, DTO_CONTRACT_VERSION, DirtyStateDto, LasError, LasFile, LasFileSummary,
    LasValue, MetadataDto, MetadataUpdateRequest, PackageId, PackageMetadata, Result,
    RevisionToken, SaveConflictDto, SavePackageResultDto, SaveSessionResponseDto, SectionItems,
    SessionId, SessionMetadataDto, SessionSummaryDto, SessionWindowDto, ValidationReportDto,
    apply_curve_edit, apply_metadata_update, asset_summary_dto, curve_catalog_dto,
    curve_catalog_result_dto, curve_window_dto, dirty_state_dto, metadata_dto, package_id_for_path,
    package_metadata_for, package_validation_report, parse_package_metadata,
    revision_token_for_bytes, save_conflict_dto, session_metadata_dto, session_summary_dto,
    session_window_dto, validate_edit_state, validate_package_metadata, validation_report_dto,
};
use lithos_table::CurveColumnDescriptor;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::{
    ArrowReaderMetadata, ArrowReaderOptions, ParquetRecordBatchReaderBuilder, RowSelection,
};
use parquet::arrow::arrow_writer::ArrowWriter;
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
        if let SaveSessionResponseDto::Saved(saved) = &response {
            self.rebind_path(session_id, &old_root, Path::new(&saved.root));
        }
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
        self.materialize_for_edit()?.apply_metadata_update(request)
    }

    fn apply_curve_edit(&mut self, request: &CurveEditRequest) -> Result<()> {
        self.materialize_for_edit()?.apply_curve_edit(request)
    }

    fn save_checked(&mut self) -> Result<SaveSessionResponseDto> {
        if let Self::Lazy(session) = self {
            let current_revision = package_revision(&session.root)?;
            if current_revision != session.revision {
                return Ok(SaveSessionResponseDto::Conflict(save_conflict_dto(
                    session.package_id.clone(),
                    session.session_id.clone(),
                    session.revision.clone(),
                    current_revision,
                    session.root.display().to_string(),
                )));
            }

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
        }

        match self.materialize_for_edit()?.save_checked()? {
            Ok(saved) => Ok(SaveSessionResponseDto::Saved(saved)),
            Err(conflict) => Ok(SaveSessionResponseDto::Conflict(conflict)),
        }
    }

    fn save_as_checked_in_place(&mut self, output_dir: &Path) -> Result<SaveSessionResponseDto> {
        if let Self::Lazy(session) = self {
            let current_revision = package_revision(&session.root)?;
            if current_revision != session.revision {
                return Ok(SaveSessionResponseDto::Conflict(save_conflict_dto(
                    session.package_id.clone(),
                    session.session_id.clone(),
                    session.revision.clone(),
                    current_revision,
                    session.root.display().to_string(),
                )));
            }
        }

        let saved = self.materialize_for_edit()?.save_as_in_place(output_dir)?;
        Ok(SaveSessionResponseDto::Saved(saved))
    }

    fn materialize_for_edit(&mut self) -> Result<&mut PackageSession> {
        if let Self::Lazy(session) = self {
            ensure_lazy_session_current(session)?;
            let mut materialized = open_package(&session.root)?;
            materialized.package_id = session.package_id.clone();
            materialized.session_id = session.session_id.clone();
            materialized.revision = session.revision.clone();
            materialized.dirty = session.dirty;
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
        match self.save_checked()? {
            Ok(_) => Ok(()),
            Err(conflict) => Err(LasError::Validation(format!(
                "save conflict for session '{}': expected {}, found {}",
                conflict.session_id.0, conflict.expected_revision.0, conflict.actual_revision.0
            ))),
        }
    }

    pub fn save_checked(
        &mut self,
    ) -> Result<std::result::Result<SavePackageResultDto, SaveConflictDto>> {
        let current_revision = package_revision(&self.root)?;
        if current_revision != self.revision {
            return Ok(Err(save_conflict_dto(
                self.package_id.clone(),
                self.session_id.clone(),
                self.revision.clone(),
                current_revision,
                self.root.display().to_string(),
            )));
        }

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
        Ok(Ok(result))
    }

    pub fn save_with_result(&mut self) -> Result<SavePackageResultDto> {
        match self.save_checked()? {
            Ok(result) => Ok(result),
            Err(conflict) => Err(LasError::Validation(format!(
                "save conflict for session '{}': expected {}, found {}",
                conflict.session_id.0, conflict.expected_revision.0, conflict.actual_revision.0
            ))),
        }
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

pub fn open_package(path: impl AsRef<Path>) -> Result<PackageSession> {
    let root = path.as_ref().to_path_buf();
    debug!(package = %root.display(), "opening LAS package");

    let metadata = read_package_metadata(&root)?;
    let revision = package_revision(&root)?;

    let batch = read_parquet_batch(curves_path(&root), metadata.document.summary.row_count)?;
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

    let root_key = package_path_key(&root);
    Ok(PackageSession {
        package_id: package_id_for_path(&root_key),
        session_id: new_session_id(&root),
        revision,
        dirty: false,
        root,
        file: LasFile {
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
        },
        table,
    })
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
    write_parquet_batch(curves_path(output_dir), &table)?;

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
    ArrowReaderMetadata::load(&file, ArrowReaderOptions::new())
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

fn write_parquet_batch(path: PathBuf, table: &CurveTable) -> Result<()> {
    let batch = table_to_record_batch(table)?;
    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None)
        .map_err(|err| LasError::Storage(err.to_string()))?;
    writer
        .write(&batch)
        .map_err(|err| LasError::Storage(err.to_string()))?;
    writer
        .close()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    Ok(())
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
