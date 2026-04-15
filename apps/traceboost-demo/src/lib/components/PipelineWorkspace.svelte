<svelte:options runes={true} />

<script lang="ts">
  import PipelineControlBar from "./PipelineControlBar.svelte";
  import PipelineOperatorEditor from "./PipelineOperatorEditor.svelte";
  import PipelineSequenceList from "./PipelineSequenceList.svelte";
  import PipelineSessionList from "./PipelineSessionList.svelte";
  import SpectrumInspector from "./SpectrumInspector.svelte";
  import { getProcessingModelContext } from "../processing-model.svelte";
  import { getViewerModelContext } from "../viewer-model.svelte";

  const viewerModel = getViewerModelContext();
  const processingModel = getProcessingModelContext();
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
      canRemove={processingModel.canRemoveSessionPipeline}
    />

    <div class="inspector-stack">
      <PipelineControlBar
        pipeline={processingModel.pipeline}
        previewState={processingModel.previewState}
        previewLabel={processingModel.previewLabel}
        presets={processingModel.presets}
        loadingPresets={processingModel.loadingPresets}
        canPreview={processingModel.canPreview}
        canRun={processingModel.canRun}
        previewBusy={processingModel.previewBusy}
        runBusy={processingModel.runBusy}
        runOutputSettingsOpen={processingModel.runOutputSettingsOpen}
        runOutputPathMode={processingModel.runOutputPathMode}
        runOutputPath={processingModel.resolvedRunOutputPath}
        resolvingRunOutputPath={processingModel.resolvingRunOutputPath}
        overwriteExistingRunOutput={processingModel.overwriteExistingRunOutput}
        onSetPipelineName={processingModel.setPipelineName}
        onPreview={() => processingModel.previewCurrentSection()}
        onShowRaw={processingModel.showRawSection}
        onRun={() => processingModel.runOnVolume()}
        onToggleRunOutputSettings={() =>
          processingModel.setRunOutputSettingsOpen(!processingModel.runOutputSettingsOpen)}
        onSetRunOutputPathMode={processingModel.setRunOutputPathMode}
        onSetCustomRunOutputPath={processingModel.setCustomRunOutputPath}
        onBrowseRunOutputPath={() => processingModel.browseRunOutputPath()}
        onResetRunOutputPath={processingModel.resetRunOutputPath}
        onSetOverwriteExistingRunOutput={processingModel.setOverwriteExistingRunOutput}
        onLoadPreset={processingModel.loadPreset}
        onSavePreset={() => processingModel.savePreset()}
        onDeletePreset={(presetId) => processingModel.deletePreset(presetId)}
      />

      <div class="detail-grid">
        <PipelineSequenceList
          operations={processingModel.workspaceOperations}
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
          activeJob={processingModel.activeJob}
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
      </div>

      <SpectrumInspector
        canInspectSpectrum={processingModel.canInspectSpectrum}
        spectrumBusy={processingModel.spectrumBusy}
        spectrumStale={processingModel.spectrumStale}
        spectrumError={processingModel.spectrumError}
        spectrumSelectionSummary={processingModel.spectrumSelectionSummary}
        spectrumAmplitudeScale={processingModel.spectrumAmplitudeScale}
        rawSpectrum={processingModel.rawSpectrum}
        processedSpectrum={processingModel.processedSpectrum}
        onSetSpectrumAmplitudeScale={processingModel.setSpectrumAmplitudeScale}
        onRefreshSpectrum={() => processingModel.refreshSpectrum()}
      />
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
