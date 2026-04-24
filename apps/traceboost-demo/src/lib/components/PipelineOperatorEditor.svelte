<svelte:options runes={true} />

<script lang="ts">
  import type {
    InspectableProcessingPlan,
    ProcessingJobRuntimeState,
    ProcessingJobArtifact,
    ProcessingJobStatus,
    ProcessingRuntimeEvent,
    SubvolumeCropOperation
  } from "@traceboost/seis-contracts";
  import {
    isAgcRms,
    isAmplitudeScalar,
    isBandpassFilter,
    isCropSubvolume,
    isEnvelope,
    isHighpassFilter,
    isInstantaneousFrequency,
    isInstantaneousPhase,
    isLowpassFilter,
    isPhaseRotation,
    isSweetness,
    isVolumeArithmetic,
    type OperatorCatalogItem,
    type SourceSubvolumeBounds,
    type WorkspaceOperation
  } from "../processing-model.svelte";
  import ProcessingDebugPanel from "./ProcessingDebugPanel.svelte";

  let {
    selectedOperation,
    selectedOperatorCatalogItem,
    activeJob,
    activeDebugPlan = null,
    activeRuntimeState = null,
    activeRuntimeEvents = [],
    processingError,
    primaryVolumeLabel,
    sourceSubvolumeBounds,
    secondaryVolumeOptions,
    selectedStepCanCheckpoint = false,
    selectedStepCheckpoint = false,
    onSetAmplitudeScalarFactor,
    onSetAgcWindow = () => {},
    onSetPhaseRotationAngle = () => {},
    onSetLowpassCorner = () => {},
    onSetHighpassCorner = () => {},
    onSetBandpassCorner = () => {},
    onSetVolumeArithmeticOperator = () => {},
    onSetVolumeArithmeticSecondaryStorePath = () => {},
    onSetSubvolumeCropBound = () => {},
    onSetSelectedCheckpoint = () => {},
    canMoveUp = true,
    canMoveDown = true,
    onMoveUp,
    onMoveDown,
    onRemove,
    onCancelJob,
    onOpenArtifact
  }: {
    selectedOperation: WorkspaceOperation | null;
    selectedOperatorCatalogItem: OperatorCatalogItem | null;
    activeJob: ProcessingJobStatus | null;
    activeDebugPlan?: InspectableProcessingPlan | null;
    activeRuntimeState?: ProcessingJobRuntimeState | null;
    activeRuntimeEvents?: ProcessingRuntimeEvent[];
    processingError: string | null;
    primaryVolumeLabel: string;
    sourceSubvolumeBounds: SourceSubvolumeBounds | null;
    secondaryVolumeOptions: { storePath: string; label: string }[];
    selectedStepCanCheckpoint?: boolean;
    selectedStepCheckpoint?: boolean;
    onSetAmplitudeScalarFactor: (value: number) => void;
    onSetAgcWindow?: (value: number) => void;
    onSetPhaseRotationAngle?: (value: number) => void;
    onSetLowpassCorner?: (corner: "f3_hz" | "f4_hz", value: number) => void;
    onSetHighpassCorner?: (corner: "f1_hz" | "f2_hz", value: number) => void;
    onSetBandpassCorner?: (corner: "f1_hz" | "f2_hz" | "f3_hz" | "f4_hz", value: number) => void;
    onSetVolumeArithmeticOperator?: (value: "add" | "subtract" | "multiply" | "divide") => void;
    onSetVolumeArithmeticSecondaryStorePath?: (value: string) => void;
    onSetSubvolumeCropBound?: (
      bound: keyof SubvolumeCropOperation,
      value: number
    ) => void;
    onSetSelectedCheckpoint?: (value: boolean) => void;
    canMoveUp?: boolean;
    canMoveDown?: boolean;
    onMoveUp: () => void;
    onMoveDown: () => void;
    onRemove: () => void;
    onCancelJob: () => void | Promise<void>;
    onOpenArtifact: (storePath: string) => void | Promise<void>;
  } = $props();

  function artifactKindLabel(artifact: ProcessingJobArtifact): string {
    return artifact.kind === "final_output" ? "Final output" : "Checkpoint";
  }

  function documentationParagraphs(markdown: string | null | undefined): string[] {
    return (markdown ?? "")
      .split(/\n\s*\n/g)
      .map((paragraph) => paragraph.trim())
      .filter((paragraph) => paragraph.length > 0);
  }

  function parameterDoc(name: string) {
    return selectedOperatorCatalogItem?.parameterDocs.find((parameter) => parameter.name === name) ?? null;
  }

  function parameterLabel(name: string, fallback: string): string {
    return parameterDoc(name)?.label ?? fallback;
  }

  function parameterDescription(name: string, fallback: string | null = null): string | null {
    return parameterDoc(name)?.description ?? fallback;
  }
</script>

<section class="editor-panel">
  <header class="editor-header">
    <h3>Step Editor</h3>
    <p>Adjust the selected operator parameters and manage ordering.</p>
  </header>

  {#if selectedOperation}
    <div class="selected-card">
      {#if selectedOperatorCatalogItem}
        <div class="operator-identity">
          <div class="operator-identity-header">
            <strong>{selectedOperatorCatalogItem.label}</strong>
            <span>{selectedOperatorCatalogItem.group}</span>
          </div>
          <p>{selectedOperatorCatalogItem.description}</p>
          <div class="operator-doc-copy">
            <strong>Help</strong>
            <p>{selectedOperatorCatalogItem.shortHelp}</p>
            {#each documentationParagraphs(selectedOperatorCatalogItem.helpMarkdown) as paragraph (`${selectedOperatorCatalogItem.canonicalId}:${paragraph}`)}
              {#if paragraph !== selectedOperatorCatalogItem.shortHelp}
                <p>{paragraph}</p>
              {/if}
            {/each}
            {#if selectedOperatorCatalogItem.helpUrl}
              <a href={selectedOperatorCatalogItem.helpUrl} target="_blank" rel="noreferrer">
                Open reference
              </a>
            {/if}
          </div>
          <div class="operator-identity-meta">
            {#if selectedOperatorCatalogItem.aliasLabel && selectedOperatorCatalogItem.canonicalName !== selectedOperatorCatalogItem.aliasLabel}
              <span>Canonical: {selectedOperatorCatalogItem.canonicalName}</span>
            {/if}
            <span>Group Id: {selectedOperatorCatalogItem.groupId}</span>
            <span>Provider: {selectedOperatorCatalogItem.provider}</span>
            {#if selectedOperatorCatalogItem.tags.length}
              <span>Tags: {selectedOperatorCatalogItem.tags.join(", ")}</span>
            {/if}
          </div>
          {#if selectedOperatorCatalogItem.parameterDocs.length}
            <div class="operator-parameter-docs">
              <strong>Parameters</strong>
              {#each selectedOperatorCatalogItem.parameterDocs as parameter (`${selectedOperatorCatalogItem.canonicalId}:${parameter.name}`)}
                <div class="operator-parameter-doc">
                  <div class="operator-parameter-doc-header">
                    <span>{parameter.label}</span>
                    <code>{parameter.name}</code>
                  </div>
                  <p>{parameter.description}</p>
                  <div class="operator-parameter-doc-meta">
                    <span>Type: {parameter.value_kind}</span>
                    <span>{parameter.required ? "Required" : "Optional"}</span>
                    {#if parameter.units}
                      <span>Units: {parameter.units}</span>
                    {/if}
                    {#if parameter.default_value !== null}
                      <span>Default: {parameter.default_value}</span>
                    {/if}
                    {#if parameter.minimum !== null}
                      <span>Min: {parameter.minimum}</span>
                    {/if}
                    {#if parameter.maximum !== null}
                      <span>Max: {parameter.maximum}</span>
                    {/if}
                    {#if parameter.options.length}
                      <span>Options: {parameter.options.join(", ")}</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <div class="selected-actions">
        <button class="chip" onclick={onMoveUp} disabled={!canMoveUp}>Move Up</button>
        <button class="chip" onclick={onMoveDown} disabled={!canMoveDown}>Move Down</button>
        <button class="chip danger" onclick={onRemove}>Delete Step</button>
      </div>

      {#if !isCropSubvolume(selectedOperation)}
        <label class="checkpoint-toggle">
          <input
            type="checkbox"
            checked={selectedStepCheckpoint}
            disabled={!selectedStepCanCheckpoint}
            onchange={(event) => onSetSelectedCheckpoint((event.currentTarget as HTMLInputElement).checked)}
          />
          <span>Save Output After This Step</span>
        </label>
        {#if !selectedStepCanCheckpoint}
          <small class="checkpoint-note">Final trace-local output is emitted automatically unless a crop tail follows it.</small>
        {/if}
      {/if}

      {#if isCropSubvolume(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>{parameterLabel("inline_min", "Inline Min")}</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.inline_min}
              oninput={(event) =>
                onSetSubvolumeCropBound("inline_min", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("inline_max", "Inline Max")}</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.inline_max}
              oninput={(event) =>
                onSetSubvolumeCropBound("inline_max", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("xline_min", "Xline Min")}</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.xline_min}
              oninput={(event) =>
                onSetSubvolumeCropBound("xline_min", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("xline_max", "Xline Max")}</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.xline_max}
              oninput={(event) =>
                onSetSubvolumeCropBound("xline_max", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("z_min_ms", "Z Min")}</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.z_min_ms}
              oninput={(event) =>
                onSetSubvolumeCropBound("z_min_ms", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("z_max_ms", "Z Max")}</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.z_max_ms}
              oninput={(event) =>
                onSetSubvolumeCropBound("z_max_ms", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
        </div>
        <div class="info-block">
          <strong>Crop Subvolume</strong>
          {#if sourceSubvolumeBounds}
            <p>
              Source bounds: IL {sourceSubvolumeBounds.inlineMin}-{sourceSubvolumeBounds.inlineMax},
              XL {sourceSubvolumeBounds.xlineMin}-{sourceSubvolumeBounds.xlineMax},
              Z {sourceSubvolumeBounds.zMinMs}-{sourceSubvolumeBounds.zMaxMs}
              {sourceSubvolumeBounds.zUnits ?? "ms"}.
            </p>
          {/if}
          <p>Crop is a terminal derivation step. Preview shows only the processing prefix above it.</p>
          <p>Bounds must stay within the source volume and define a strict subset on at least one axis.</p>
        </div>
      {:else if isAmplitudeScalar(selectedOperation)}
        <label class="field">
          <span>{parameterLabel("factor", "Factor")}</span>
          <input
            type="number"
            min="0"
            max="10"
            step="0.1"
            value={selectedOperation.amplitude_scalar.factor}
            oninput={(event) =>
              onSetAmplitudeScalarFactor(Number((event.currentTarget as HTMLInputElement).value))}
          />
          <small>{parameterDescription("factor", "Linear multiplier applied to every trace sample.")}</small>
        </label>
      {:else if isAgcRms(selectedOperation)}
        <label class="field">
          <span>{parameterLabel("window_ms", "Window")}</span>
          <input
            type="number"
            min="1"
            max="10000"
            step="10"
            value={selectedOperation.agc_rms.window_ms}
            oninput={(event) => onSetAgcWindow(Number((event.currentTarget as HTMLInputElement).value))}
          />
          <small>{parameterDescription("window_ms", "Centered RMS window length used for AGC balancing.")}</small>
        </label>
      {:else if isPhaseRotation(selectedOperation)}
        <label class="field">
          <span>{parameterLabel("angle_degrees", "Angle")}</span>
          <input
            type="number"
            min="-180"
            max="180"
            step="1"
            value={selectedOperation.phase_rotation.angle_degrees}
            oninput={(event) =>
              onSetPhaseRotationAngle(Number((event.currentTarget as HTMLInputElement).value))}
          />
          <small>{parameterDescription("angle_degrees", "Constant phase rotation angle applied to the trace.")}</small>
        </label>
      {:else if isLowpassFilter(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>{parameterLabel("f3_hz", "F3")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.lowpass_filter.f3_hz}
              oninput={(event) =>
                onSetLowpassCorner("f3_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("f4_hz", "F4")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.lowpass_filter.f4_hz}
              oninput={(event) =>
                onSetLowpassCorner("f4_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
        </div>
        <div class="info-block">
          <strong>Current Filter Mode</strong>
          <p>{parameterDescription("phase", "Phase mode used by the spectral filter.")} {selectedOperation.lowpass_filter.phase}.</p>
          <p>{parameterDescription("window", "Transition window used in the taper region.")} {selectedOperation.lowpass_filter.window}.</p>
        </div>
      {:else if isHighpassFilter(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>{parameterLabel("f1_hz", "F1")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.highpass_filter.f1_hz}
              oninput={(event) =>
                onSetHighpassCorner("f1_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("f2_hz", "F2")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.highpass_filter.f2_hz}
              oninput={(event) =>
                onSetHighpassCorner("f2_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
        </div>
        <div class="info-block">
          <strong>Current Filter Mode</strong>
          <p>{parameterDescription("phase", "Phase mode used by the spectral filter.")} {selectedOperation.highpass_filter.phase}.</p>
          <p>{parameterDescription("window", "Transition window used in the taper region.")} {selectedOperation.highpass_filter.window}.</p>
        </div>
      {:else if isBandpassFilter(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>{parameterLabel("f1_hz", "F1")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.bandpass_filter.f1_hz}
              oninput={(event) =>
                onSetBandpassCorner("f1_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("f2_hz", "F2")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.bandpass_filter.f2_hz}
              oninput={(event) =>
                onSetBandpassCorner("f2_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("f3_hz", "F3")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.bandpass_filter.f3_hz}
              oninput={(event) =>
                onSetBandpassCorner("f3_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>{parameterLabel("f4_hz", "F4")}</span>
            <input
              type="number"
              min="0"
              step="0.5"
              value={selectedOperation.bandpass_filter.f4_hz}
              oninput={(event) =>
                onSetBandpassCorner("f4_hz", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
        </div>
        <div class="info-block">
          <strong>Current Filter Mode</strong>
          <p>{parameterDescription("phase", "Phase mode used by the spectral filter.")} {selectedOperation.bandpass_filter.phase}.</p>
          <p>{parameterDescription("window", "Transition window used in the taper region.")} {selectedOperation.bandpass_filter.window}.</p>
        </div>
      {:else if isVolumeArithmetic(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>{parameterLabel("operator", "Operator")}</span>
            <select
              value={selectedOperation.volume_arithmetic.operator}
              onchange={(event) =>
                onSetVolumeArithmeticOperator((event.currentTarget as HTMLSelectElement).value as "add" | "subtract" | "multiply" | "divide")}
            >
              <option value="subtract">Subtract</option>
              <option value="add">Add</option>
              <option value="multiply">Multiply</option>
              <option value="divide">Divide</option>
            </select>
          </label>
          <label class="field">
            <span>Primary Volume</span>
            <input type="text" value={primaryVolumeLabel} readonly />
          </label>
        </div>
        <label class="field">
          <span>{parameterLabel("secondary_input", "Secondary Input")}</span>
          <select
            value={selectedOperation.volume_arithmetic.secondary_store_path}
            disabled={!secondaryVolumeOptions.length}
            onchange={(event) =>
              onSetVolumeArithmeticSecondaryStorePath((event.currentTarget as HTMLSelectElement).value)}
          >
            <option value="">Select compatible volume...</option>
            {#each secondaryVolumeOptions as option (option.storePath)}
              <option value={option.storePath}>{option.label}</option>
            {/each}
          </select>
          <small>{parameterDescription("secondary_input", "Reference to a second compatible seismic volume.")}</small>
        </label>
        <div class="info-block">
          <strong>Compatibility</strong>
          <p>TraceBoost only lists workspace volumes whose geometry fingerprint and tile layout match the active volume.</p>
        </div>
      {:else}
        <div class="info-block">
          <strong>No Editable Parameters</strong>
          <p>This operator is configured entirely by its canonical defaults in the current runtime.</p>
        </div>
      {/if}
    </div>
  {:else}
    <div class="info-block empty">
      <strong>No step selected</strong>
      <p>Select a pipeline step to edit it.</p>
    </div>
  {/if}

  {#if activeJob}
    <div class="job-card">
      <div class="job-header">
        <strong>Background Job</strong>
        <span>{activeJob.state}</span>
      </div>
      {#if activeJob.current_stage_label}
        <div class="job-stage">{activeJob.current_stage_label}</div>
      {/if}
      <div class="job-progress">
        {activeJob.progress.completed} / {activeJob.progress.total || 0} tiles
      </div>
      <ProcessingDebugPanel
        {activeJob}
        debugPlan={activeDebugPlan}
        runtimeState={activeRuntimeState}
        runtimeEvents={activeRuntimeEvents}
        {onCancelJob}
        {onOpenArtifact}
      />
    </div>
  {/if}

  {#if processingError}
    <div class="error-bar">{processingError}</div>
  {/if}
</section>

<style>
  .editor-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: 8px;
    padding: 10px;
    overflow: auto;
  }

  .editor-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .editor-header h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .editor-header p {
    margin: 0;
    color: var(--text-muted);
    font-size: 11px;
  }

  .selected-actions {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }

  .operator-identity {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
  }

  .operator-identity-header {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: baseline;
    flex-wrap: wrap;
  }

  .operator-identity-header strong {
    color: var(--text-primary);
    font-size: 12px;
  }

  .operator-identity-header span {
    color: var(--text-dim);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .operator-identity p {
    margin: 0;
    color: var(--text-muted);
    font-size: 11px;
  }

  .operator-doc-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 4px;
    border-top: 1px solid var(--app-border);
  }

  .operator-doc-copy strong,
  .operator-parameter-docs strong {
    color: var(--text-primary);
    font-size: 11px;
  }

  .operator-doc-copy a {
    color: #315b75;
    font-size: 11px;
    text-decoration: none;
  }

  .operator-identity-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    color: var(--text-dim);
    font-size: 10px;
  }

  .operator-parameter-docs {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 4px;
    border-top: 1px solid var(--app-border);
  }

  .operator-parameter-doc {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 8px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: #fff;
  }

  .operator-parameter-doc-header {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: baseline;
    flex-wrap: wrap;
    color: var(--text-primary);
    font-size: 11px;
    font-weight: 600;
  }

  .operator-parameter-doc-header code {
    color: var(--text-dim);
    font-size: 10px;
  }

  .operator-parameter-doc p {
    margin: 0;
    color: var(--text-muted);
    font-size: 11px;
  }

  .operator-parameter-doc-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    color: var(--text-dim);
    font-size: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .checkpoint-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 0 2px;
    color: var(--text-primary);
    font-size: 11px;
  }

  .checkpoint-toggle input {
    margin: 0;
  }

  .checkpoint-note {
    display: block;
    margin: 0 0 10px;
    color: var(--text-muted);
    font-size: 11px;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .field span {
    font-size: 11px;
    color: var(--text-muted);
  }

  .field input {
    background: #fff;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    color: var(--text-primary);
    padding: 6px 8px;
    font: inherit;
    font-size: 12px;
  }

  .field select {
    background: #fff;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    color: var(--text-primary);
    padding: 6px 8px;
    font: inherit;
    font-size: 12px;
  }

  .field small {
    color: var(--text-dim);
    font-size: 11px;
  }

  .chip {
    border: 1px solid var(--app-border);
    background: var(--surface-subtle);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 11px;
    cursor: pointer;
  }

  .chip:hover:not(:disabled) {
    background: var(--surface-bg);
    color: var(--text-primary);
  }

  .chip.danger {
    border-color: #e0b7b7;
    color: #a74646;
  }

  .chip:disabled {
    opacity: 0.38;
    cursor: not-allowed;
  }

  .selected-card,
  .job-card,
  .info-block {
    border: 1px solid var(--app-border);
    border-radius: 8px;
    padding: 10px;
    background: var(--surface-bg);
  }

  .info-block strong,
  .job-header strong {
    display: block;
    margin-bottom: 4px;
    color: var(--text-primary);
    font-size: 12px;
  }

  .info-block p,
  .job-progress {
    margin: 0;
    color: var(--text-muted);
    font-size: 11px;
    line-height: 1.5;
  }

  .job-header {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: center;
    margin-bottom: 6px;
  }

  .job-header span {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
  }

  .job-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .job-stage {
    font-size: 11px;
    color: #315b75;
  }

  .error-bar {
    border: 1px solid #e0b7b7;
    border-radius: 8px;
    background: #f9ecec;
    color: #8f3c3c;
    font-size: 11px;
    padding: 8px 10px;
  }

  @media (max-width: 720px) {
    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
