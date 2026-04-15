<svelte:options runes={true} />

<script lang="ts">
  import { IPC_SCHEMA_VERSION } from "@traceboost/seis-contracts";
  import type {
    BuildSurveyTimeDepthTransformRequest,
    StratigraphicBoundaryReference,
    VelocityIntervalTrend
  } from "@traceboost/seis-contracts";
  import { getViewerModelContext } from "../viewer-model.svelte";

  type TrendKind = "constant" | "linear_depth" | "linear_time";

  type IntervalDraft = {
    id: string;
    name: string;
    topBoundaryKey: string;
    baseBoundaryKey: string;
    trendKind: TrendKind;
    constantVelocityDraft: string;
    topVelocityDraft: string;
    depthGradientDraft: string;
    timeGradientDraft: string;
  };

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const viewerModel = getViewerModelContext();

  let modelName = $state("Velocity Model");
  let outputIdDraft = $state("");
  let activateAfterBuild = $state(true);
  let intervalDrafts = $state<IntervalDraft[]>([
    {
      id: "interval-1",
      name: "Interval 1",
      topBoundaryKey: "survey_top",
      baseBoundaryKey: "survey_base",
      trendKind: "constant",
      constantVelocityDraft: "2200",
      topVelocityDraft: "1800",
      depthGradientDraft: "0.7",
      timeGradientDraft: "4.5"
    }
  ]);

  const importedHorizons = $derived(viewerModel.importedHorizons);
  const intervalValidationError = $derived(validateIntervals(intervalDrafts));
  const canAddInterval = $derived(importedHorizons.length > 0);
  const canBuild = $derived(
    !!viewerModel.activeStorePath &&
      !viewerModel.velocityModelWorkbenchBuilding &&
      !viewerModel.loading &&
      !intervalValidationError
  );

  function closeWorkbench(): void {
    open = false;
    viewerModel.closeVelocityModelWorkbench();
  }

  function slugify(value: string): string {
    const normalized = value
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
    return normalized || "velocity-model";
  }

  function parseNumber(value: string, fieldLabel: string): number {
    const parsed = Number(value);
    if (!Number.isFinite(parsed)) {
      throw new Error(`${fieldLabel} must be a finite number.`);
    }
    return parsed;
  }

  function defaultIntervalDraft(index: number, topBoundaryKey = "survey_top"): IntervalDraft {
    return {
      id: `interval-${index + 1}`,
      name: `Interval ${index + 1}`,
      topBoundaryKey,
      baseBoundaryKey: "survey_base",
      trendKind: "constant",
      constantVelocityDraft: "2200",
      topVelocityDraft: "1800",
      depthGradientDraft: "0.7",
      timeGradientDraft: "4.5"
    };
  }

  function boundaryFromKey(value: string): StratigraphicBoundaryReference {
    if (value === "survey_top" || value === "survey_base") {
      return value;
    }
    if (value.startsWith("horizon:")) {
      return {
        horizon_asset: {
          horizon_id: value.slice("horizon:".length)
        }
      };
    }
    throw new Error(`Unsupported boundary reference '${value}'.`);
  }

  function draftTrend(interval: IntervalDraft): VelocityIntervalTrend {
    if (interval.trendKind === "constant") {
      return {
        constant: {
          velocity_m_per_s: parseNumber(interval.constantVelocityDraft, "Constant velocity")
        }
      };
    }

    if (interval.trendKind === "linear_depth") {
      return {
        linear_with_depth: {
          velocity_at_top_m_per_s: parseNumber(interval.topVelocityDraft, "Top velocity"),
          gradient_m_per_s_per_m: parseNumber(interval.depthGradientDraft, "Depth gradient")
        }
      };
    }

    return {
      linear_with_time: {
        velocity_at_top_m_per_s: parseNumber(interval.topVelocityDraft, "Top velocity"),
        gradient_m_per_s_per_ms: parseNumber(interval.timeGradientDraft, "Time gradient")
      }
    };
  }

  function setIntervalCount(targetCount: number): void {
    if (targetCount < 1) {
      return;
    }
    const next = intervalDrafts.map((interval) => ({ ...interval }));
    while (next.length < targetCount) {
      const previous = next[next.length - 1];
      next.push(defaultIntervalDraft(next.length, previous.baseBoundaryKey));
    }
    while (next.length > targetCount) {
      next.pop();
    }
    for (let index = 0; index < next.length; index += 1) {
      next[index].id = `interval-${index + 1}`;
      next[index].name = `Interval ${index + 1}`;
      if (index === 0) {
        next[index].topBoundaryKey = "survey_top";
      } else {
        next[index].topBoundaryKey = next[index - 1].baseBoundaryKey;
      }
    }
    next[next.length - 1].baseBoundaryKey = "survey_base";
    for (let index = 1; index < next.length; index += 1) {
      next[index].topBoundaryKey = next[index - 1].baseBoundaryKey;
    }
    intervalDrafts = next;
  }

  function addInterval(): void {
    if (!importedHorizons.length) {
      return;
    }
    const next = intervalDrafts.map((interval) => ({ ...interval }));
    const firstHorizonBoundary = `horizon:${importedHorizons[0].id}`;
    if (next[next.length - 1].baseBoundaryKey === "survey_base") {
      next[next.length - 1].baseBoundaryKey = firstHorizonBoundary;
    }
    const previous = next[next.length - 1];
    next.push(defaultIntervalDraft(next.length, previous.baseBoundaryKey));
    intervalDrafts = next;
    setIntervalCount(next.length);
  }

  function removeLastInterval(): void {
    setIntervalCount(intervalDrafts.length - 1);
  }

  function updateBaseBoundary(index: number, value: string): void {
    const next = intervalDrafts.map((interval) => ({ ...interval }));
    next[index].baseBoundaryKey = value;
    for (let intervalIndex = index + 1; intervalIndex < next.length; intervalIndex += 1) {
      next[intervalIndex].topBoundaryKey = next[intervalIndex - 1].baseBoundaryKey;
    }
    intervalDrafts = next;
  }

  function validateIntervals(intervals: IntervalDraft[]): string | null {
    if (!intervals.length) {
      return "Add at least one interval.";
    }
    if (intervals[0].topBoundaryKey !== "survey_top") {
      return "The first interval must start at survey top.";
    }
    if (intervals[intervals.length - 1].baseBoundaryKey !== "survey_base") {
      return "The last interval must end at survey base.";
    }
    for (let index = 0; index < intervals.length; index += 1) {
      const interval = intervals[index];
      if (interval.topBoundaryKey === interval.baseBoundaryKey) {
        return `${interval.name} must have different top and base boundaries.`;
      }
      if (index > 0 && interval.topBoundaryKey !== intervals[index - 1].baseBoundaryKey) {
        return `${interval.name} must start where the previous interval ends.`;
      }
    }
    return null;
  }

  function buildRequest(): BuildSurveyTimeDepthTransformRequest {
    const cleanedName = modelName.trim() || "Velocity Model";
    const cleanedOutputId = outputIdDraft.trim() || `${slugify(cleanedName)}-survey-transform`;

    return {
      schema_version: IPC_SCHEMA_VERSION,
      store_path: viewerModel.activeStorePath,
      model: {
        id: `${slugify(cleanedName)}-recipe`,
        name: cleanedName,
        derived_from: [],
        coordinate_reference: null,
        grid_transform: null,
        vertical_domain: "time",
        travel_time_reference: "two_way",
        depth_reference: "true_vertical_depth",
        intervals: intervalDrafts.map((interval, index) => ({
          id: `survey-interval-${index + 1}`,
          name: interval.name,
          top_boundary: boundaryFromKey(interval.topBoundaryKey),
          base_boundary: boundaryFromKey(interval.baseBoundaryKey),
          trend: draftTrend(interval),
          control_profile_set_id: null,
          control_profile_velocity_kind: null,
          lateral_interpolation: "nearest",
          vertical_interpolation: "linear",
          control_blend_weight: null,
          notes: []
        })),
        notes: [
          "Built from the experimental TraceBoost velocity-model workbench.",
          `Current workbench scope is ${intervalDrafts.length} interval(s) with optional imported horizon boundaries.`
        ]
      },
      control_profile_sets: [],
      output_id: cleanedOutputId,
      output_name: cleanedName,
      preferred_velocity_kind: "interval",
      output_depth_unit: "m",
      notes: [
        "Preview/experimental authored velocity-model workflow.",
        "Stacked interval builds currently use nearest lateral interpolation and linear/step vertical interpolation only."
      ]
    };
  }

  async function handleBuild(): Promise<void> {
    try {
      await viewerModel.buildAuthoredVelocityModel(buildRequest(), activateAfterBuild);
      if (activateAfterBuild) {
        closeWorkbench();
      }
    } catch {
      // ViewerModel already owns the user-facing error state.
    }
  }
</script>

{#if open}
  <div class="workbench-backdrop" role="presentation" onclick={closeWorkbench}>
    <div
      class="workbench-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Velocity model definition"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <div class="workbench-header">
        <div>
          <h3>Velocity Model Definition</h3>
          <p>
            Preview / experimental workbench. Current scope supports a stacked interval recipe, optionally bounded by
            imported horizons, compiled into an activatable survey transform.
          </p>
        </div>
        <button class="close-btn" type="button" onclick={closeWorkbench}>Close</button>
      </div>

      <div class="workbench-layout">
        <section class="workbench-panel">
          <div class="field-grid">
            <label class="field">
              <span>Model Name</span>
              <input bind:value={modelName} type="text" placeholder="Velocity Model" />
            </label>

            <label class="field">
              <span>Output Id</span>
              <input bind:value={outputIdDraft} type="text" placeholder="Auto-generated if empty" />
            </label>

            <div class="field checkbox-field">
              <span>Activate After Build</span>
              <label class="checkbox-toggle">
                <input bind:checked={activateAfterBuild} type="checkbox" />
                <strong>{activateAfterBuild ? "Yes" : "No"}</strong>
              </label>
            </div>
          </div>

          <div class="interval-toolbar">
            <div>
              <strong>Intervals</strong>
              <p>Build a continuous stack from survey top to survey base. Adjacent intervals share boundaries automatically.</p>
            </div>
            <div class="interval-toolbar-actions">
              <button class="secondary" type="button" onclick={removeLastInterval} disabled={intervalDrafts.length <= 1}>
                Remove Last
              </button>
              <button type="button" onclick={addInterval} disabled={!canAddInterval}>Add Interval</button>
            </div>
          </div>

          <div class="interval-stack">
            {#each intervalDrafts as interval, index (interval.id)}
              <section class="interval-card">
                <div class="interval-card-header">
                  <h4>{interval.name}</h4>
                  <small>
                    {index === 0 ? "Starts at survey top" : `Starts at ${intervalDrafts[index - 1].baseBoundaryKey.replace("horizon:", "")}`}
                  </small>
                </div>

                <div class="field-grid interval-field-grid">
                  <label class="field">
                    <span>Top Boundary</span>
                    <input value={index === 0 ? "Survey top" : interval.topBoundaryKey.replace("horizon:", "")} disabled />
                  </label>

                  <label class="field">
                    <span>Base Boundary</span>
                    <select
                      value={interval.baseBoundaryKey}
                      onchange={(event) =>
                        updateBaseBoundary(index, (event.currentTarget as HTMLSelectElement).value)}
                    >
                      {#each importedHorizons as horizon (horizon.id)}
                        <option value={`horizon:${horizon.id}`}>{horizon.name}</option>
                      {/each}
                      <option value="survey_base">Survey base</option>
                    </select>
                  </label>

                  <label class="field">
                    <span>Trend Type</span>
                    <select bind:value={interval.trendKind}>
                      <option value="constant">Constant velocity</option>
                      <option value="linear_depth">Linear with depth</option>
                      <option value="linear_time">Linear with time</option>
                    </select>
                  </label>

                  {#if interval.trendKind === "constant"}
                    <label class="field">
                      <span>Constant Velocity (m/s)</span>
                      <input bind:value={interval.constantVelocityDraft} type="number" min="1" step="10" />
                    </label>
                  {:else if interval.trendKind === "linear_depth"}
                    <label class="field">
                      <span>Velocity At Top (m/s)</span>
                      <input bind:value={interval.topVelocityDraft} type="number" min="1" step="10" />
                    </label>
                    <label class="field">
                      <span>Gradient (m/s per m)</span>
                      <input bind:value={interval.depthGradientDraft} type="number" step="0.05" />
                    </label>
                  {:else}
                    <label class="field">
                      <span>Velocity At Top (m/s)</span>
                      <input bind:value={interval.topVelocityDraft} type="number" min="1" step="10" />
                    </label>
                    <label class="field">
                      <span>Gradient (m/s per ms)</span>
                      <input bind:value={interval.timeGradientDraft} type="number" step="0.05" />
                    </label>
                  {/if}
                </div>
              </section>
            {/each}
          </div>

          <div class="experimental-note">
            <strong>Current limits</strong>
            <p>
              The backend now supports stacked intervals, but only with nearest lateral interpolation and
              step/linear vertical interpolation. Control-profile blending and richer geospatial interpolation are
              still next steps.
            </p>
            <p>
              If a referenced horizon is missing over part of the survey, the affected traces stay uncovered in the
              built transform.
            </p>
          </div>
        </section>

        <aside class="workbench-panel workbench-sidebar">
          <div class="sidebar-block">
            <h4>Available Horizons</h4>
            <p class="sidebar-copy">
              Imported horizons from the active seismic store are selectable structural boundaries for stacked
              interval modeling.
            </p>
            {#if importedHorizons.length}
              <div class="horizon-list">
                {#each importedHorizons as horizon (horizon.id)}
                  <div class="horizon-entry">
                    <span>{horizon.name}</span>
                    <small>{horizon.id}</small>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="sidebar-empty">No imported horizons are available for the active volume.</p>
            {/if}
          </div>

          <div class="sidebar-block">
            <h4>Build Target</h4>
            <p class="sidebar-copy">
              Build output is a survey-aligned time-depth transform that shows up in the existing <strong>Velocity Models</strong> list and can be activated for TWT/depth switching.
            </p>
          </div>
        </aside>
      </div>

      {#if intervalValidationError}
        <p class="build-error">{intervalValidationError}</p>
      {/if}

      {#if viewerModel.velocityModelWorkbenchError}
        <p class="build-error">{viewerModel.velocityModelWorkbenchError}</p>
      {/if}

      <div class="workbench-actions">
        <button class="secondary" type="button" onclick={closeWorkbench}>Cancel</button>
        <button type="button" disabled={!canBuild} onclick={() => void handleBuild()}>
          {viewerModel.velocityModelWorkbenchBuilding ? "Building..." : activateAfterBuild ? "Build & Activate" : "Build"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .workbench-backdrop {
    position: fixed;
    inset: 0;
    z-index: 45;
    display: grid;
    place-items: center;
    background: rgb(38 55 71 / 0.2);
    backdrop-filter: blur(4px);
  }

  .workbench-dialog {
    width: min(1120px, calc(100vw - 40px));
    max-height: calc(100vh - 40px);
    overflow: auto;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    box-shadow: 0 24px 60px rgba(42, 64, 84, 0.22);
  }

  .workbench-header,
  .workbench-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
    border-bottom: 1px solid var(--app-border);
  }

  .workbench-actions {
    border-top: 1px solid var(--app-border);
    border-bottom: 0;
  }

  .workbench-header h3,
  .workbench-sidebar h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .workbench-header p,
  .sidebar-copy,
  .sidebar-empty,
  .experimental-note p {
    margin: 6px 0 0;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .workbench-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(280px, 0.9fr);
    gap: 16px;
    padding: 20px;
  }

  .workbench-panel {
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
    padding: 16px;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .checkbox-field {
    align-content: end;
  }

  .field span {
    color: var(--text-muted);
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  input,
  select,
  button {
    font: inherit;
  }

  input,
  select {
    min-height: 34px;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    background: #fff;
    color: var(--text-primary);
    padding: 0 10px;
  }

  .checkbox-toggle {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    background: #fff;
    color: var(--text-primary);
    padding: 0 10px;
  }

  input:disabled,
  select:disabled {
    color: var(--text-dim);
  }

  .experimental-note,
  .sidebar-block {
    margin-top: 18px;
    padding-top: 14px;
    border-top: 1px solid var(--app-border);
  }

  .interval-toolbar,
  .interval-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .interval-toolbar {
    margin-top: 18px;
    padding-top: 14px;
    border-top: 1px solid var(--app-border);
  }

  .interval-toolbar p,
  .interval-card-header small {
    margin: 6px 0 0;
    color: var(--text-muted);
  }

  .interval-toolbar-actions {
    display: flex;
    gap: 10px;
  }

  .interval-stack {
    display: grid;
    gap: 14px;
    margin-top: 16px;
  }

  .interval-card {
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: #fff;
    padding: 14px;
  }

  .interval-card h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .interval-field-grid {
    margin-top: 14px;
  }

  .horizon-list {
    display: grid;
    gap: 8px;
    margin-top: 12px;
  }

  .horizon-entry {
    display: grid;
    gap: 2px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: #fff;
    padding: 10px 12px;
  }

  .horizon-entry span {
    color: var(--text-primary);
  }

  .horizon-entry small {
    color: var(--text-muted);
  }

  .close-btn,
  .secondary,
  .workbench-actions button {
    min-height: 34px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--surface-subtle);
    color: var(--text-primary);
    padding: 0 12px;
  }

  .workbench-actions button:last-child {
    background: #e8f3fb;
    border-color: #b0d4ee;
    color: #274b61;
  }

  .build-error {
    margin: 0;
    padding: 0 20px 18px;
    color: #8f3c3c;
  }

  @media (max-width: 980px) {
    .workbench-layout {
      grid-template-columns: 1fr;
    }

    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
