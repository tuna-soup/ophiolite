use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::{Endianness, SampleFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureCase {
    pub path: &'static str,
    pub endianness: Endianness,
    pub sample_format: SampleFormat,
    pub samples_per_trace: u16,
    pub trace_count: u64,
    pub extended_textual_headers: i16,
}

const CURATED_FIXTURES: &[FixtureCase] = &[
    FixtureCase {
        path: "small.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 50,
        trace_count: 25,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "small-lsb.sgy",
        endianness: Endianness::Little,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 50,
        trace_count: 25,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "f3.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::Int16,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "f3-lsb.sgy",
        endianness: Endianness::Little,
        sample_format: SampleFormat::Int16,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "small-ps.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 10,
        trace_count: 24,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "shot-gather.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 25,
        trace_count: 61,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multi-text.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 1,
        trace_count: 1,
        extended_textual_headers: 4,
    },
    FixtureCase {
        path: "text-embed-null.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 50,
        trace_count: 25,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "long.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 60_000,
        trace_count: 3,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format1msb.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format1lsb.sgy",
        endianness: Endianness::Little,
        sample_format: SampleFormat::IbmFloat32,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format3msb.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::Int16,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format3lsb.sgy",
        endianness: Endianness::Little,
        sample_format: SampleFormat::Int16,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format5msb.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::IeeeFloat32,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format5lsb.sgy",
        endianness: Endianness::Little,
        sample_format: SampleFormat::IeeeFloat32,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format8msb.sgy",
        endianness: Endianness::Big,
        sample_format: SampleFormat::Int8,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
    FixtureCase {
        path: "multiformats/Format8lsb.sgy",
        endianness: Endianness::Little,
        sample_format: SampleFormat::Int8,
        samples_per_trace: 75,
        trace_count: 414,
        extended_textual_headers: 0,
    },
];

pub fn curated_fixtures() -> &'static [FixtureCase] {
    CURATED_FIXTURES
}

pub fn write_small_prestack_segy_fixture(path: impl AsRef<Path>) -> io::Result<()> {
    const SAMPLE_INTERVAL_US: u16 = 4_000;
    const SAMPLES_PER_TRACE: u16 = 10;

    let mut file = File::create(path)?;
    let mut textual_header = [b' '; 3200];
    let header_lines = [
        "C 1 Ophiolite synthetic prestack SEG-Y fixture                              ",
        "C 2 layout: 4 inlines x 3 xlines x 2 offsets x 10 samples                  ",
        "C 3 sample format code 5 (IEEE float32), big-endian                        ",
        "C 4 inline/xline/offset headers: 189/193/37                                ",
    ];
    for (index, line) in header_lines.iter().enumerate() {
        let start = index * 80;
        let bytes = line.as_bytes();
        let len = bytes.len().min(80);
        textual_header[start..start + len].copy_from_slice(&bytes[..len]);
    }
    file.write_all(&textual_header)?;

    let mut binary_header = [0_u8; 400];
    put_u16_be(&mut binary_header, 16, SAMPLE_INTERVAL_US);
    put_u16_be(&mut binary_header, 20, SAMPLES_PER_TRACE);
    put_u16_be(&mut binary_header, 24, SampleFormat::IeeeFloat32.code());
    put_u16_be(&mut binary_header, 300, 0x0100);
    put_u16_be(&mut binary_header, 302, 1);
    put_i16_be(&mut binary_header, 304, 0);
    file.write_all(&binary_header)?;

    let mut trace_sequence = 1_i32;
    for inline in 1_i32..=4 {
        for xline in 1_i32..=3 {
            for offset in 1_i32..=2 {
                let mut trace_header = [0_u8; 240];
                put_i32_be(&mut trace_header, 0, trace_sequence);
                put_i32_be(&mut trace_header, 4, trace_sequence);
                put_i32_be(&mut trace_header, 36, offset);
                put_i16_be(&mut trace_header, 70, 1);
                put_i16_be(&mut trace_header, 88, 1);
                put_i32_be(&mut trace_header, 180, 1_000 + (xline - 1) * 20);
                put_i32_be(&mut trace_header, 184, 2_000 + (inline - 1) * 10);
                put_i32_be(&mut trace_header, 188, inline);
                put_i32_be(&mut trace_header, 192, xline);
                put_i16_be(&mut trace_header, 114, SAMPLES_PER_TRACE as i16);
                put_i16_be(&mut trace_header, 116, SAMPLE_INTERVAL_US as i16);
                file.write_all(&trace_header)?;

                for sample in 0..SAMPLES_PER_TRACE {
                    let amplitude = offset as f32 * 100.0
                        + inline as f32
                        + xline as f32 / 100.0
                        + sample as f32 / 1000.0;
                    file.write_all(&amplitude.to_be_bytes())?;
                }

                trace_sequence += 1;
            }
        }
    }

    file.flush()?;
    Ok(())
}

fn put_u16_be(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn put_i16_be(buffer: &mut [u8], offset: usize, value: i16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn put_i32_be(buffer: &mut [u8], offset: usize, value: i32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
