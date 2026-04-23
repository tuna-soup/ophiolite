import type { SeismicSectionAnalysisSelection } from "@ophiolite/charts";
import type { SectionSpectrumSelection, SectionViewport, SectionView } from "@traceboost/seis-contracts";
import type { TransportSectionView } from "./bridge";

export type DisplaySectionView = SectionView | TransportSectionView;

export function selectionFromMode(
  mode: "whole-section" | "viewport",
  viewport: SectionViewport | null
): SeismicSectionAnalysisSelection | null {
  if (mode === "viewport") {
    if (!viewport) {
      return null;
    }
    return {
      kind: "viewport",
      viewport: { ...viewport }
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
      return `${baseKey}:viewport:${selection.viewport.trace_start}:${selection.viewport.trace_end}:${selection.viewport.sample_start}:${selection.viewport.sample_end}`;
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
      const traces = Math.max(0, selection.viewport.trace_end - selection.viewport.trace_start);
      const samples = Math.max(0, selection.viewport.sample_end - selection.viewport.sample_start);
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
          trace_start: selection.viewport.trace_start,
          trace_end: selection.viewport.trace_end,
          sample_start: selection.viewport.sample_start,
          sample_end: selection.viewport.sample_end
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
