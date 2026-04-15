<svelte:options runes={true} />

<script lang="ts">
  import type {
    WellTieChartModel,
    WellTieCurveTrack,
    WellTieSectionPanel,
    WellTieTrack,
    WellTieWavelet,
    WellTieWiggleTrack
  } from "@ophiolite/charts-data-models";

  interface TickValue {
    value: number;
    label: string;
  }

  interface TrackFrame {
    track: WellTieTrack;
    x: number;
    width: number;
    centerX: number;
  }

  interface SectionCell {
    x: number;
    y: number;
    width: number;
    height: number;
    color: string;
  }

  const tieWidth = 760;
  const tieHeight = 780;
  const tieTop = 52;
  const tieBottom = 24;
  const tieLeft = 70;
  const tieTrackGap = 12;
  const tieTrackWidth = 122;
  const tiePlotHeight = tieHeight - tieTop - tieBottom;

  const sectionWidth = 336;
  const sectionHeight = 560;
  const sectionTop = 42;
  const sectionLeft = 18;
  const sectionRight = 18;
  const sectionBottom = 28;
  const sectionPlotWidth = sectionWidth - sectionLeft - sectionRight;
  const sectionPlotHeight = sectionHeight - sectionTop - sectionBottom;

  const waveletWidth = 188;
  const waveletHeight = 280;
  const waveletTop = 42;
  const waveletLeft = 30;
  const waveletRight = 18;
  const waveletBottom = 22;
  const waveletPlotWidth = waveletWidth - waveletLeft - waveletRight;
  const waveletPlotHeight = waveletHeight - waveletTop - waveletBottom;

  let {
    model = null,
    ariaLabel = "Well tie chart",
    emptyMessage = "Prepare a well tie to render the integrated AI, synthetic, seismic, and wavelet views."
  }: {
    model?: WellTieChartModel | null;
    ariaLabel?: string;
    emptyMessage?: string;
  } = $props();

  const timeTicks = $derived.by<TickValue[]>(() => {
    if (!model) {
      return [];
    }
    return buildTicks(model.timeRangeMs.start, model.timeRangeMs.end, 7);
  });
  const trackFrames = $derived.by<TrackFrame[]>(() =>
    (model?.tracks ?? []).map((track, index) => ({
      track,
      x: tieLeft + index * (tieTrackWidth + tieTrackGap),
      width: tieTrackWidth,
      centerX: tieLeft + index * (tieTrackWidth + tieTrackGap) + tieTrackWidth / 2
    }))
  );
  const tiePlotWidth = $derived.by(() => {
    if (!trackFrames.length) {
      return 0;
    }
    const lastTrack = trackFrames[trackFrames.length - 1]!;
    return lastTrack.x + lastTrack.width - tieLeft;
  });
  const tieSvgWidth = $derived(tieLeft + tiePlotWidth + 24);
  const sectionCells = $derived.by<SectionCell[]>(() => buildSectionCells(model?.section ?? null));
  const waveletTicks = $derived.by<TickValue[]>(() => {
    if (!model?.wavelet) {
      return [];
    }
    return buildTicks(
      model.wavelet.timesMs[0] ?? -80,
      model.wavelet.timesMs[model.wavelet.timesMs.length - 1] ?? 80,
      5
    );
  });

  function isCurveTrack(track: WellTieTrack): track is WellTieCurveTrack {
    return track.kind === "curve";
  }

  function isWiggleTrack(track: WellTieTrack): track is WellTieWiggleTrack {
    return track.kind === "wiggle";
  }

  function buildTicks(min: number, max: number, count: number): TickValue[] {
    if (!Number.isFinite(min) || !Number.isFinite(max) || max <= min || count < 2) {
      return [{ value: 0, label: "0" }];
    }

    return Array.from({ length: count }, (_, index) => {
      const value = min + ((max - min) * index) / (count - 1);
      return {
        value,
        label: value.toFixed(0)
      };
    });
  }

  function scaleTime(timeMs: number): number {
    if (!model) {
      return tieTop;
    }

    const { start, end } = model.timeRangeMs;
    if (end <= start) {
      return tieTop;
    }

    const clamped = Math.max(start, Math.min(end, timeMs));
    return tieTop + ((clamped - start) / (end - start)) * tiePlotHeight;
  }

  function curveValueRange(track: WellTieCurveTrack): { min: number; max: number } {
    if (track.valueRange) {
      return track.valueRange;
    }

    let min = Number.POSITIVE_INFINITY;
    let max = Number.NEGATIVE_INFINITY;
    for (const value of track.values) {
      min = Math.min(min, value);
      max = Math.max(max, value);
    }

    if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) {
      return { min: 0, max: 1 };
    }

    return { min, max };
  }

  function scaleCurveValue(track: WellTieCurveTrack, value: number, frame: TrackFrame): number {
    const range = curveValueRange(track);
    if (range.max <= range.min) {
      return frame.centerX;
    }

    const normalized = (value - range.min) / (range.max - range.min);
    return frame.x + normalized * frame.width;
  }

  function curvePath(track: WellTieCurveTrack, frame: TrackFrame): string {
    const points: string[] = [];

    for (let index = 0; index < track.timesMs.length; index += 1) {
      const x = scaleCurveValue(track, track.values[index] ?? 0, frame);
      const y = scaleTime(track.timesMs[index] ?? 0);
      points.push(`${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`);
    }

    return points.join(" ");
  }

  function wiggleLinePath(track: WellTieWiggleTrack, frame: TrackFrame): string {
    const points: string[] = [];
    const scale = frame.width * 0.44 * (track.amplitudeScale ?? 1);

    for (let index = 0; index < track.timesMs.length; index += 1) {
      const x = frame.centerX + (track.amplitudes[index] ?? 0) * scale;
      const y = scaleTime(track.timesMs[index] ?? 0);
      points.push(`${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`);
    }

    return points.join(" ");
  }

  function wiggleFillPath(track: WellTieWiggleTrack, frame: TrackFrame, polarity: "positive" | "negative"): string {
    const scale = frame.width * 0.44 * (track.amplitudeScale ?? 1);
    const forward: string[] = [];
    const reverse: string[] = [];

    for (let index = 0; index < track.timesMs.length; index += 1) {
      const amplitude = track.amplitudes[index] ?? 0;
      const component = polarity === "positive" ? Math.max(0, amplitude) : Math.min(0, amplitude);
      const y = scaleTime(track.timesMs[index] ?? 0);
      const x = frame.centerX + component * scale;
      forward.push(`${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`);
    }

    for (let index = track.timesMs.length - 1; index >= 0; index -= 1) {
      const y = scaleTime(track.timesMs[index] ?? 0);
      reverse.push(`L${frame.centerX.toFixed(2)},${y.toFixed(2)}`);
    }

    return `${forward.join(" ")} ${reverse.join(" ")} Z`;
  }

  function markerLineColor(color?: string): string {
    return color?.trim() || "#c65d2f";
  }

  function buildSectionCells(section: WellTieSectionPanel | null): SectionCell[] {
    if (!section) {
      return [];
    }

    const traceStep = section.traceCount > 24 ? Math.ceil(section.traceCount / 24) : 1;
    const sampleStep = section.sampleCount > 180 ? Math.ceil(section.sampleCount / 180) : 1;
    const cells: SectionCell[] = [];
    const renderedTraceCount = Math.ceil(section.traceCount / traceStep);
    const renderedSampleCount = Math.ceil(section.sampleCount / sampleStep);
    const cellWidth = sectionPlotWidth / Math.max(renderedTraceCount, 1);
    const cellHeight = sectionPlotHeight / Math.max(renderedSampleCount, 1);

    for (let traceIndex = 0; traceIndex < section.traceCount; traceIndex += traceStep) {
      for (let sampleIndex = 0; sampleIndex < section.sampleCount; sampleIndex += sampleStep) {
        const amplitude = section.amplitudes[traceIndex * section.sampleCount + sampleIndex] ?? 0;
        cells.push({
          x: sectionLeft + (traceIndex / traceStep) * cellWidth,
          y: sectionTop + (sampleIndex / sampleStep) * cellHeight,
          width: cellWidth + 0.2,
          height: cellHeight + 0.2,
          color: seismicColor(amplitude)
        });
      }
    }

    return cells;
  }

  function seismicColor(amplitude: number): string {
    const clipped = Math.max(-1, Math.min(1, amplitude));
    const alpha = 0.15 + Math.abs(clipped) * 0.85;

    if (clipped >= 0) {
      return `rgba(210, 54, 45, ${alpha.toFixed(3)})`;
    }

    return `rgba(43, 77, 170, ${alpha.toFixed(3)})`;
  }

  function scaleSectionTime(timeMs: number): number {
    if (!model) {
      return sectionTop;
    }
    const { start, end } = model.timeRangeMs;
    if (end <= start) {
      return sectionTop;
    }
    const clamped = Math.max(start, Math.min(end, timeMs));
    return sectionTop + ((clamped - start) / (end - start)) * sectionPlotHeight;
  }

  function scaleSectionTrace(section: WellTieSectionPanel, traceIndex: number): number {
    if (section.traceCount <= 0) {
      return sectionLeft;
    }
    return sectionLeft + ((traceIndex + 0.5) / section.traceCount) * sectionPlotWidth;
  }

  function formatSignedOffset(offsetM: number | undefined): string {
    if (offsetM === undefined || !Number.isFinite(offsetM)) {
      return "0 m";
    }
    return `${offsetM >= 0 ? "+" : ""}${offsetM.toFixed(0)} m`;
  }

  function hasDistinctSectionMatch(section: WellTieSectionPanel): boolean {
    return (
      section.matchTraceIndex !== undefined &&
      section.wellTraceIndex !== undefined &&
      section.matchTraceIndex !== section.wellTraceIndex
    );
  }

  function waveletRange(wavelet: WellTieWavelet): { min: number; max: number } {
    if (wavelet.amplitudeRange) {
      return wavelet.amplitudeRange;
    }

    let min = Number.POSITIVE_INFINITY;
    let max = Number.NEGATIVE_INFINITY;
    for (const amplitude of wavelet.amplitudes) {
      min = Math.min(min, amplitude);
      max = Math.max(max, amplitude);
    }

    if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) {
      return { min: -1, max: 1 };
    }

    return { min, max };
  }

  function scaleWaveletAmplitude(wavelet: WellTieWavelet, amplitude: number): number {
    const range = waveletRange(wavelet);
    if (range.max <= range.min) {
      return waveletLeft + waveletPlotWidth / 2;
    }

    const normalized = (amplitude - range.min) / (range.max - range.min);
    return waveletLeft + normalized * waveletPlotWidth;
  }

  function scaleWaveletTime(wavelet: WellTieWavelet, timeMs: number): number {
    const start = wavelet.timesMs[0] ?? -80;
    const end = wavelet.timesMs[wavelet.timesMs.length - 1] ?? 80;
    if (end <= start) {
      return waveletTop;
    }

    const clamped = Math.max(start, Math.min(end, timeMs));
    return waveletTop + ((clamped - start) / (end - start)) * waveletPlotHeight;
  }

  function waveletLinePath(wavelet: WellTieWavelet): string {
    const points: string[] = [];

    for (let index = 0; index < wavelet.timesMs.length; index += 1) {
      const x = scaleWaveletAmplitude(wavelet, wavelet.amplitudes[index] ?? 0);
      const y = scaleWaveletTime(wavelet, wavelet.timesMs[index] ?? 0);
      points.push(`${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`);
    }

    return points.join(" ");
  }

  function waveletFillPath(wavelet: WellTieWavelet, polarity: "positive" | "negative"): string {
    const centerX = scaleWaveletAmplitude(wavelet, 0);
    const forward: string[] = [];
    const reverse: string[] = [];

    for (let index = 0; index < wavelet.timesMs.length; index += 1) {
      const amplitude = wavelet.amplitudes[index] ?? 0;
      const component = polarity === "positive" ? Math.max(0, amplitude) : Math.min(0, amplitude);
      const x = scaleWaveletAmplitude(wavelet, component);
      const y = scaleWaveletTime(wavelet, wavelet.timesMs[index] ?? 0);
      forward.push(`${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`);
    }

    for (let index = wavelet.timesMs.length - 1; index >= 0; index -= 1) {
      const y = scaleWaveletTime(wavelet, wavelet.timesMs[index] ?? 0);
      reverse.push(`L${centerX.toFixed(2)},${y.toFixed(2)}`);
    }

    return `${forward.join(" ")} ${reverse.join(" ")} Z`;
  }
</script>

<div class="ophiolite-charts-well-tie">
  {#if model}
    <div class="chart-header">
      <div class="chart-title-block">
        <h3>{model.name}</h3>
        <p>
          {model.timeRangeMs.start.toFixed(0)}-{model.timeRangeMs.end.toFixed(0)} {model.timeRangeMs.unit ?? "ms"}
          {#if model.depthRangeM}
            <span class="dot">.</span>
            {model.depthRangeM.start.toFixed(0)}-{model.depthRangeM.end.toFixed(0)} m
          {/if}
        </p>
      </div>

      {#if model.metrics?.length}
        <div class="metric-strip" aria-label="Tie diagnostics">
          {#each model.metrics as metric (metric.id)}
            <div class={`metric metric-${metric.emphasis ?? "neutral"}`}>
              <span>{metric.label}</span>
              <strong>{metric.value}</strong>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="chart-layout">
      <div class="tie-stage">
        <svg viewBox={`0 0 ${tieSvgWidth} ${tieHeight}`} role="img" aria-label={ariaLabel}>
          <g class="grid">
            {#each timeTicks as tick (tick.value)}
              <line x1={tieLeft} y1={scaleTime(tick.value)} x2={tieLeft + tiePlotWidth} y2={scaleTime(tick.value)} />
              <text class="time-tick" x={tieLeft - 10} y={scaleTime(tick.value) + 4}>{tick.label}</text>
            {/each}
          </g>

          {#each trackFrames as frame (frame.track.id)}
            <text class="track-title" x={frame.centerX} y="24">{frame.track.label}</text>
            <rect class="track-frame" x={frame.x} y={tieTop} width={frame.width} height={tiePlotHeight} rx="6" />

            {#if isCurveTrack(frame.track)}
              <path class="curve-line" d={curvePath(frame.track, frame)} stroke={frame.track.color} />
            {:else if isWiggleTrack(frame.track)}
              <line class="track-centerline" x1={frame.centerX} y1={tieTop} x2={frame.centerX} y2={tieTop + tiePlotHeight} />
              <path d={wiggleFillPath(frame.track, frame, "positive")} fill={frame.track.positiveFill ?? "rgba(210, 54, 45, 0.78)"} />
              <path d={wiggleFillPath(frame.track, frame, "negative")} fill={frame.track.negativeFill ?? "rgba(43, 77, 170, 0.76)"} />
              <path class="wiggle-line" d={wiggleLinePath(frame.track, frame)} stroke={frame.track.lineColor ?? "#213140"} />
            {/if}
          {/each}

          {#each model.markers ?? [] as marker (marker.id)}
            <line
              class="marker-line"
              x1={tieLeft}
              y1={scaleTime(marker.timeMs)}
              x2={tieLeft + tiePlotWidth}
              y2={scaleTime(marker.timeMs)}
              stroke={markerLineColor(marker.color)}
            />
            <text class="marker-label" x={tieLeft + tiePlotWidth - 8} y={scaleTime(marker.timeMs) - 6} fill={markerLineColor(marker.color)}>
              {marker.label}
            </text>
          {/each}

          <text class="axis-label" x="20" y={tieTop + tiePlotHeight / 2} transform={`rotate(-90 20 ${tieTop + tiePlotHeight / 2})`}>
            Time ({model.timeRangeMs.unit ?? "ms"})
          </text>
        </svg>
      </div>

      <div class="aux-stage">
        {#if model.section}
          <div class="section-block">
            <svg viewBox={`0 0 ${sectionWidth} ${sectionHeight}`} role="img" aria-label="Local seismic section">
              <text class="aux-title" x={sectionLeft} y="22">{model.section.label}</text>
              <text class="section-summary" x={sectionWidth - sectionRight} y="22">
                Best {formatSignedOffset(model.section.matchOffsetM)}
              </text>
              <rect class="aux-frame" x={sectionLeft} y={sectionTop} width={sectionPlotWidth} height={sectionPlotHeight} rx="6" />

              {#each sectionCells as cell, index (`${cell.x}-${cell.y}-${index}`)}
                <rect x={cell.x} y={cell.y} width={cell.width} height={cell.height} fill={cell.color} />
              {/each}

              {#if model.section.wellTraceIndex !== undefined}
                <line
                  class="section-well-line"
                  x1={scaleSectionTrace(model.section, model.section.wellTraceIndex)}
                  y1={sectionTop}
                  x2={scaleSectionTrace(model.section, model.section.wellTraceIndex)}
                  y2={sectionTop + sectionPlotHeight}
                />
                <text
                  class="section-trace-label well-label"
                  x={scaleSectionTrace(model.section, model.section.wellTraceIndex)}
                  y={sectionTop + 16}
                >
                  {model.section.wellLabel ?? "Well"}
                </text>
              {/if}

              {#if model.section.matchTraceIndex !== undefined}
                <line
                  class={`section-match-line ${hasDistinctSectionMatch(model.section) ? "" : "section-match-shared"}`}
                  x1={scaleSectionTrace(model.section, model.section.matchTraceIndex)}
                  y1={sectionTop}
                  x2={scaleSectionTrace(model.section, model.section.matchTraceIndex)}
                  y2={sectionTop + sectionPlotHeight}
                />
                <text
                  class={`section-trace-label match-label ${hasDistinctSectionMatch(model.section) ? "" : "shared-label"}`}
                  x={scaleSectionTrace(model.section, model.section.matchTraceIndex)}
                  y={hasDistinctSectionMatch(model.section) ? sectionTop + 32 : sectionTop + 16}
                >
                  {model.section.matchLabel ?? "Best Match"}
                </text>
              {/if}

              {#each timeTicks as tick (tick.value)}
                <line
                  x1={sectionLeft}
                  y1={scaleSectionTime(tick.value)}
                  x2={sectionLeft + sectionPlotWidth}
                  y2={scaleSectionTime(tick.value)}
                  class="aux-grid"
                />
              {/each}

              <text class="axis-label aux-y" x="12" y={sectionTop + sectionPlotHeight / 2} transform={`rotate(-90 12 ${sectionTop + sectionPlotHeight / 2})`}>
                Time ({model.timeRangeMs.unit ?? "ms"})
              </text>
              <text class="section-offset-tick" x={sectionLeft} y={sectionHeight - 20}>
                {model.section.traceOffsetsM[0]?.toFixed(0) ?? "0"} m
              </text>
              <text class="section-offset-tick section-offset-center" x={sectionLeft + sectionPlotWidth / 2} y={sectionHeight - 20}>
                Well 0 m
              </text>
              <text class="section-offset-tick section-offset-right" x={sectionLeft + sectionPlotWidth} y={sectionHeight - 20}>
                {model.section.traceOffsetsM[model.section.traceOffsetsM.length - 1]?.toFixed(0) ?? "0"} m
              </text>
              <text class="aux-axis-label" x={sectionLeft + sectionPlotWidth / 2} y={sectionHeight - 6}>Trace Window Around Well</text>
            </svg>
          </div>
        {/if}

        {#if model.wavelet}
          <div class="wavelet-block">
            <svg viewBox={`0 0 ${waveletWidth} ${waveletHeight}`} role="img" aria-label="Extracted wavelet">
              <text class="aux-title" x={waveletLeft} y="22">{model.wavelet.label}</text>
              <text class={`wavelet-state ${model.wavelet.state ?? "provisional"}`} x={waveletWidth - waveletRight} y="22">
                {model.wavelet.state === "extracted" ? "Extracted" : "Provisional"}
              </text>
              {#if model.wavelet.detail}
                <text class="wavelet-detail" x={waveletLeft} y="34">{model.wavelet.detail}</text>
              {/if}
              <rect
                class={`aux-frame wavelet-frame ${model.wavelet.state ?? "provisional"}`}
                x={waveletLeft}
                y={waveletTop}
                width={waveletPlotWidth}
                height={waveletPlotHeight}
                rx="6"
              />

              {#each waveletTicks as tick (tick.value)}
                <line
                  class="aux-grid"
                  x1={waveletLeft}
                  y1={scaleWaveletTime(model.wavelet, tick.value)}
                  x2={waveletLeft + waveletPlotWidth}
                  y2={scaleWaveletTime(model.wavelet, tick.value)}
                />
                <text class="wavelet-tick" x={waveletLeft - 8} y={scaleWaveletTime(model.wavelet, tick.value) + 4}>{tick.label}</text>
              {/each}

              <line
                class="track-centerline"
                x1={scaleWaveletAmplitude(model.wavelet, 0)}
                y1={waveletTop}
                x2={scaleWaveletAmplitude(model.wavelet, 0)}
                y2={waveletTop + waveletPlotHeight}
              />
              <path d={waveletFillPath(model.wavelet, "positive")} fill="rgba(221, 70, 61, 0.84)" />
              <path d={waveletFillPath(model.wavelet, "negative")} fill="rgba(52, 89, 178, 0.82)" />
              <path class="wiggle-line" d={waveletLinePath(model.wavelet)} stroke="#213140" />

              <text class="axis-label aux-y" x="12" y={waveletTop + waveletPlotHeight / 2} transform={`rotate(-90 12 ${waveletTop + waveletPlotHeight / 2})`}>
                Time (ms)
              </text>
              <text class="aux-axis-label" x={waveletLeft + waveletPlotWidth / 2} y={waveletHeight - 6}>Amplitude</text>
            </svg>
          </div>
        {/if}
      </div>
    </div>

    {#if model.notes?.length}
      <div class="notes-row">
        {#each model.notes as note (`${model.id}-${note}`)}
          <p>{note}</p>
        {/each}
      </div>
    {/if}
  {:else}
    <div class="chart-empty">
      <p>{emptyMessage}</p>
    </div>
  {/if}
</div>

<style>
  .ophiolite-charts-well-tie {
    display: grid;
    gap: 14px;
    width: 100%;
    min-width: 0;
  }

  .chart-header {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .chart-title-block h3 {
    margin: 0;
    color: #243a4a;
    font-size: 18px;
    font-weight: 700;
  }

  .chart-title-block p {
    margin: 4px 0 0;
    color: #5d7180;
    font-size: 13px;
  }

  .dot {
    margin: 0 6px;
  }

  .metric-strip {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .metric {
    min-width: 88px;
    padding: 8px 10px;
    border: 1px solid rgba(162, 183, 196, 0.9);
    border-radius: 8px;
    background: #f7fafc;
    display: grid;
    gap: 2px;
  }

  .metric span {
    color: #5d7180;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .metric strong {
    color: #243a4a;
    font-size: 14px;
    font-weight: 700;
  }

  .metric-good {
    background: #eef8f4;
  }

  .metric-warn {
    background: #fcf4ec;
  }

  .chart-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.65fr) minmax(240px, 0.9fr);
    gap: 14px;
  }

  .tie-stage,
  .section-block,
  .wavelet-block {
    border: 1px solid rgba(162, 183, 196, 0.88);
    border-radius: 8px;
    background: #ffffff;
    overflow: hidden;
  }

  .tie-stage svg,
  .section-block svg,
  .wavelet-block svg {
    display: block;
    width: 100%;
    height: auto;
  }

  .aux-stage {
    display: grid;
    align-content: start;
    gap: 14px;
  }

  .grid line,
  .aux-grid {
    stroke: rgba(118, 137, 151, 0.28);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }

  .track-frame,
  .aux-frame {
    fill: #ffffff;
    stroke: rgba(160, 186, 205, 0.92);
    stroke-width: 1.2;
  }

  .wavelet-frame.extracted {
    fill: #f5fcf8;
    stroke: rgba(111, 190, 152, 0.92);
  }

  .wavelet-frame.provisional {
    fill: #fff8f0;
    stroke: rgba(220, 160, 103, 0.92);
  }

  .track-title,
  .aux-title {
    fill: #243a4a;
    font-size: 12px;
    font-weight: 700;
    text-anchor: middle;
  }

  .aux-title {
    text-anchor: start;
  }

  .section-summary,
  .wavelet-state {
    font-size: 10px;
    font-weight: 700;
    text-anchor: end;
  }

  .section-summary {
    fill: #5b6f7f;
  }

  .wavelet-state.extracted {
    fill: #1d7b55;
  }

  .wavelet-state.provisional {
    fill: #b85f16;
  }

  .wavelet-detail {
    fill: #6f8290;
    font-size: 9px;
    font-weight: 600;
  }

  .time-tick,
  .wavelet-tick {
    fill: #647785;
    font-size: 10px;
    font-weight: 600;
    text-anchor: end;
  }

  .curve-line,
  .wiggle-line {
    fill: none;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }

  .track-centerline,
  .section-well-line {
    stroke: rgba(79, 103, 121, 0.52);
    stroke-width: 1.1;
    stroke-dasharray: 4 4;
  }

  .section-match-line {
    stroke: rgba(208, 106, 36, 0.92);
    stroke-width: 1.6;
    stroke-dasharray: 6 4;
  }

  .section-match-shared {
    stroke: rgba(135, 86, 27, 0.9);
    stroke-width: 1.3;
    stroke-dasharray: 2 4;
  }

  .section-trace-label {
    font-size: 9px;
    font-weight: 700;
    text-anchor: middle;
  }

  .well-label {
    fill: #4a5f6f;
  }

  .match-label {
    fill: #c66324;
  }

  .shared-label {
    fill: #8f6122;
  }

  .marker-line {
    stroke-width: 1.2;
    stroke-dasharray: 7 4;
  }

  .marker-label {
    font-size: 10px;
    font-weight: 700;
    text-anchor: end;
  }

  .axis-label,
  .aux-axis-label {
    fill: #35505f;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    text-anchor: middle;
  }

  .section-offset-tick {
    fill: #647785;
    font-size: 9px;
    font-weight: 600;
    text-anchor: start;
  }

  .section-offset-center {
    text-anchor: middle;
  }

  .section-offset-right {
    text-anchor: end;
  }

  .notes-row {
    display: grid;
    gap: 4px;
  }

  .notes-row p {
    margin: 0;
    color: #5d7180;
    font-size: 12px;
    line-height: 1.4;
  }

  .chart-empty {
    min-height: 260px;
    display: grid;
    place-items: center;
    border: 1px dashed rgba(155, 176, 191, 0.92);
    border-radius: 8px;
    background: #f8fbfd;
    padding: 24px;
  }

  .chart-empty p {
    margin: 0;
    max-width: 42ch;
    text-align: center;
    color: #58707f;
    line-height: 1.5;
  }

  @media (max-width: 1100px) {
    .chart-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
