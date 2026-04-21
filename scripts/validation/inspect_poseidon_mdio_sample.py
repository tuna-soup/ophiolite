#!/usr/bin/env python3
import json
import math
import os
import sys
from pathlib import Path

import numpy as np

try:
    from numcodecs import Blosc
except Exception as exc:  # pragma: no cover - runtime dependency probe
    print(
        "Missing dependency: numcodecs. Install it, for example:\n"
        "  python3 -m pip install --target /tmp/poseidon-inspect numcodecs\n"
        "Then run with PYTHONPATH=/tmp/poseidon-inspect",
        file=sys.stderr,
    )
    raise SystemExit(2) from exc


def load_metadata(root: Path) -> dict:
    return json.loads((root / ".zmetadata").read_text())["metadata"]


def dtype_from_zarr(dtype_spec):
    if isinstance(dtype_spec, str):
        return np.dtype(dtype_spec)
    if isinstance(dtype_spec, list):
        fields = []
        for entry in dtype_spec:
            if len(entry) == 2:
                name, field_dtype = entry
                fields.append((name, dtype_from_zarr(field_dtype)))
            elif len(entry) == 3:
                name, field_dtype, shape = entry
                fields.append((name, dtype_from_zarr(field_dtype), tuple(shape)))
            else:
                raise TypeError(f"Unsupported structured dtype entry: {entry!r}")
        return np.dtype(fields)
    return np.dtype(dtype_spec)


def array_meta(metadata: dict, name: str) -> dict:
    return metadata[f"{name}/.zarray"]


def chunk_shape_for_index(shape, chunks, chunk_index):
    resolved = []
    for dim_size, chunk_size, index in zip(shape, chunks, chunk_index):
        start = index * chunk_size
        resolved.append(min(chunk_size, dim_size - start))
    return tuple(resolved)


def decode_chunk(root: Path, metadata: dict, name: str, chunk_index: tuple[int, ...]) -> np.ndarray:
    meta = array_meta(metadata, name)
    dtype = dtype_from_zarr(meta["dtype"])
    shape = tuple(meta["shape"])
    chunks = tuple(meta["chunks"])
    compressor_config = dict(meta["compressor"])
    compressor_config.pop("id", None)
    compressor = Blosc.from_config(compressor_config)
    chunk_path = root / name / "/".join(str(part) for part in chunk_index)
    raw = chunk_path.read_bytes()
    decoded = compressor.decode(raw)
    resolved_shape = chunk_shape_for_index(shape, chunks, chunk_index)
    return np.frombuffer(decoded, dtype=dtype).reshape(resolved_shape, order=meta.get("order", "C"))


def summarize_numeric(name: str, values: np.ndarray):
    flat = np.asarray(values).astype(np.float64).ravel()
    print(f"{name}: shape={values.shape} min={flat.min():.3f} max={flat.max():.3f} mean={flat.mean():.3f}")


def summarize_headers(headers: np.ndarray):
    print(
        "headers: shape={} inline=[{}, {}] crossline=[{}, {}] cdp-x=[{}, {}] cdp-y=[{}, {}]".format(
            headers.shape,
            int(headers["inline"].min()),
            int(headers["inline"].max()),
            int(headers["crossline"].min()),
            int(headers["crossline"].max()),
            int(headers["cdp-x"].min()),
            int(headers["cdp-x"].max()),
            int(headers["cdp-y"].min()),
            int(headers["cdp-y"].max()),
        )
    )


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: inspect_poseidon_mdio_sample.py <sample-root>", file=sys.stderr)
        return 2

    root = Path(sys.argv[1]).expanduser().resolve()
    metadata = load_metadata(root)

    print(f"sample_root={root}")
    for name in ("inline", "crossline", "time", "trace_mask", "headers", "seismic"):
        meta = array_meta(metadata, name)
        print(
            f"{name}: shape={meta['shape']} chunks={meta['chunks']} dtype={meta['dtype']} compressor={meta['compressor']}"
        )
    print()

    inline = decode_chunk(root, metadata, "inline", (0,))
    crossline = decode_chunk(root, metadata, "crossline", (0,))
    time_axis = decode_chunk(root, metadata, "time", (0,))
    print(
        "axis_ranges: inline=[{}, {}] crossline=[{}, {}] time=[{}, {}]".format(
            int(inline.min()),
            int(inline.max()),
            int(crossline.min()),
            int(crossline.max()),
            int(time_axis.min()),
            int(time_axis.max()),
        )
    )

    mask = decode_chunk(root, metadata, "trace_mask", (0, 0))
    headers = decode_chunk(root, metadata, "headers", (0, 0))
    seismic = decode_chunk(root, metadata, "seismic", (0, 1, 1))

    summarize_numeric("trace_mask[0,0]", mask.astype(np.uint8))
    summarize_headers(headers)
    summarize_numeric("seismic[0,1,1]", seismic)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
