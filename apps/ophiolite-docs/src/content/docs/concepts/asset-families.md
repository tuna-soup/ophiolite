---
title: Asset Families
description: The asset kinds Ophiolite treats as first-class.
draft: false
---

Ophiolite does not flatten everything into one generic table abstraction at the public API layer.

## Current families

### Log

Depth- or time-indexed sampled curves, often imported from LAS but not conceptually limited to LAS as a source artifact.

### Trajectory

Measured-depth survey rows and related wellbore geometry.

### TopSet

Named marker rows or intervals tied to a wellbore depth context.

### PressureObservation

Pressure measurements and related attributes.

### DrillingObservation

Structured drilling rows or events captured in a typed form.

### Seismic Trace Data

Post-stack volumes and prestack gathers with explicit stacking, layout, and gather-axis semantics.

## Adjacent canonical outputs

Not every important Ophiolite type is a persisted family. The core also owns reusable DTO meaning for:

- section and gather views
- survey-map payloads
- well-on-section overlays
- time-depth and velocity-model boundaries

This typed model is what enables:

- family-specific editing
- family-specific compute
- consistent project browsing
- clearer future ingest and app-boundary paths
