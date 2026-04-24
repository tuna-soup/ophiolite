import type { SeismicSectionAnalysisSelection, SeismicViewport } from "@ophiolite/charts";
import type { SectionSpectrumSelection, SectionViewport, SectionView } from "@traceboost/seis-contracts";
import type { TransportSectionView } from "./bridge";

export type DisplaySectionView = SectionView | TransportSectionView;

export function selectionFromMode(
  mode: "whole-section" | "viewport",
  viewport: SectionViewport | SeismicViewport | null
): SeismicSectionAnalysisSelection | null {
  if (mode === "viewport") {
    if (!viewport) {
      return null;
    }
    return {
      kind: "viewport",
      viewport: {
        traceStart:
          (viewport as { traceStart?: number; trace_start?: number }).traceStart ??
          (viewport as { trace_start?: number }).trace_start ??
          0,
        traceEnd:
          (viewport as { traceEnd?: number; trace_end?: number }).traceEnd ??
          (viewport as { trace_end?: number }).trace_end ??
          0,
        sampleStart:
          (viewport as { sampleStart?: number; sample_start?: number }).sampleStart ??
          (viewport as { sample_start?: number }).sample_start ??
          0,
        sampleEnd:
          (viewport as { sampleEnd?: number; sample_end?: number }).sampleEnd ??
          (viewport as { sample_end?: number }).sample_end ??
          0
      }
    };
  }

  return {
    kind: "whole-section"
  };
}

export function buildAnalysisSelectionKey(
  baseKey: string | null,
  selection: SeismicSectionAnalysisSelection | null
): string | null {
  if (!baseKey || !selection) {
    return null;
  }

  switch (selection.kind) {
    case "whole-section":
      return `${baseKey}:whole-section`;
    case "viewport":
      return `${baseKey}:viewport:${selection.viewport.traceStart}:${selection.viewport.traceEnd}:${selection.viewport.sampleStart}:${selection.viewport.sampleEnd}`;
    case "rectangle":
      return `${baseKey}:rectangle:${selection.rectangle.left}:${selection.rectangle.top}:${selection.rectangle.right}:${selection.rectangle.bottom}`;
    default:
      return baseKey;
  }
}

export function buildAnalysisSelectionSummary(
  section: DisplaySectionView | null,
  selection: SeismicSectionAnalysisSelection | null
): string {
  if (!section || !selection) {
    return "Load a section to inspect analysis results.";
  }

  switch (selection.kind) {
    case "whole-section":
      return `Displayed ${section.axis} section ${section.coordinate.index} · ${section.traces} traces x ${section.samples} samples`;
    case "viewport": {
      const traces = Math.max(0, selection.viewport.traceEnd - selection.viewport.traceStart);
      const samples = Math.max(0, selection.viewport.sampleEnd - selection.viewport.sampleStart);
      return `Viewport of ${section.axis} section ${section.coordinate.index} · ${traces} traces x ${samples} samples`;
    }
    case "rectangle": {
      const traces = Math.max(0, selection.rectangle.right - selection.rectangle.left);
      const samples = Math.max(0, selection.rectangle.bottom - selection.rectangle.top);
      return `Window in ${section.axis} section ${section.coordinate.index} · ${traces} traces x ${samples} samples`;
    }
    default:
      return `Displayed ${section.axis} section ${section.coordinate.index}`;
  }
}

export function toSpectrumSelection(selection: SeismicSectionAnalysisSelection): SectionSpectrumSelection {
  switch (selection.kind) {
    case "whole-section":
      return "whole_section";
    case "viewport":
      return {
        rect_window: {
          trace_start: selection.viewport.traceStart,
          trace_end: selection.viewport.traceEnd,
          sample_start: selection.viewport.sampleStart,
          sample_end: selection.viewport.sampleEnd
        }
      };
    case "rectangle":
      return {
        rect_window: {
          trace_start: selection.rectangle.left,
          trace_end: selection.rectangle.right,
          sample_start: selection.rectangle.top,
          sample_end: selection.rectangle.bottom
        }
      };
  }
}
