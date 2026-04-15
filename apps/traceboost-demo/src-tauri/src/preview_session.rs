use std::sync::{Arc, Mutex};

use seis_contracts_operations::processing_ops::{
    PreviewTraceLocalProcessingRequest, PreviewTraceLocalProcessingResponse,
};
use seis_contracts_operations::resolve::IPC_SCHEMA_VERSION;
use seis_runtime::{PreviewSectionPrefixReuse, PreviewSectionSession, PreviewView};
use traceboost_app::preview_processing_label;

struct ActivePreviewSession {
    store_path: String,
    dataset_id: String,
    session: PreviewSectionSession,
}

#[derive(Clone)]
pub struct PreviewSessionState {
    active: Arc<Mutex<Option<ActivePreviewSession>>>,
}

impl Default for PreviewSessionState {
    fn default() -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
        }
    }
}

impl PreviewSessionState {
    pub fn preview_processing(
        &self,
        request: PreviewTraceLocalProcessingRequest,
    ) -> Result<
        (
            PreviewTraceLocalProcessingResponse,
            PreviewSectionPrefixReuse,
        ),
        String,
    > {
        let mut active = self.active.lock().expect("preview session mutex poisoned");
        let session = ensure_active_session(&mut active, &request)?;
        let (section, reuse) = session
            .session
            .preview_section_view_with_prefix_cache(
                request.section.axis,
                request.section.index,
                &request.pipeline.operations().cloned().collect::<Vec<_>>(),
            )
            .map_err(|error| error.to_string())?;
        let processing_label = preview_processing_label(&request.pipeline);
        Ok((
            PreviewTraceLocalProcessingResponse {
                schema_version: IPC_SCHEMA_VERSION,
                preview: PreviewView {
                    section,
                    processing_label,
                    preview_ready: true,
                },
                pipeline: request.pipeline,
            },
            reuse,
        ))
    }
}

fn ensure_active_session<'a>(
    active: &'a mut Option<ActivePreviewSession>,
    request: &PreviewTraceLocalProcessingRequest,
) -> Result<&'a mut ActivePreviewSession, String> {
    let needs_new_session = active.as_ref().map_or(true, |session| {
        session.store_path != request.store_path
            || session.dataset_id != request.section.dataset_id.0
    });
    if needs_new_session {
        let session =
            PreviewSectionSession::open(&request.store_path).map_err(|error| error.to_string())?;
        let dataset_id = session.dataset_id().0;
        if dataset_id != request.section.dataset_id.0 {
            return Err(format!(
                "Section request dataset mismatch: expected {}, found {}",
                request.section.dataset_id.0, dataset_id
            ));
        }
        *active = Some(ActivePreviewSession {
            store_path: request.store_path.clone(),
            dataset_id,
            session,
        });
    }
    Ok(active
        .as_mut()
        .expect("active preview session should exist after initialization"))
}
