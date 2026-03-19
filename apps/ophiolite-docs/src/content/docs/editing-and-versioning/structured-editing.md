---
title: Structured Editing
description: Typed edit sessions for non-log asset families.
draft: false
---

Trajectory, tops, pressure, and drilling assets use typed project-scoped edit sessions.

## Scope

- row add
- row update
- row delete
- family-specific validation
- explicit save or discard

## Important bound

Ophiolite allows edits within a family, not across families. A top set remains a top set, and a trajectory remains a trajectory.

That gives the application enough editing power without collapsing the public model into generic table mutation.
