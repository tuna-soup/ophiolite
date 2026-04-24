<svelte:options runes={true} />

<script lang="ts">
  import PipelineControlBar from "./PipelineControlBar.svelte";
  import NeighborhoodOperatorEditor from "./NeighborhoodOperatorEditor.svelte";
  import NeighborhoodSequenceList from "./NeighborhoodSequenceList.svelte";
  import ProcessingActivityStrip from "./ProcessingActivityStrip.svelte";
  import PipelineOperatorEditor from "./PipelineOperatorEditor.svelte";
  import PipelineSequenceList from "./PipelineSequenceList.svelte";
  import PipelineSessionList from "./PipelineSessionList.svelte";
  import { SpectrumInspector } from "@ophiolite/charts/extras";
  import type { SpectrumResponseLike } from "@ophiolite/charts";
  import { adaptAmplitudeSpectrum } from "../spectrum-adapter";
  import { getProcessingModelContext } from "../processing-model.svelte";
  import { getViewerModelContext } from "../viewer-model.svelte";

  const viewerModel = getViewerModelContext();
  const processingModel = getProcessingModelContext();
  const rawSpectrum = $derived.by<SpectrumResponseLike | null>(() => adaptAmplitudeSpectrum(processingModel.rawSpectrum));
  const processedSpectrum = $derived.by<SpectrumResponseLike | null>(() =>
    adaptAmplitudeSpectrum(processingModel.processedSpectrum)
  );
</script>

<svelte:window onkeydown={(event) => void processingModel.handleKeydown(event)} />

<div class="workspace-shell">
  <div class="workspace-header">
    <div>
      <span class="eyebrow">Processing Workspace</span>
      <h2>{processingModel.pipelineTitle}</h2>
      <p>
        {viewerModel.dataset
          ? `Working on ${viewerModel.activeDatasetDisplayName} at ${viewerModel.axis}:${viewerModel.index}`
          : "Open a runtime store to preview processing on the current section."}
      </p>
      <p class="workflow-origin">{processingModel.operatorCatalogSourceLabel}</p>
    </div>
    <div class="family-switch" role="tablist" aria-label="Processing family">
      <button
        class:active={processingModel.pipelineFamily === "trace_local"}
        class="family-chip"
        onclick={() => processingModel.setPipelineFamily("trace_local")}
      >
        Trace-Local
      </button>
      <button
        class:active={processingModel.pipelineFamily === "post_stack_neighborhood"}
        class="family-chip"
        onclick={() => processingModel.setPipelineFamily("post_stack_neighborhood")}
      >
        Neighborhood
      </button>
    </div>
  </div>

  <div class="workspace-grid">
    <PipelineSessionList
      pipelines={processingModel.sessionPipelineItems}
      activePipelineId={processingModel.activeSessionPipelineId}
      onSelect={processingModel.activateSessionPipeline}
      onCreate={processingModel.createSessionPipeline}
      onDuplicate={processingModel.duplicateActiveSessionPipeline}
      onCopy={processingModel.copyActiveSessionPipeline}
      onPaste={processingModel.pasteCopiedSessionPipeline}
      onRemove={processingModel.removeActiveSessionPipeline}
      onRemoveItem={processingModel.removeSessionPipeline}
      getLabel={processingModel.sessionPipelineLabel}
      getSummary={processingModel.sessionPipelineSummary}
      canRemove={processingModel.canRemoveSessionPipeline}
    />

    <div class="inspector-stack">
      <PipelineControlBar
        processingFamily={processingModel.pipelineFamily}
        pipeline={
          processingModel.pipelineFamily === "post_stack_neighborhood"
            ? processingModel.postStackNeighborhoodPipeline
            : processingModel.pipeline
        }
        previewState={processingModel.previewState}
        previewLabel={processingModel.previewLabel}
        presets={processingModel.visiblePresets}
        loadingPresets={processingModel.loadingPresets}
        canPreview={processingModel.canPreview}
        canRun={processingModel.canRun}
        previewBusy={processingModel.previewBusy}
        runBusy={processingModel.runBusy}
        batchBusy={processingModel.batchBusy}
        activeBatch={processingModel.activeBatch}
        batchCandidates={processingModel.batchCandidates}
        selectedBatchStorePaths={processingModel.selectedBatchStorePaths}
        batchExecutionMode={processingModel.batchExecutionMode}
        batchMaxActiveJobs={processingModel.batchMaxActiveJobs}
        runOutputSettingsOpen={processingModel.runOutputSettingsOpen}
        runOutputPathMode={processingModel.runOutputPathMode}
        runOutputPath={processingModel.resolvedRunOutputPath}
        resolvingRunOutputPath={processingModel.resolvingRunOutputPath}
        overwriteExistingRunOutput={processingModel.overwriteExistingRunOutput}
        onSetPipelineName={processingModel.setPipelineName}
        onPreview={() => processingModel.previewCurrentSection()}
        onShowRaw={processingModel.showRawSection}
        onRun={() => processingModel.runOnVolume()}
        onRunBatch={() => processingModel.runBatchOnVolumes()}
        onCancelBatch={() => processingModel.cancelActiveBatch()}
        onToggleRunOutputSettings={() =>
          processingModel.setRunOutputSettingsOpen(!processingModel.runOutputSettingsOpen)}
        onSetRunOutputPathMode={processingModel.setRunOutputPathMode}
        onSetCustomRunOutputPath={processingModel.setCustomRunOutputPath}
        onBrowseRunOutputPath={() => processingModel.browseRunOutputPath()}
        onResetRunOutputPath={processingModel.resetRunOutputPath}
        onSetOverwriteExistingRunOutput={processingModel.setOverwriteExistingRunOutput}
        onToggleBatchStorePath={processingModel.toggleBatchStorePath}
        onSelectAllBatchCandidates={processingModel.selectAllBatchCandidates}
        onClearBatchSelection={processingModel.clearBatchSelection}
        onSetBatchExecutionMode={processingModel.setBatchExecutionMode}
        onSetBatchMaxActiveJobs={processingModel.setBatchMaxActiveJobs}
        onLoadPreset={processingModel.loadPreset}
        onSavePreset={() => processingModel.savePreset()}
        onDeletePreset={(presetId) => processingModel.deletePreset(presetId)}
      />

      <ProcessingActivityStrip
        entries={processingModel.recentActivityEntries}
        activeJobId={processingModel.activeJob?.job_id ?? null}
        activeBatchId={processingModel.activeBatch?.batch_id ?? null}
        onSelectJob={processingModel.focusRecentJob}
        onSelectBatch={processingModel.focusRecentBatch}
        onOpenArtifact={processingModel.openProcessingArtifact}
        canClearFinished={processingModel.hasClearableRecentActivity}
        onClearFinished={processingModel.clearFinishedRecentActivity}
      />

      <div class="detail-grid">
        {#if processingModel.pipelineFamily === "post_stack_neighborhood"}
          <div class="neighborhood-sequences">
            <PipelineSequenceList
              operations={processingModel.neighborhoodTraceLocalOperations}
              operatorCatalogItems={processingModel.availableOperatorCatalogItems}
              catalogSourceLabel={processingModel.operatorCatalogSourceLabel}
              catalogSourceDetail={processingModel.operatorCatalogSourceDetail}
              catalogEmptyMessage={processingModel.operatorCatalogEmptyMessage}
              traceLocalOperationCount={processingModel.neighborhoodTraceLocalOperations.length}
              hasSubvolumeCrop={false}
              selectedIndex={processingModel.selectedStepIndex}
              checkpointAfterOperationIndexes={[]}
              checkpointWarning={null}
              onSelect={processingModel.selectStep}
              onInsertOperator={processingModel.insertOperatorById}
              onCopy={processingModel.copySelectedOperation}
              onPaste={processingModel.pasteCopiedOperation}
              onRemove={processingModel.removeOperationAt}
              onToggleCheckpoint={() => {}}
            />

            <NeighborhoodSequenceList
              operations={processingModel.neighborhoodOperations}
              selectedIndex={
                processingModel.selectedStepIndex -
                processingModel.neighborhoodTraceLocalOperations.length
              }
              onSelect={(index) =>
                processingModel.selectStep(
                  index + processingModel.neighborhoodTraceLocalOperations.length
                )}
            />
          </div>

          {#if processingModel.selectedOperation}
            <PipelineOperatorEditor
              selectedOperation={processingModel.selectedOperation}
              selectedOperatorCatalogItem={processingModel.selectedOperatorCatalogItem}
              activeJob={processingModel.activeJob}
              activeDebugPlan={processingModel.activeDebugPlan}
              activeRuntimeState={processingModel.activeRuntimeState}
              activeRuntimeEvents={processingModel.activeRuntimeEvents}
              processingError={processingModel.error}
              primaryVolumeLabel={processingModel.activePrimaryVolumeLabel}
              sourceSubvolumeBounds={processingModel.sourceSubvolumeBounds}
              secondaryVolumeOptions={processingModel.volumeArithmeticSecondaryOptions}
              selectedStepCanCheckpoint={false}
              selectedStepCheckpoint={false}
              onSetAmplitudeScalarFactor={processingModel.setSelectedAmplitudeScalarFactor}
              onSetAgcWindow={processingModel.setSelectedAgcWindow}
              onSetPhaseRotationAngle={processingModel.setSelectedPhaseRotationAngle}
              onSetLowpassCorner={processingModel.setSelectedLowpassCorner}
              onSetHighpassCorner={processingModel.setSelectedHighpassCorner}
              onSetBandpassCorner={processingModel.setSelectedBandpassCorner}
              onSetVolumeArithmeticOperator={processingModel.setSelectedVolumeArithmeticOperator}
              onSetVolumeArithmeticSecondaryStorePath={processingModel.setSelectedVolumeArithmeticSecondaryStorePath}
              onSetSubvolumeCropBound={processingModel.setSelectedSubvolumeCropBound}
              onSetSelectedCheckpoint={() => {}}
              canMoveUp={processingModel.canMoveSelectedUp}
              canMoveDown={processingModel.canMoveSelectedDown}
              onMoveUp={processingModel.moveSelectedUp}
              onMoveDown={processingModel.moveSelectedDown}
              onRemove={processingModel.removeSelected}
              onCancelJob={() => processingModel.cancelActiveJob()}
              onOpenArtifact={(storePath) => processingModel.openProcessingArtifact(storePath)}
            />
          {:else}
            <NeighborhoodOperatorEditor
              selectedOperation={processingModel.selectedNeighborhoodOperation}
              activeJob={processingModel.activeJob}
              activeDebugPlan={processingModel.activeDebugPlan}
              activeRuntimeState={processingModel.activeRuntimeState}
              activeRuntimeEvents={processingModel.activeRuntimeEvents}
              processingError={processingModel.error}
              onSetWindow={processingModel.setSelectedNeighborhoodWindow}
              onSetStatistic={processingModel.setSelectedNeighborhoodStatistic}
              onSetDipOutput={processingModel.setSelectedNeighborhoodDipOutput}
              onSetOperationKind={processingModel.setSelectedNeighborhoodOperatorKind}
              onCancelJob={() => processingModel.cancelActiveJob()}
              onOpenArtifact={(storePath) => processingModel.openProcessingArtifact(storePath)}
            />
          {/if}
        {:else}
          <PipelineSequenceList
            operations={processingModel.workspaceOperations}
            operatorCatalogItems={processingModel.availableOperatorCatalogItems}
            catalogSourceLabel={processingModel.operatorCatalogSourceLabel}
            catalogSourceDetail={processingModel.operatorCatalogSourceDetail}
            catalogEmptyMessage={processingModel.operatorCatalogEmptyMessage}
            traceLocalOperationCount={processingModel.pipeline.steps.length}
            hasSubvolumeCrop={processingModel.hasSubvolumeCrop}
            selectedIndex={processingModel.selectedStepIndex}
            checkpointAfterOperationIndexes={processingModel.checkpointAfterOperationIndexes}
            checkpointWarning={processingModel.checkpointWarning}
            onSelect={processingModel.selectStep}
            onInsertOperator={processingModel.insertOperatorById}
            onCopy={processingModel.copySelectedOperation}
            onPaste={processingModel.pasteCopiedOperation}
            onRemove={processingModel.removeOperationAt}
            onToggleCheckpoint={processingModel.toggleCheckpointAfterOperation}
          />

          <PipelineOperatorEditor
            selectedOperation={processingModel.selectedOperation}
            selectedOperatorCatalogItem={processingModel.selectedOperatorCatalogItem}
            activeJob={processingModel.activeJob}
            activeDebugPlan={processingModel.activeDebugPlan}
            activeRuntimeState={processingModel.activeRuntimeState}
            activeRuntimeEvents={processingModel.activeRuntimeEvents}
            processingError={processingModel.error}
            primaryVolumeLabel={processingModel.activePrimaryVolumeLabel}
            sourceSubvolumeBounds={processingModel.sourceSubvolumeBounds}
            secondaryVolumeOptions={processingModel.volumeArithmeticSecondaryOptions}
            selectedStepCanCheckpoint={processingModel.canToggleSelectedCheckpoint}
            selectedStepCheckpoint={processingModel.selectedStepCheckpoint}
            onSetAmplitudeScalarFactor={processingModel.setSelectedAmplitudeScalarFactor}
            onSetAgcWindow={processingModel.setSelectedAgcWindow}
            onSetPhaseRotationAngle={processingModel.setSelectedPhaseRotationAngle}
            onSetLowpassCorner={processingModel.setSelectedLowpassCorner}
            onSetHighpassCorner={processingModel.setSelectedHighpassCorner}
            onSetBandpassCorner={processingModel.setSelectedBandpassCorner}
            onSetVolumeArithmeticOperator={processingModel.setSelectedVolumeArithmeticOperator}
            onSetVolumeArithmeticSecondaryStorePath={processingModel.setSelectedVolumeArithmeticSecondaryStorePath}
            onSetSubvolumeCropBound={processingModel.setSelectedSubvolumeCropBound}
            onSetSelectedCheckpoint={processingModel.setSelectedCheckpoint}
            canMoveUp={processingModel.canMoveSelectedUp}
            canMoveDown={processingModel.canMoveSelectedDown}
            onMoveUp={processingModel.moveSelectedUp}
            onMoveDown={processingModel.moveSelectedDown}
            onRemove={processingModel.removeSelected}
            onCancelJob={() => processingModel.cancelActiveJob()}
            onOpenArtifact={(storePath) => processingModel.openProcessingArtifact(storePath)}
          />
        {/if}
      </div>

      {#if processingModel.pipelineFamily === "trace_local"}
        <SpectrumInspector
          canInspectSpectrum={processingModel.canInspectSpectrum}
          spectrumBusy={processingModel.spectrumBusy}
          spectrumStale={processingModel.spectrumStale}
          spectrumError={processingModel.spectrumError}
          spectrumSelectionSummary={processingModel.spectrumSelectionSummary}
          spectrumAmplitudeScale={processingModel.spectrumAmplitudeScale}
          rawSpectrum={rawSpectrum}
          processedSpectrum={processedSpectrum}
          onSetSpectrumAmplitudeScale={processingModel.setSpectrumAmplitudeScale}
          onRefreshSpectrum={() => processingModel.refreshSpectrum()}
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .workspace-shell {
    display: flex;
    flex-direction: column;
    gap: var(--ui-panel-gap);
    min-height: 0;
    padding: var(--ui-panel-padding) var(--ui-space-5) var(--ui-space-3);
    outline: none;
  }

  .workspace-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-5);
    align-items: flex-start;
  }

  .eyebrow {
    display: inline-block;
    margin-bottom: 2px;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .workspace-header p {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .workflow-origin {
    color: var(--text-primary);
    font-weight: 600;
  }

  .family-switch {
    display: flex;
    gap: var(--ui-space-2);
    align-items: center;
  }

  .family-chip {
    border: 1px solid var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-primary);
    border-radius: var(--ui-radius-md);
    min-height: var(--ui-button-height);
    padding: 0 var(--ui-button-padding-x);
    font-size: 11px;
    cursor: pointer;
  }

  .family-chip.active {
    background: #e8f3fb;
    border-color: #b0d4ee;
    color: #1f5577;
  }

  .workspace-grid {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(220px, 0.7fr) minmax(0, 1.35fr);
    gap: var(--ui-panel-gap);
    flex: 1;
  }

  .inspector-stack {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--ui-panel-gap);
    min-height: 0;
  }

  .detail-grid {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(300px, 0.95fr) minmax(380px, 1.2fr);
    gap: var(--ui-panel-gap);
  }

  .neighborhood-sequences {
    min-height: 0;
    display: grid;
    gap: var(--ui-panel-gap);
    grid-template-rows: minmax(0, 1fr) auto;
  }

  @media (max-width: 1100px) {
    .workspace-header {
      flex-direction: column;
    }

    .workspace-grid {
      grid-template-columns: 1fr;
    }

    .inspector-stack {
      grid-template-rows: auto;
    }

    .detail-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
