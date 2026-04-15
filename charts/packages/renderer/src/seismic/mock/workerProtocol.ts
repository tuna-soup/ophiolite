import type { DisplayTransform, OverlayPayload, SectionPayload, SectionViewport } from "@ophiolite/charts-data-models";
import type { PlotRect } from "./wiggleGeometry";

export interface WorkerSectionPayload {
  axis: SectionPayload["axis"];
  coordinate: SectionPayload["coordinate"];
  dimensions: SectionPayload["dimensions"];
  amplitudesBuffer: ArrayBuffer;
  horizontalAxisBuffer: ArrayBuffer;
  inlineAxisBuffer?: ArrayBuffer;
  xlineAxisBuffer?: ArrayBuffer;
  sampleAxisBuffer: ArrayBuffer;
  units?: SectionPayload["units"];
  metadata?: SectionPayload["metadata"];
}

export interface WorkerOverlayPayload {
  kind: OverlayPayload["kind"];
  width: number;
  height: number;
  valuesBuffer: ArrayBuffer;
  opacity?: number;
}

export interface WorkerBaseStatePayload {
  width: number;
  height: number;
  plotRect: PlotRect;
}

export type WorkerIncomingMessage =
  | {
      type: "init";
      canvas: OffscreenCanvas;
      state: WorkerBaseStatePayload;
    }
  | {
      type: "resize";
      state: WorkerBaseStatePayload;
    }
  | {
      type: "setSection";
      section: WorkerSectionPayload;
    }
  | {
      type: "setViewport";
      viewport: SectionViewport;
    }
  | {
      type: "setDisplayTransform";
      displayTransform: DisplayTransform;
    }
  | {
      type: "setOverlay";
      overlay: WorkerOverlayPayload | null;
    }
  | {
      type: "dispose";
    };

export type WorkerOutgoingMessage =
  | {
      type: "ready";
      offscreen: boolean;
      webgl2: boolean;
    }
  | {
      type: "frameRendered";
    }
  | {
      type: "error";
      message: string;
    };

export function cloneSectionForWorker(section: SectionPayload): {
  payload: WorkerSectionPayload;
  transfer: Transferable[];
} {
  const amplitudes = new Float32Array(section.amplitudes);
  const horizontalAxis = new Float64Array(section.horizontalAxis);
  const inlineAxis = section.inlineAxis ? new Float64Array(section.inlineAxis) : null;
  const xlineAxis = section.xlineAxis ? new Float64Array(section.xlineAxis) : null;
  const sampleAxis = new Float32Array(section.sampleAxis);
  const transfer: Transferable[] = [amplitudes.buffer, horizontalAxis.buffer, sampleAxis.buffer];

  if (inlineAxis) {
    transfer.push(inlineAxis.buffer);
  }
  if (xlineAxis) {
    transfer.push(xlineAxis.buffer);
  }

  return {
    payload: {
      axis: section.axis,
      coordinate: { ...section.coordinate },
      dimensions: { ...section.dimensions },
      amplitudesBuffer: amplitudes.buffer,
      horizontalAxisBuffer: horizontalAxis.buffer,
      inlineAxisBuffer: inlineAxis?.buffer,
      xlineAxisBuffer: xlineAxis?.buffer,
      sampleAxisBuffer: sampleAxis.buffer,
      units: section.units ? { ...section.units } : undefined,
      metadata: section.metadata
        ? {
            ...section.metadata,
            notes: section.metadata.notes ? [...section.metadata.notes] : undefined
          }
        : undefined
    },
    transfer
  };
}

export function cloneOverlayForWorker(overlay: OverlayPayload | null): {
  payload: WorkerOverlayPayload | null;
  transfer: Transferable[];
} {
  if (!overlay) {
    return {
      payload: null,
      transfer: []
    };
  }

  const values = new Uint8Array(overlay.values);
  return {
    payload: {
      kind: overlay.kind,
      width: overlay.width,
      height: overlay.height,
      valuesBuffer: values.buffer,
      opacity: overlay.opacity
    },
    transfer: [values.buffer]
  };
}

export function canUseOffscreenWorkerRenderer(canvas: HTMLCanvasElement): boolean {
  return (
    typeof Worker !== "undefined" &&
    typeof OffscreenCanvas !== "undefined" &&
    typeof canvas.transferControlToOffscreen === "function"
  );
}
