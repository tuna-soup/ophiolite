from __future__ import annotations

import json
import struct
import subprocess
import sys
from pathlib import Path

import numpy as np


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BRUGES_ROOT = REPO_ROOT.parent / "bruges"


def decode_f32le(payload: list[int]) -> np.ndarray:
    return np.array(
        [value[0] for value in struct.iter_unpack("<f", bytes(payload))],
        dtype=np.float64,
    )


def run_cli(request: dict[str, object]) -> dict[str, object]:
    command = [
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(REPO_ROOT / "Cargo.toml"),
        "-p",
        "ophiolite-cli",
        "--",
        "run-rock-physics-attribute",
        "-",
    ]
    completed = subprocess.run(
        command,
        cwd=REPO_ROOT,
        input=json.dumps(request),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=True,
    )
    return json.loads(completed.stdout)


def main() -> int:
    bruges_root = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_BRUGES_ROOT
    if not bruges_root.exists():
        raise SystemExit(f"Bruges checkout not found at {bruges_root}")
    sys.path.insert(0, str(bruges_root))

    from bruges.rockphysics.elastic import elastic_impedance

    vp = np.array([2600.0, 2800.0, 3000.0, 3100.0], dtype=np.float64)
    vs = np.array([1200.0, 1400.0, 1600.0, 1700.0], dtype=np.float64)
    rho = np.array([2.15, 2.22, 2.28, 2.35], dtype=np.float64)

    ei_payload = run_cli(
        {
            "schema_version": 2,
            "method": "elastic_impedance",
            "sample_shape": [4],
            "vp_m_per_s": vp.tolist(),
            "vs_m_per_s": vs.tolist(),
            "density_g_cc": rho.tolist(),
            "incident_angle_deg": 30.0,
        }
    )
    ei_actual = decode_f32le(ei_payload["values_f32le"])
    ei_expected = np.asarray(
        elastic_impedance(vp, vs, rho, theta1=30.0, normalize=True),
        dtype=np.float64,
    )

    eei_payload = run_cli(
        {
            "schema_version": 2,
            "method": "extended_elastic_impedance",
            "sample_shape": [4],
            "vp_m_per_s": vp.tolist(),
            "vs_m_per_s": vs.tolist(),
            "density_g_cc": rho.tolist(),
            "chi_angle_deg": 20.0,
        }
    )
    eei_actual = decode_f32le(eei_payload["values_f32le"])
    ai = vp * rho
    ai0 = float(np.mean(vp) * np.mean(rho))
    ei_ninety = np.asarray(
        elastic_impedance(vp, vs, rho, theta1=90.0, normalize=True, use_sin=True),
        dtype=np.float64,
    )
    chi_rad = np.deg2rad(20.0)
    eei_expected = (ai0 ** (1.0 - np.cos(chi_rad))) * (ai ** np.cos(chi_rad)) * (
        (ei_ninety / ai) ** np.sin(chi_rad)
    )

    result = {
        "bruges_root": str(bruges_root),
        "ei_max_abs_diff": float(np.max(np.abs(ei_actual - ei_expected))),
        "eei_max_abs_diff": float(np.max(np.abs(eei_actual - eei_expected))),
        "ok": bool(
            np.max(np.abs(ei_actual - ei_expected)) < 1.0e-3
            and np.max(np.abs(eei_actual - eei_expected)) < 1.0e-3
        ),
    }
    print(json.dumps(result, indent=2))
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
