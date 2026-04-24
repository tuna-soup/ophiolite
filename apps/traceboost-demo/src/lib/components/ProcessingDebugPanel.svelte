<script lang="ts">
  import type {
    InspectablePlanDecision,
    InspectableProcessingPlan,
    ProcessingJobArtifact,
    ProcessingJobRuntimeState,
    ProcessingRuntimePolicyDivergence,
    ProcessingJobStatus,
    ProcessingRuntimeEvent,
    ProcessingRuntimeEventDetails,
    SectionAssemblyDebugRecord
  } from "@traceboost/seis-contracts";

  type DebugTab = "explain" | "why" | "runtime" | "lineage" | "sections";

  let {
    activeJob,
    debugPlan = null,
    runtimeState = null,
    runtimeEvents = [],
    onCancelJob,
    onOpenArtifact
  }: {
    activeJob: ProcessingJobStatus;
    debugPlan?: InspectableProcessingPlan | null;
    runtimeState?: ProcessingJobRuntimeState | null;
    runtimeEvents?: ProcessingRuntimeEvent[];
    onCancelJob: () => void | Promise<void>;
    onOpenArtifact: (storePath: string) => void | Promise<void>;
  } = $props();

  let activeTab = $state<DebugTab>("explain");

  const decisionsById = $derived.by(() => {
    const map: Record<string, InspectablePlanDecision> = {};
    for (const decision of debugPlan?.decisions ?? []) {
      map[decision.decision_id] = decision;
    }
    return map;
  });

  const planStages = $derived.by(() => debugPlan?.execution_plan.stages ?? []);
  const planArtifacts = $derived.by(() => debugPlan?.artifacts ?? []);
  const runtimeStageSnapshots = $derived.by(() => runtimeState?.stage_snapshots ?? []);
  const sectionEvents = $derived.by(() =>
    runtimeEvents.filter(
      (event) =>
        event.details.kind === "section_read" &&
        (event.event_kind === "section_window_read" || event.event_kind === "section_assembled_read")
    )
  );

  function formatBytes(value: number | null | undefined): string {
    if (value === null || value === undefined) {
      return "-";
    }
    if (value < 1024 * 1024) {
      return `${Math.round(value / 1024)} KiB`;
    }
    return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
  }

  function formatDecision(decision: InspectablePlanDecision | null | undefined): string {
    return decision?.human_summary ?? "No structured decision recorded.";
  }

  function stageDecision(decisionId: string | null | undefined): InspectablePlanDecision | null {
    return decisionId ? decisionsById[decisionId] ?? null : null;
  }

  function artifactDecision(decisionId: string | null | undefined): InspectablePlanDecision | null {
    return decisionId ? decisionsById[decisionId] ?? null : null;
  }

  function runtimeDetailSummary(details: ProcessingRuntimeEventDetails): string {
    switch (details.kind) {
      case "queue_state":
        return [details.queue_class ?? "queue", details.wait_reason ?? null, details.admitted ? "admitted" : null]
          .filter(Boolean)
          .join(" | ");
      case "progress":
        return `${details.completed}/${details.total}`;
      case "retry_scheduled":
        return `attempt ${details.attempt}`;
      case "artifact_emitted":
        return details.artifact_store_path;
      case "reuse_lookup":
        return details.reused
          ? details.artifact_store_path ?? "reused"
          : details.miss_reason ?? "reuse miss";
      case "section_read":
        return sectionSummary(details.record);
      default:
        return "";
    }
  }

  function sectionSummary(record: SectionAssemblyDebugRecord): string {
    return `${record.artifact_kind} ${record.axis} ${record.section_index} lod ${record.lod}`;
  }

  function artifactKindLabel(artifact: ProcessingJobArtifact): string {
    return artifact.kind === "final_output" ? "Final output" : "Checkpoint";
  }

  function runtimePolicyValueList(
    divergences: ProcessingRuntimePolicyDivergence[] | null | undefined
  ): string[] {
    return (divergences ?? []).map(
      (divergence) =>
        `${divergence.field}: planned ${divergence.planned_value}, actual ${divergence.actual_value}`
    );
  }
</script>

<div class="debug-panel">
  <div class="debug-header">
    <div>
      <strong>Debug Workflow</strong>
      <p>
        {activeJob.state}
        {#if activeJob.current_stage_label}
          | {activeJob.current_stage_label}
        {/if}
      </p>
    </div>
    <div class="debug-actions">
      <button
        class="chip"
        onclick={onCancelJob}
        disabled={activeJob.state !== "queued" && activeJob.state !== "running"}
      >
        Cancel Job
      </button>
      {#each activeJob.artifacts as artifact (`${artifact.kind}:${artifact.store_path}`)}
        <button class="chip" onclick={() => onOpenArtifact(artifact.store_path)}>
          Open {artifactKindLabel(artifact)}
        </button>
      {/each}
    </div>
  </div>

  <div class="debug-tabs">
    {#each [
      ["explain", "Explain"],
      ["why", "Why"],
      ["runtime", "Runtime"],
      ["lineage", "Lineage"],
      ["sections", "Sections"]
    ] as [tab, label] (tab)}
      <button
        class:active={activeTab === tab}
        onclick={() => {
          activeTab = tab as DebugTab;
        }}
      >
        {label}
      </button>
    {/each}
  </div>

  {#if activeTab === "explain"}
    <div class="debug-content">
      {#if debugPlan}
        <div class="debug-card">
          <strong>Plan</strong>
          <p>{planStages.length} stages | {debugPlan.execution_plan.summary.compute_stage_count} compute</p>
          <div class="pill-row">
            <span>priority {debugPlan.execution_plan.scheduler_hints.priority_class}</span>
            <span>expected partitions {debugPlan.execution_plan.scheduler_hints.expected_partition_count ?? "-"}</span>
            <span>operator digest {debugPlan.operator_set_identity.effective_operator_digest.slice(0, 12)}</span>
          </div>
        </div>

        <div class="debug-card">
          <strong>Stages</strong>
          {#each planStages as stage (stage.stage_id)}
            <div class="entry">
              <div class="entry-header">
                <span>{stage.stage_label}</span>
                <span>{stage.stage_kind}</span>
              </div>
              <p>
                {stage.partition.family} | queue {stage.resource_envelope.preferred_queue_class} | target
                {stage.expected_partition_count ?? "-"}
              </p>
            </div>
          {/each}
        </div>

        <div class="debug-card">
          <strong>Planner Passes</strong>
          {#each debugPlan.planner_diagnostics.pass_snapshots ?? [] as snapshot (snapshot.pass_name)}
            <div class="entry">
              <div class="entry-header">
                <span>{snapshot.pass_name}</span>
                <span>{snapshot.pass_id}</span>
              </div>
              {#if snapshot.snapshot_text}
                <p>{snapshot.snapshot_text}</p>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <div class="debug-card empty">No explain plan loaded.</div>
      {/if}
    </div>
  {:else if activeTab === "why"}
    <div class="debug-content">
      {#if debugPlan}
        {#each planStages as stage (stage.stage_id)}
          <div class="debug-card">
            <strong>{stage.stage_label}</strong>
            <p>{formatDecision(stageDecision(stage.planning_decision_id))}</p>
            {#if stageDecision(stage.planning_decision_id)?.stage_planning?.factors?.length}
              <div class="pill-row">
                {#each stageDecision(stage.planning_decision_id)?.stage_planning?.factors ?? [] as factor (`${stage.stage_id}:${factor.code}`)}
                  <span>{factor.code}: {factor.value ?? factor.summary}</span>
                {/each}
              </div>
            {/if}
            {#if stage.reuse_decision_id}
              {@const reuseDecision = stageDecision(stage.reuse_decision_id)}
              <p>{formatDecision(reuseDecision)}</p>
              {#if reuseDecision?.reuse_decision?.selected_candidate_reuse_key}
                <p>reuse key {reuseDecision.reuse_decision.selected_candidate_reuse_key}</p>
              {/if}
              {#if reuseDecision?.reuse_decision?.selected_candidate_artifact_key}
                <p>artifact key {reuseDecision.reuse_decision.selected_candidate_artifact_key}</p>
              {/if}
            {/if}
          </div>
        {/each}
      {:else}
        <div class="debug-card empty">No structured why-plan data loaded.</div>
      {/if}
    </div>
  {:else if activeTab === "runtime"}
    <div class="debug-content">
      <div class="debug-card">
        <strong>Runtime State</strong>
        {#if runtimeState}
          <p>
            {runtimeState.state}
            {#if runtimeState.snapshot}
              | {runtimeState.snapshot.queue_class} | {runtimeState.snapshot.wait_reason}
            {/if}
          </p>
          {#if runtimeState.snapshot}
            <div class="pill-row">
              <span>reserved {formatBytes(runtimeState.snapshot.reserved_memory_bytes)}</span>
              <span>budget {formatBytes(runtimeState.snapshot.memory_budget_bytes)}</span>
              <span>max active {runtimeState.snapshot.effective_max_active_partitions}</span>
            </div>
            {@const jobPolicyDivergences = runtimePolicyValueList(
              runtimeState.snapshot.policy_divergences
            )}
            {#if jobPolicyDivergences.length}
              <div class="pill-row">
                {#each jobPolicyDivergences as divergence (divergence)}
                  <span class="warning-pill">{divergence}</span>
                {/each}
              </div>
            {/if}
          {/if}
        {:else}
          <p>No runtime state loaded.</p>
        {/if}
      </div>

      <div class="debug-card">
        <strong>Stages</strong>
        {#if runtimeStageSnapshots.length}
          {#each runtimeStageSnapshots as stage (stage.stage_id)}
            <div class="entry">
              <div class="entry-header">
                <span>{stage.stage_label}</span>
                <span>{stage.state}</span>
              </div>
              <p>
                {stage.wait_reason ?? "ready"} | reserved {formatBytes(stage.reserved_memory_bytes)} |
                progress {stage.completed_partitions ?? 0}/{stage.total_partitions ?? "-"}
              </p>
              {#if runtimePolicyValueList(stage.policy_divergences).length}
                <div class="pill-row">
                  {#each runtimePolicyValueList(stage.policy_divergences) as divergence (`${stage.stage_id}:${divergence}`)}
                    <span class="warning-pill">{divergence}</span>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        {:else}
          <p>No stage runtime snapshots yet.</p>
        {/if}
      </div>

      <div class="debug-card">
        <strong>Recent Events</strong>
        {#if runtimeEvents.length}
          {#each [...runtimeEvents].reverse().slice(0, 12) as event (`${event.seq}`)}
            <div class="entry">
              <div class="entry-header">
                <span>{event.event_kind}</span>
                <span>#{event.seq}</span>
              </div>
              <p>{event.stage_label ?? event.job_id}</p>
              {#if runtimeDetailSummary(event.details)}
                <p>{runtimeDetailSummary(event.details)}</p>
              {/if}
            </div>
          {/each}
        {:else}
          <p>No runtime events captured yet.</p>
        {/if}
      </div>
    </div>
  {:else if activeTab === "lineage"}
    <div class="debug-content">
      {#if debugPlan}
        {#each planArtifacts as artifact (artifact.artifact_id)}
          <div class="debug-card">
            <div class="entry-header">
              <strong>{artifact.artifact_id}</strong>
              <span>{artifact.role}</span>
            </div>
            <p>{artifact.materialization_class ?? "unknown"} | {artifact.boundary_reason ?? "unspecified"}</p>
            {#if artifact.artifact_key}
              <p>{artifact.artifact_key.cache_key}</p>
              <div class="pill-row">
                <span>lineage {artifact.artifact_key.lineage_digest.slice(0, 12)}</span>
                <span>{artifact.artifact_key.materialization_class}</span>
                {#if artifact.chunk_grid_spec?.kind === "regular"}
                  <span>chunk {artifact.chunk_grid_spec.chunk_shape.join("x")}</span>
                {/if}
              </div>
            {/if}
            <p>{formatDecision(artifactDecision(artifact.artifact_derivation_decision_id))}</p>
          </div>
        {/each}
      {:else}
        <div class="debug-card empty">No artifact lineage loaded.</div>
      {/if}
    </div>
  {:else}
    <div class="debug-content">
      <div class="debug-card">
        <strong>Section Assembly</strong>
        {#if sectionEvents.length}
          {#each [...sectionEvents].reverse() as event (`section:${event.seq}`)}
            <div class="entry">
              <div class="entry-header">
                <span>{event.event_kind}</span>
                <span>#{event.seq}</span>
              </div>
              {#if event.details.kind === "section_read"}
                <p>{sectionSummary(event.details.record)}</p>
                <p>tiles {event.details.record.source_tiles?.length ?? 0}</p>
              {/if}
            </div>
          {/each}
        {:else}
          <p>No section assembly events captured in this job yet.</p>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .debug-panel {
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-3);
    padding-top: var(--ui-space-2);
    border-top: 1px solid var(--app-border);
  }

  .debug-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-3);
    align-items: flex-start;
  }

  .debug-header p {
    margin: 4px 0 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .debug-actions {
    display: flex;
    gap: var(--ui-space-2);
    flex-wrap: wrap;
  }

  .debug-tabs {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .debug-tabs button {
    border: 1px solid var(--app-border);
    background: color-mix(in srgb, var(--panel-bg) 72%, transparent);
    color: var(--text-muted);
    border-radius: 999px;
    padding: 6px 10px;
    font-size: 11px;
    cursor: pointer;
  }

  .debug-tabs button.active {
    color: var(--text-primary);
    background: color-mix(in srgb, var(--accent-soft) 28%, var(--panel-bg));
    border-color: color-mix(in srgb, var(--accent) 42%, var(--app-border));
  }

  .debug-content {
    display: grid;
    gap: var(--ui-space-3);
  }

  .debug-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-radius: var(--ui-radius-md);
    border: 1px solid var(--app-border);
    background: color-mix(in srgb, var(--panel-bg) 84%, transparent);
  }

  .debug-card.empty {
    color: var(--text-muted);
  }

  .entry {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 6px;
    border-top: 1px solid color-mix(in srgb, var(--app-border) 70%, transparent);
  }

  .entry-header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 11px;
    color: var(--text-primary);
  }

  .entry p {
    margin: 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .pill-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pill-row span,
  .chip {
    border: 1px solid var(--app-border);
    border-radius: 999px;
    padding: 5px 9px;
    font-size: 10px;
    color: var(--text-muted);
    background: color-mix(in srgb, var(--surface-subtle) 88%, transparent);
  }

  .chip {
    cursor: pointer;
    color: var(--text-primary);
  }

  .warning-pill {
    color: color-mix(in srgb, var(--accent) 64%, var(--text-primary));
    border-color: color-mix(in srgb, var(--accent) 42%, var(--app-border));
    background: color-mix(in srgb, var(--accent-soft) 36%, var(--panel-bg));
  }
</style>
