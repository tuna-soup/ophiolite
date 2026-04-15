---
title: ADRs
description: Architecture decision records for Ophiolite.
draft: false
---

The ADRs remain the durable record of accepted architectural decisions.

## Current ADR set

- [ADR-0001 Canonical LAS Model](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0001-canonical-las-model.md)
- [ADR-0002 Staged Arrow/Parquet Adoption](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0002-staged-arrow-parquet-adoption.md)
- [ADR-0003 Package Format: metadata.json + curves.parquet](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0003-package-format-metadata-json-plus-curves-parquet.md)
- [ADR-0004 LASIO Parity and Scope](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0004-lasio-parity-and-scope.md)
- [ADR-0005 Workspace Split and Table Boundary](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0005-staged-workspace-split-and-table-boundary.md)
- [ADR-0006 Package Session and DTO Boundary](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0006-package-session-and-dto-boundary.md)
- [ADR-0007 Canonical Schema Target](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0007-canonical-schema-target.md)
- [ADR-0008 Project Catalog and Single-Asset Packages](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0008-project-catalog-and-single-asset-packages.md)
- [ADR-0009 Future Ecosystem Boundaries](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0009-future-ecosystem-boundaries.md)
- [ADR-0010 Typed Compute and Derived Assets](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0010-typed-compute-and-derived-assets.md)
- [ADR-0011 Structured Asset Edit Sessions](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0011-structured-asset-edit-sessions.md)
- [ADR-0012 Revisioned Overwrite-Oriented Saves](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0012-revisioned-last-save-wins.md)
- [ADR-0013 Shared Subsurface Core and Seismic Expansion](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0013-shared-subsurface-core-and-seismic-expansion.md)
- [ADR-0014 Seismic CRS Native, Effective, and Display Boundary](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0014-seismic-crs-native-effective-display-boundary.md)
- [ADR-0015 Authored Models, Compiled Runtime Assets, Analysis APIs, and Display DTOs](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0015-authored-models-compiled-runtime-assets-and-display-dtos.md)
- [ADR-0016 Canonical Wellbore Geometry and Resolved Trajectory Boundary](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0016-canonical-wellbore-geometry-and-resolved-trajectory-boundary.md)
- [ADR-0017 Well Time-Depth Source Assets, Authored Models, and Compiled Runtime Output](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0017-well-time-depth-source-assets-authored-models-and-compiled-runtime-output.md)
- [ADR-0018 Project-Aware Well-On-Section Overlay DTOs and Backend Projection Rules](https://github.com/scrooijmans/ophiolite/blob/master/docs/architecture/ADR-0018-project-aware-well-on-section-overlay-dtos-and-backend-projection-rules.md)

## Reading order

If you are new to Ophiolite, start with:

1. ADR-0008
2. ADR-0009
3. ADR-0013
4. ADR-0014
5. ADR-0015
6. ADR-0017
