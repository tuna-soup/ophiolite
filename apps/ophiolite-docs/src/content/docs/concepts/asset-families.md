---
title: Asset Families
description: The asset kinds Ophiolite treats as first-class.
draft: false
---

Ophiolite does not flatten everything into one generic table abstraction at the public API layer.

## Current families

### Log

Depth- or time-indexed sampled curves, usually imported from LAS and stored in log packages.

### Trajectory

Measured-depth survey rows and related wellbore geometry.

### TopSet

Named marker rows or intervals tied to a wellbore depth context.

### PressureObservation

Pressure measurements and related attributes.

### DrillingObservation

Structured drilling rows or events captured in a typed form.

This typed model is what enables:

- family-specific editing
- family-specific compute
- consistent project browsing
- clearer future ingest paths
