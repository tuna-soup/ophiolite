import type {
  CorrelationMarkerLink,
  InteractionState,
  WellCorrelationProbe,
  WellCorrelationViewport
} from "@ophiolite/charts-data-models";
import type { NormalizedWellPanelModel } from "@ophiolite/charts-core";

export interface TopPreview {
  wellId: string;
  topId: string;
  nativeDepth: number;
  panelDepth: number;
}

export interface WellCorrelationViewState {
  panel: NormalizedWellPanelModel | null;
  viewport: WellCorrelationViewport | null;
  probe: WellCorrelationProbe | null;
  interactions: InteractionState;
  activeMarkerName: string | null;
  activeMarkerColor: string;
  correlationLines: CorrelationMarkerLink[];
  previewCorrelationLines: CorrelationMarkerLink[] | null;
  previewTop: TopPreview | null;
}

export interface WellCorrelationRenderFrame {
  state: WellCorrelationViewState;
}

export interface WellCorrelationRendererAdapter {
  mount(container: HTMLElement): void;
  render(frame: WellCorrelationRenderFrame): void;
  dispose(): void;
}
