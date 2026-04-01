use crate::{
    ChunkProcessingError, ChunkReadConfig, Cube, ReadError, SegyReader, TraceChunkIter,
    TraceSelection,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TraceExportMetadata {
    pub start_trace: u64,
    pub trace_count: usize,
    pub samples_per_trace: usize,
    pub sample_interval_us: u16,
    pub sample_axis_ms: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportTraceChunk<T> {
    pub start_trace: u64,
    pub trace_count: usize,
    pub samples_per_trace: usize,
    pub data: Vec<T>,
}

impl<T> ExportTraceChunk<T> {
    pub fn trace(&self, trace_index: usize) -> &[T] {
        let start = trace_index * self.samples_per_trace;
        let end = start + self.samples_per_trace;
        &self.data[start..end]
    }
}

#[derive(Debug)]
pub struct ExportTraceChunkIter {
    inner: TraceChunkIter,
}

impl ExportTraceChunkIter {
    pub(crate) fn new(inner: TraceChunkIter) -> Self {
        Self { inner }
    }
}

impl Iterator for ExportTraceChunkIter {
    type Item = Result<ExportTraceChunk<f32>, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            result.map(|chunk| ExportTraceChunk {
                start_trace: chunk.start_trace,
                trace_count: chunk.trace_count(),
                samples_per_trace: chunk.samples_per_trace,
                data: chunk.data,
            })
        })
    }
}

impl SegyReader {
    pub fn trace_export_metadata(
        &self,
        selection: TraceSelection,
    ) -> Result<TraceExportMetadata, ReadError> {
        let (start_trace, end_trace) = selection.resolve(self.summary().trace_count)?;
        Ok(TraceExportMetadata {
            start_trace,
            trace_count: usize::try_from(end_trace - start_trace)
                .map_err(|_| ReadError::TraceCountOverflow)?,
            samples_per_trace: self.summary().samples_per_trace as usize,
            sample_interval_us: self.resolved_sample_interval_us(),
            sample_axis_ms: self.sample_axis_ms(),
        })
    }

    pub fn export_trace_chunks(
        &self,
        config: ChunkReadConfig,
    ) -> Result<ExportTraceChunkIter, ReadError> {
        Ok(ExportTraceChunkIter::new(self.read_trace_chunks(config)?))
    }

    pub fn export_trace_chunks_into<E, F>(
        &self,
        config: ChunkReadConfig,
        scratch: &mut [f32],
        mut sink: F,
    ) -> Result<(), ChunkProcessingError<E>>
    where
        F: FnMut(ExportTraceChunkRef<'_>) -> Result<(), E>,
    {
        self.process_trace_chunks_into(config, scratch, |chunk| {
            sink(ExportTraceChunkRef {
                start_trace: chunk.start_trace,
                trace_count: chunk.trace_count,
                samples_per_trace: chunk.samples_per_trace,
                data: chunk.data,
            })
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExportTraceChunkRef<'a> {
    pub start_trace: u64,
    pub trace_count: usize,
    pub samples_per_trace: usize,
    pub data: &'a [f32],
}

impl<'a> ExportTraceChunkRef<'a> {
    pub fn trace(&self, trace_index: usize) -> &[f32] {
        let start = trace_index * self.samples_per_trace;
        let end = start + self.samples_per_trace;
        &self.data[start..end]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubeExportMetadata {
    pub ilines: Vec<i64>,
    pub xlines: Vec<i64>,
    pub offsets: Vec<i64>,
    pub samples_per_trace: usize,
    pub sample_interval_us: u16,
    pub sample_axis_ms: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubeChunkShape {
    pub iline_count: usize,
    pub xline_count: usize,
    pub offset_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubeChunkDescriptor {
    pub iline_start: usize,
    pub iline_count: usize,
    pub xline_start: usize,
    pub xline_count: usize,
    pub offset_start: usize,
    pub offset_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubeExportChunk<T> {
    pub descriptor: CubeChunkDescriptor,
    pub samples_per_trace: usize,
    pub data: Vec<T>,
}

impl<T> CubeExportChunk<T> {
    pub fn dimensions(&self) -> (usize, usize, usize, usize) {
        (
            self.descriptor.iline_count,
            self.descriptor.xline_count,
            self.descriptor.offset_count,
            self.samples_per_trace,
        )
    }
}

impl<T: Copy> Cube<T> {
    pub fn export_metadata(&self) -> CubeExportMetadata {
        CubeExportMetadata {
            ilines: self.ilines.clone(),
            xlines: self.xlines.clone(),
            offsets: self.offsets.clone(),
            samples_per_trace: self.samples_per_trace,
            sample_interval_us: self.sample_interval_us,
            sample_axis_ms: self.sample_axis_ms.clone(),
        }
    }

    pub fn chunk_descriptors(
        &self,
        shape: CubeChunkShape,
    ) -> Result<Vec<CubeChunkDescriptor>, ReadError> {
        if shape.iline_count == 0 || shape.xline_count == 0 || shape.offset_count == 0 {
            return Err(ReadError::InvalidChunkSize);
        }

        let mut descriptors = Vec::new();
        let iline_total = self.ilines.len();
        let xline_total = self.xlines.len();
        let offset_total = self.offsets.len();

        let mut iline_start = 0;
        while iline_start < iline_total {
            let iline_count = (iline_total - iline_start).min(shape.iline_count);
            let mut xline_start = 0;
            while xline_start < xline_total {
                let xline_count = (xline_total - xline_start).min(shape.xline_count);
                let mut offset_start = 0;
                while offset_start < offset_total {
                    let offset_count = (offset_total - offset_start).min(shape.offset_count);
                    descriptors.push(CubeChunkDescriptor {
                        iline_start,
                        iline_count,
                        xline_start,
                        xline_count,
                        offset_start,
                        offset_count,
                    });
                    offset_start += shape.offset_count;
                }
                xline_start += shape.xline_count;
            }
            iline_start += shape.iline_count;
        }

        Ok(descriptors)
    }

    pub fn export_chunk(
        &self,
        descriptor: CubeChunkDescriptor,
    ) -> Result<CubeExportChunk<T>, ReadError> {
        let trace_count = descriptor.iline_count * descriptor.xline_count * descriptor.offset_count;
        let expected_len = trace_count * self.samples_per_trace;
        let mut data = Vec::with_capacity(expected_len);
        if expected_len > 0 {
            data.resize(expected_len, self.data[0]);
        }
        self.export_chunk_into(descriptor, &mut data)?;

        Ok(CubeExportChunk {
            descriptor,
            samples_per_trace: self.samples_per_trace,
            data,
        })
    }

    pub fn export_chunk_into(
        &self,
        descriptor: CubeChunkDescriptor,
        dst: &mut [T],
    ) -> Result<(), ReadError> {
        let iline_end = descriptor.iline_start + descriptor.iline_count;
        let xline_end = descriptor.xline_start + descriptor.xline_count;
        let offset_end = descriptor.offset_start + descriptor.offset_count;

        if descriptor.iline_count == 0
            || descriptor.xline_count == 0
            || descriptor.offset_count == 0
            || iline_end > self.ilines.len()
            || xline_end > self.xlines.len()
            || offset_end > self.offsets.len()
        {
            return Err(ReadError::InvalidChunkSize);
        }

        let trace_count = descriptor.iline_count * descriptor.xline_count * descriptor.offset_count;
        let expected_len = trace_count * self.samples_per_trace;
        if dst.len() != expected_len {
            return Err(ReadError::InvalidDestinationBuffer {
                actual_len: dst.len(),
                expected_len,
            });
        }

        let mut dst_trace = 0usize;
        for iline_index in descriptor.iline_start..iline_end {
            for xline_index in descriptor.xline_start..xline_end {
                for offset_index in descriptor.offset_start..offset_end {
                    let dst_start = dst_trace * self.samples_per_trace;
                    let dst_end = dst_start + self.samples_per_trace;
                    dst[dst_start..dst_end].copy_from_slice(self.trace(
                        iline_index,
                        xline_index,
                        offset_index,
                    ));
                    dst_trace += 1;
                }
            }
        }

        Ok(())
    }
}
