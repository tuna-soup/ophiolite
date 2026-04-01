use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{Cube, CubeChunkDescriptor, CubeChunkShape, ReadError, SegyReader, TraceSelection};

#[derive(Debug, Clone, PartialEq)]
pub struct Hdf5CubeLayout {
    pub shape: (usize, usize, usize, usize),
    pub chunk_shape: (usize, usize, usize, usize),
    pub ilines: Vec<i64>,
    pub xlines: Vec<i64>,
    pub offsets: Vec<i64>,
    pub sample_interval_us: u16,
    pub sample_axis_ms: Vec<f32>,
}

pub trait Hdf5CubeWriter {
    type Error;

    fn write_layout(&mut self, layout: &Hdf5CubeLayout) -> Result<(), Self::Error>;

    fn write_chunk(
        &mut self,
        descriptor: CubeChunkDescriptor,
        samples_per_trace: usize,
        data: &[f32],
    ) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum Hdf5CubeWriteError<E> {
    Read(ReadError),
    Sink(E),
}

impl<E: Display> Display for Hdf5CubeWriteError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => write!(f, "{error}"),
            Self::Sink(error) => write!(f, "{error}"),
        }
    }
}

impl<E: Error + 'static> Error for Hdf5CubeWriteError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Sink(error) => Some(error),
        }
    }
}

impl SegyReader {
    pub fn plan_hdf5_cube_layout(
        &self,
        chunk_shape: CubeChunkShape,
    ) -> Result<Hdf5CubeLayout, ReadError> {
        if chunk_shape.iline_count == 0
            || chunk_shape.xline_count == 0
            || chunk_shape.offset_count == 0
        {
            return Err(ReadError::InvalidChunkSize);
        }

        let inline_field = self.header_mapping().inline_3d();
        let crossline_field = self.header_mapping().crossline_3d();
        let offset_field = self.header_mapping().offset();
        let headers = self.load_trace_headers(
            &[inline_field, crossline_field, offset_field],
            TraceSelection::All,
        )?;
        let ilines = sorted_unique(headers.column(inline_field).ok_or(
            ReadError::MissingGeometryField {
                field: inline_field.name,
            },
        )?);
        let xlines = sorted_unique(headers.column(crossline_field).ok_or(
            ReadError::MissingGeometryField {
                field: crossline_field.name,
            },
        )?);
        let offsets = sorted_unique(headers.column(offset_field).ok_or(
            ReadError::MissingGeometryField {
                field: offset_field.name,
            },
        )?);

        let expected_trace_count = ilines.len() * xlines.len() * offsets.len();
        if expected_trace_count != headers.rows() {
            return Err(ReadError::IrregularGeometry {
                trace_count: headers.rows(),
                expected_trace_count,
            });
        }

        Ok(Hdf5CubeLayout {
            shape: (
                ilines.len(),
                xlines.len(),
                offsets.len(),
                self.summary().samples_per_trace as usize,
            ),
            chunk_shape: (
                chunk_shape.iline_count.min(ilines.len()),
                chunk_shape.xline_count.min(xlines.len()),
                chunk_shape.offset_count.min(offsets.len()),
                self.summary().samples_per_trace as usize,
            ),
            ilines,
            xlines,
            offsets,
            sample_interval_us: self.resolved_sample_interval_us(),
            sample_axis_ms: self.sample_axis_ms(),
        })
    }
}

impl Cube<f32> {
    pub fn hdf5_layout(&self, chunk_shape: CubeChunkShape) -> Result<Hdf5CubeLayout, ReadError> {
        if chunk_shape.iline_count == 0
            || chunk_shape.xline_count == 0
            || chunk_shape.offset_count == 0
        {
            return Err(ReadError::InvalidChunkSize);
        }

        Ok(Hdf5CubeLayout {
            shape: self.dimensions(),
            chunk_shape: (
                chunk_shape.iline_count.min(self.ilines.len()),
                chunk_shape.xline_count.min(self.xlines.len()),
                chunk_shape.offset_count.min(self.offsets.len()),
                self.samples_per_trace,
            ),
            ilines: self.ilines.clone(),
            xlines: self.xlines.clone(),
            offsets: self.offsets.clone(),
            sample_interval_us: self.sample_interval_us,
            sample_axis_ms: self.sample_axis_ms.clone(),
        })
    }

    pub fn write_hdf5_like<W>(
        &self,
        chunk_shape: CubeChunkShape,
        writer: &mut W,
    ) -> Result<(), Hdf5CubeWriteError<W::Error>>
    where
        W: Hdf5CubeWriter,
    {
        let layout = self
            .hdf5_layout(chunk_shape)
            .map_err(Hdf5CubeWriteError::Read)?;
        writer
            .write_layout(&layout)
            .map_err(Hdf5CubeWriteError::Sink)?;

        let descriptors = self
            .chunk_descriptors(chunk_shape)
            .map_err(Hdf5CubeWriteError::Read)?;
        let mut scratch = Vec::new();

        for descriptor in descriptors {
            let trace_count =
                descriptor.iline_count * descriptor.xline_count * descriptor.offset_count;
            let expected_len = trace_count * self.samples_per_trace;
            if scratch.len() != expected_len {
                scratch.resize(expected_len, 0.0);
            }

            self.export_chunk_into(descriptor, &mut scratch)
                .map_err(Hdf5CubeWriteError::Read)?;
            writer
                .write_chunk(descriptor, self.samples_per_trace, &scratch)
                .map_err(Hdf5CubeWriteError::Sink)?;
        }

        Ok(())
    }
}

fn sorted_unique(values: &[i64]) -> Vec<i64> {
    let mut values = values.to_vec();
    values.sort_unstable();
    values.dedup();
    values
}
