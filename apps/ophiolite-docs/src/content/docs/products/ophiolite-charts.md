---
title: Ophiolite Charts
description: The embeddable chart SDK inside the Ophiolite platform.
draft: false
---

Ophiolite Charts is the embeddable chart SDK inside the Ophiolite platform.

## What it owns

- seismic, gather, survey-map, and well-correlation chart rendering
- chart-native interaction behavior
- wrapper APIs for embedders
- chart-relative anchors, intrinsic sizing, and viewport behavior

## What it does not own

- canonical subsurface meaning
- storage or backend transport details
- product workflow state

## Design rule

Ophiolite Charts should consume canonical contracts or normalized chart payloads and turn them into reusable interactive views.

It should not become a second domain model or a hidden app shell.
