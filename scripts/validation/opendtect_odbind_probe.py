#!/usr/bin/env python3

import argparse
import json
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Probe OpendTect odbind access for a survey and optional 3D volume."
    )
    parser.add_argument("--basedir", required=True, help="OpendTect data root")
    parser.add_argument("--survey", required=True, help="Survey directory name")
    parser.add_argument(
        "--odbind-root",
        help="Path to the OpendTect bin/python directory that contains the odbind package",
    )
    parser.add_argument(
        "--volume",
        help="Optional 3D seismic volume name to open through odbind.seismic3d",
    )
    parser.add_argument(
        "--indent",
        type=int,
        default=2,
        help="JSON indentation to use for output",
    )
    return parser.parse_args()


def add_odbind_root(root: str | None) -> None:
    if not root:
        return
    root_path = str(Path(root).expanduser())
    if root_path not in sys.path:
        sys.path.insert(0, root_path)


def exc_info(exc: Exception) -> dict[str, str]:
    return {"type": type(exc).__name__, "message": str(exc)}


def main() -> int:
    args = parse_args()
    add_odbind_root(args.odbind_root)

    result: dict[str, object] = {
        "basedir": str(Path(args.basedir).expanduser()),
        "survey": args.survey,
        "status": "ok",
    }

    try:
        from odbind.survey import Survey
    except Exception as exc:  # pragma: no cover - environment-dependent
        result["status"] = "import_error"
        result["error"] = exc_info(exc)
        print(json.dumps(result, indent=args.indent, sort_keys=True))
        return 1

    try:
        result["surveyNames"] = Survey.names(args.basedir)
        survey = Survey(args.survey, args.basedir)
        result["surveyInfo"] = survey.info()
        result["has2d"] = survey.has2d
        result["has3d"] = survey.has3d
        result["surveyType"] = survey.survey_type
    except Exception as exc:
        result["status"] = "survey_error"
        result["error"] = exc_info(exc)
        print(json.dumps(result, indent=args.indent, sort_keys=True))
        return 1

    groups = ["Seismic Data", "Geometry", "Well"]
    object_names: dict[str, object] = {}
    for group in groups:
        try:
            object_names[group] = survey.get_object_names(group)
        except Exception as exc:
            object_names[group] = {"error": exc_info(exc)}
    result["objectNames"] = object_names

    if args.volume:
        probe: dict[str, object] = {"name": args.volume}
        try:
            probe["hasObject"] = survey.has_object(args.volume, "Seismic Data")
        except Exception as exc:
            probe["hasObjectError"] = exc_info(exc)

        try:
            probe["objectInfo"] = survey.get_object_info(args.volume, "Seismic Data")
        except Exception as exc:
            probe["objectInfoError"] = exc_info(exc)

        try:
            from odbind.seismic3d import Seismic3D
        except Exception as exc:
            probe["openStatus"] = "import_error"
            probe["importError"] = exc_info(exc)
        else:
            try:
                volume = Seismic3D(survey, args.volume)
                probe["openStatus"] = "ok"
                probe["shape"] = list(volume.shape)
                probe["ranges"] = {
                    "inl": list(volume.ranges.inlrg),
                    "crl": list(volume.ranges.crlrg),
                    "z": list(volume.ranges.zrg),
                }
                probe["components"] = volume.comp_names
            except Exception as exc:
                probe["openStatus"] = "error"
                probe["openError"] = exc_info(exc)

        result["volumeProbe"] = probe

    print(json.dumps(result, indent=args.indent, sort_keys=True))
    return 0 if result["status"] == "ok" else 1


if __name__ == "__main__":
    raise SystemExit(main())
