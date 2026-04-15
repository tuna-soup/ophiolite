<svelte:options runes={true} />

<script lang="ts">
  import type {
    ProcessingJobArtifact,
    ProcessingJobStatus,
    SubvolumeCropOperation
  } from "@traceboost/seis-contracts";
  import {
    isAgcRms,
    isAmplitudeScalar,
    isBandpassFilter,
    isCropSubvolume,
    isHighpassFilter,
    isLowpassFilter,
    isPhaseRotation,
    isVolumeArithmetic,
    type SourceSubvolumeBounds,
    type WorkspaceOperation
  } from "../processing-model.svelte";

  let {
    selectedOperation,
    activeJob,
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
    activeJob: ProcessingJobStatus | null;
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
</script>

<section class="editor-panel">
  <header class="editor-header">
    <h3>Step Editor</h3>
    <p>Adjust the selected operator parameters and manage ordering.</p>
  </header>

  {#if selectedOperation}
    <div class="selected-card">
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
            <span>Inline Min</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.inline_min}
              oninput={(event) =>
                onSetSubvolumeCropBound("inline_min", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>Inline Max</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.inline_max}
              oninput={(event) =>
                onSetSubvolumeCropBound("inline_max", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>Xline Min</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.xline_min}
              oninput={(event) =>
                onSetSubvolumeCropBound("xline_min", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>Xline Max</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.xline_max}
              oninput={(event) =>
                onSetSubvolumeCropBound("xline_max", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>Z Min</span>
            <input
              type="number"
              value={selectedOperation.crop_subvolume.z_min_ms}
              oninput={(event) =>
                onSetSubvolumeCropBound("z_min_ms", Number((event.currentTarget as HTMLInputElement).value))}
            />
          </label>
          <label class="field">
            <span>Z Max</span>
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
          <p>Crop Subvolume is a run-volume tail step. Preview shows only the processing steps above it.</p>
          <p>It is always appended to the end of the pipeline and only one crop can be active at a time.</p>
          <p>Bounds must stay within the source volume and define a strict subset on at least one axis.</p>
        </div>
      {:else if isAmplitudeScalar(selectedOperation)}
        <label class="field">
          <span>Amplitude Scalar Factor</span>
          <input
            type="number"
            min="0"
            max="10"
            step="0.1"
            value={selectedOperation.amplitude_scalar.factor}
            oninput={(event) =>
              onSetAmplitudeScalarFactor(Number((event.currentTarget as HTMLInputElement).value))}
          />
          <small>Valid range: 0.0 to 10.0</small>
        </label>
      {:else if isAgcRms(selectedOperation)}
        <label class="field">
          <span>AGC Window</span>
          <input
            type="number"
            min="1"
            max="10000"
            step="10"
            value={selectedOperation.agc_rms.window_ms}
            oninput={(event) => onSetAgcWindow(Number((event.currentTarget as HTMLInputElement).value))}
          />
          <small>Milliseconds. Backend validation enforces a positive centered RMS window.</small>
        </label>
        <div class="info-block">
          <strong>RMS AGC</strong>
          <p>Automatic gain control using a centered moving RMS window. This is useful for balancing weak and strong events in post-stack sections.</p>
          <p>AGC changes relative amplitudes, so treat it as conditioning rather than amplitude-preserving processing.</p>
        </div>
      {:else if isPhaseRotation(selectedOperation)}
        <label class="field">
          <span>Phase Rotation Angle</span>
          <input
            type="number"
            min="-180"
            max="180"
            step="1"
            value={selectedOperation.phase_rotation.angle_degrees}
            oninput={(event) =>
              onSetPhaseRotationAngle(Number((event.currentTarget as HTMLInputElement).value))}
          />
          <small>Degrees. 0 = unchanged, 90 = quadrature, 180 = polarity flip.</small>
        </label>
        <div class="info-block">
          <strong>Phase Rotation</strong>
          <p>Constant trace phase rotation applied in the spectral domain using the analytic-trace formulation.</p>
          <p>Phase rotation changes wavelet shape and timing character but preserves amplitude spectrum magnitude.</p>
        </div>
      {:else if isLowpassFilter(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>F3 Pass Corner</span>
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
            <span>F4 Stop Corner</span>
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
          <strong>Lowpass Filter</strong>
          <p>Zero-phase frequency-domain lowpass with a cosine high-cut taper. Runtime validation enforces f3 ≤ f4 ≤ Nyquist.</p>
          <p>Phase: {selectedOperation.lowpass_filter.phase}. Window: {selectedOperation.lowpass_filter.window}.</p>
        </div>
      {:else if isHighpassFilter(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>F1 Stop Corner</span>
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
            <span>F2 Pass Corner</span>
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
          <strong>Highpass Filter</strong>
          <p>Zero-phase frequency-domain highpass with a cosine low-cut taper. Runtime validation enforces f1 ≤ f2 ≤ Nyquist.</p>
          <p>Phase: {selectedOperation.highpass_filter.phase}. Window: {selectedOperation.highpass_filter.window}.</p>
        </div>
      {:else if isBandpassFilter(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>F1 Low Stop</span>
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
            <span>F2 Low Pass</span>
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
            <span>F3 High Pass</span>
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
            <span>F4 High Stop</span>
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
          <strong>Bandpass Filter</strong>
          <p>Zero-phase frequency-domain bandpass with cosine tapers. Runtime validation enforces f1 ≤ f2 ≤ f3 ≤ f4 ≤ Nyquist.</p>
          <p>Phase: {selectedOperation.bandpass_filter.phase}. Window: {selectedOperation.bandpass_filter.window}.</p>
        </div>
      {:else if isVolumeArithmetic(selectedOperation)}
        <div class="field-grid">
          <label class="field">
            <span>Arithmetic Mode</span>
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
          <span>Secondary Volume</span>
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
          <small>TraceBoost only lists workspace volumes whose geometry fingerprint and tile layout match the active volume.</small>
        </label>
        <div class="info-block">
          <strong>Volume Arithmetic</strong>
          <p>Combines the active volume with another compatible workspace volume sample-by-sample.</p>
          <p>Subtract is the usual difference-volume workflow. Multiply and divide treat missing secondary traces as zeros.</p>
        </div>
      {:else}
        <div class="info-block">
          <strong>Trace RMS Normalize</strong>
          <p>Scales each trace so its RMS amplitude becomes 1.0, with backend safeguards for zero-amplitude traces.</p>
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
      {#if activeJob.state === "queued" || activeJob.state === "running"}
        <button class="chip danger" onclick={onCancelJob}>Cancel Job</button>
      {/if}
      {#if activeJob.artifacts.length}
        <div class="artifact-list">
          {#each activeJob.artifacts as artifact (`${artifact.kind}:${artifact.store_path}`)}
            <div class="artifact-row">
              <div class="artifact-copy">
                <strong>{artifact.label}</strong>
                <span>{artifactKindLabel(artifact)}</span>
              </div>
              <button class="chip" onclick={() => onOpenArtifact(artifact.store_path)}>Open</button>
            </div>
          {/each}
        </div>
      {/if}
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

  .artifact-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 2px;
    padding-top: 6px;
    border-top: 1px solid var(--app-border);
  }

  .artifact-row {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: center;
  }

  .artifact-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .artifact-copy strong {
    font-size: 11px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artifact-copy span {
    font-size: 10px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
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
