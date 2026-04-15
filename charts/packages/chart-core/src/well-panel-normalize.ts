import type {
  CurveFillStyle,
  CurveSeries,
  DepthDatum,
  DepthDomain,
  DepthMappingSample,
  PointObservationSample,
  SectionPayload,
  TopOverlayStyle,
  TrackAxis,
  WellCorrelationPanelModel,
  WellPanelCurveData,
  WellPanelDataCatalog,
  WellPanelModel,
  WellPanelPointObservationData,
  WellPanelSeismicSectionData,
  WellPanelSeismicTraceData,
  WellPanelTop,
  WellPanelTopSetData
} from "@ophiolite/charts-data-models";
import type { DisplayTransform, RenderMode } from "@ophiolite/charts-data-models";

export interface NormalizedWellPanelModel {
  id: string;
  name: string;
  depthDomain: DepthDomain;
  wells: NormalizedWellColumn[];
  background?: string;
}

export interface NormalizedWellColumn {
  id: string;
  name: string;
  nativeDepthDatum: DepthDatum;
  panelDepthMapping: DepthMappingSample[];
  tracks: NormalizedTrack[];
  headerNote?: string;
}

export type NormalizedTrack =
  | NormalizedReferenceTrack
  | NormalizedScalarTrack
  | NormalizedSeismicTraceTrack
  | NormalizedSeismicSectionTrack;

export interface NormalizedBaseTrack {
  kind: "reference" | "scalar" | "seismic-trace" | "seismic-section";
  id: string;
  title: string;
  width: number;
}

export interface NormalizedReferenceTrack extends NormalizedBaseTrack {
  kind: "reference";
  topOverlays: NormalizedTopOverlayLayer[];
}

export interface NormalizedScalarTrack extends NormalizedBaseTrack {
  kind: "scalar";
  xAxis: TrackAxis;
  layers: Array<NormalizedCurveLayer | NormalizedPointLayer | NormalizedCompositionLayer | NormalizedTopOverlayLayer>;
  betweenCurveFills?: Array<{
    leftSeriesId: string;
    rightSeriesId: string;
    color: string;
    fillWhen: "rightOf";
  }>;
}

export interface NormalizedSeismicTraceTrack extends NormalizedBaseTrack {
  kind: "seismic-trace";
  layers: Array<NormalizedSeismicTraceLayer | NormalizedTopOverlayLayer>;
}

export interface NormalizedSeismicSectionTrack extends NormalizedBaseTrack {
  kind: "seismic-section";
  layers: Array<NormalizedSeismicSectionLayer | NormalizedTopOverlayLayer>;
}

export interface NormalizedCurveLayer {
  kind: "curve";
  id: string;
  name: string;
  series: CurveSeries;
}

export interface NormalizedPointLayer {
  kind: "point-observation";
  id: string;
  name: string;
  family: string;
  axis: TrackAxis;
  unit?: string;
  points: PointObservationSample[];
  style: {
    shape: "circle" | "square" | "diamond" | "triangle" | "cross" | "x";
    size: number;
    fillColor: string;
    strokeColor?: string;
    strokeWidth?: number;
  };
}

export interface NormalizedCompositionLayer {
  kind: "composition";
  id: string;
  name: string;
  nativeDepths: Float32Array;
  components: Array<{
    id: string;
    name: string;
    color: string;
    values: Float32Array;
  }>;
}

export interface NormalizedTopOverlayLayer {
  kind: "top-overlay";
  id: string;
  name: string;
  tops: WellPanelTop[];
  style: TopOverlayStyle;
}

export interface NormalizedSeismicTraceLayer {
  kind: "seismic-trace";
  id: string;
  name: string;
  nativeDepths: Float32Array;
  panelDepths?: Float32Array;
  normalization: "shared-domain" | "per-trace";
  traces: Array<{
    id: string;
    name: string;
    amplitudes: Float32Array;
    style: {
      positiveFill: string;
      negativeFill?: string;
      lineColor?: string;
      lineWidth?: number;
      fillOpacity?: number;
    };
  }>;
}

export interface NormalizedSeismicSectionLayer {
  kind: "seismic-section";
  id: string;
  name: string;
  section: SectionPayload;
  panelDepths: Float32Array;
  nativeDepths?: Float32Array;
  wellTraceIndex?: number;
  style: Partial<DisplayTransform> & {
    renderMode: RenderMode;
  };
}

export function normalizeWellPanelModel(
  panel: WellCorrelationPanelModel | WellPanelModel | null
): NormalizedWellPanelModel | null {
  if (!panel) {
    return null;
  }
  const firstWell = panel.wells[0];
  if (firstWell && "data" in firstWell) {
    return normalizeLayeredWellPanel(panel as WellPanelModel);
  }
  return normalizeLegacyWellPanel(panel as WellCorrelationPanelModel);
}

function normalizeLegacyWellPanel(panel: WellCorrelationPanelModel): NormalizedWellPanelModel {
  return {
    id: panel.id,
    name: panel.name,
    depthDomain: { ...panel.depthDomain },
    background: panel.background,
    wells: panel.wells.map((well) => ({
      id: well.id,
      name: well.name,
      nativeDepthDatum: well.nativeDepthDatum,
      panelDepthMapping: well.panelDepthMapping.map((sample) => ({ ...sample })),
      headerNote: well.headerNote,
      tracks: well.tracks.map((track) => {
        switch (track.kind) {
          case "reference":
            return {
              kind: "reference",
              id: track.id,
              title: track.title,
              width: track.width,
              topOverlays: []
            } satisfies NormalizedReferenceTrack;
          case "curve":
            return {
              kind: "scalar",
              id: track.id,
              title: track.title,
              width: track.width,
              xAxis: { ...track.xAxis },
              layers: track.series.map((series) => ({
                kind: "curve",
                id: series.id,
                name: series.name,
                series: cloneSeries(series)
              }))
            } satisfies NormalizedScalarTrack;
          case "multi-curve":
            return {
              kind: "scalar",
              id: track.id,
              title: track.title,
              width: track.width,
              xAxis: { ...track.xAxis },
              betweenCurveFills: track.crossoverFill ? [{ ...track.crossoverFill }] : undefined,
              layers: track.series.map((series) => ({
                kind: "curve",
                id: series.id,
                name: series.name,
                series: cloneSeries(series)
              }))
            } satisfies NormalizedScalarTrack;
          case "filled-curve":
            return {
              kind: "scalar",
              id: track.id,
              title: track.title,
              width: track.width,
              xAxis: { ...track.xAxis },
              layers: track.series.map((series) => ({
                kind: "curve",
                id: series.id,
                name: series.name,
                series: {
                  ...cloneSeries(series),
                  fill: cloneCurveFill(track.fill)
                }
              }))
            } satisfies NormalizedScalarTrack;
          case "lithology":
            return {
              kind: "scalar",
              id: track.id,
              title: track.title,
              width: track.width,
              xAxis: { ...track.xAxis },
              layers: [
                {
                  kind: "composition",
                  id: track.id,
                  name: track.title,
                  nativeDepths: new Float32Array(track.nativeDepths),
                  components: track.components.map((component) => ({
                    ...component,
                    values: new Float32Array(component.values)
                  }))
                }
              ]
            } satisfies NormalizedScalarTrack;
          case "tops":
            return {
              kind: "reference",
              id: track.id,
              title: track.title,
              width: track.width,
              topOverlays: [
                {
                  kind: "top-overlay",
                  id: `${track.id}-tops`,
                  name: track.title,
                  tops: well.tops.map((top) => ({ ...top })),
                  style: {
                    color: "#b44d4d",
                    lineWidth: 1,
                    labelColor: "#5b2b2b",
                    showLabels: true,
                    editable: true
                  }
                }
              ]
            } satisfies NormalizedReferenceTrack;
        }
      })
    }))
  };
}

function normalizeLayeredWellPanel(panel: WellPanelModel): NormalizedWellPanelModel {
  return {
    id: panel.id,
    name: panel.name,
    depthDomain: { ...panel.depthDomain },
    background: panel.background,
    wells: panel.wells.map((well) => {
      const topSetLayers = new Map<string, NormalizedTopOverlayLayer>();
      const tracks = well.tracks.map((track) => {
        if (track.kind === "reference") {
          return {
            kind: "reference",
            id: track.id,
            title: track.title,
            width: track.width,
            topOverlays: track.layers?.flatMap((layer) => resolveTopOverlayLayer(layer.dataId, well.data, layer.id, layer.name, layer.style, topSetLayers)) ?? []
          } satisfies NormalizedReferenceTrack;
        }
        if (track.kind === "scalar") {
          const scalarLayers: NormalizedScalarTrack["layers"] = [];
          for (const layer of track.layers) {
            if (layer.kind === "curve") {
              const curve = lookupCurve(well.data, layer.dataId);
              if (!curve) {
                continue;
              }
              scalarLayers.push({
                kind: "curve",
                id: layer.id,
                name: layer.name ?? curve.name,
                series: curveDataToSeries(curve, track.xAxis, layer.style.color, layer.style.lineWidth, layer.style.lineDash, layer.style.fill)
              });
              continue;
            }
            if (layer.kind === "point-observation") {
              const points = lookupPointObservation(well.data, layer.dataId);
              if (!points) {
                continue;
              }
              scalarLayers.push({
                kind: "point-observation",
                id: layer.id,
                name: layer.name ?? points.name,
                family: points.family,
                axis: points.axis ?? track.xAxis,
                unit: points.unit,
                points: points.points.map((point) => ({ ...point })),
                style: { ...layer.style }
              });
              continue;
            }
            scalarLayers.push(...resolveTopOverlayLayer(layer.dataId, well.data, layer.id, layer.name, layer.style, topSetLayers));
          }
          return {
            kind: "scalar",
            id: track.id,
            title: track.title,
            width: track.width,
            xAxis: { ...track.xAxis },
            betweenCurveFills: track.layers
              .filter((layer) => layer.kind === "curve")
              .flatMap((layer) => {
                const curve = lookupCurve(well.data, layer.dataId);
                const fill = layer.style.fill;
                if (!curve || !fill || fill.mode !== "between-curves" || !fill.targetCurveId) {
                  return [];
                }
                return [{
                  leftSeriesId: layer.dataId,
                  rightSeriesId: fill.targetCurveId,
                  color: fill.color,
                  fillWhen: "rightOf" as const
                }];
              }),
            layers: scalarLayers
          } satisfies NormalizedScalarTrack;
        }
        if (track.kind === "seismic-trace") {
          const traceLayers: NormalizedSeismicTraceTrack["layers"] = [];
          for (const layer of track.layers) {
            if (layer.kind === "top-overlay") {
              traceLayers.push(...resolveTopOverlayLayer(layer.dataId, well.data, layer.id, layer.name, layer.style, topSetLayers));
              continue;
            }
            const traceData = lookupSeismicTrace(well.data, layer.dataId);
            if (!traceData) {
              continue;
            }
            const selectedTraces = layer.traceIds?.length
              ? traceData.traces.filter((trace) => layer.traceIds?.includes(trace.id))
              : traceData.traces;
            traceLayers.push({
              kind: "seismic-trace",
              id: layer.id,
              name: layer.name ?? traceData.name,
              nativeDepths: new Float32Array(traceData.nativeDepths),
              panelDepths: traceData.panelDepths ? new Float32Array(traceData.panelDepths) : undefined,
              normalization: layer.normalization ?? "shared-domain",
              traces: selectedTraces.map((trace, index) => ({
                id: trace.id,
                name: trace.name,
                amplitudes: new Float32Array(trace.amplitudes),
                style: {
                  positiveFill: layer.styleByTraceId?.[trace.id]?.positiveFill ?? defaultTraceColor(index),
                  negativeFill: layer.styleByTraceId?.[trace.id]?.negativeFill,
                  lineColor: layer.styleByTraceId?.[trace.id]?.lineColor,
                  lineWidth: layer.styleByTraceId?.[trace.id]?.lineWidth,
                  fillOpacity: layer.styleByTraceId?.[trace.id]?.fillOpacity
                }
              }))
            });
          }
          return {
            kind: "seismic-trace",
            id: track.id,
            title: track.title,
            width: track.width,
            layers: traceLayers
          } satisfies NormalizedSeismicTraceTrack;
        }
        const sectionLayers: NormalizedSeismicSectionTrack["layers"] = [];
        for (const layer of track.layers) {
          if (layer.kind === "top-overlay") {
            sectionLayers.push(...resolveTopOverlayLayer(layer.dataId, well.data, layer.id, layer.name, layer.style, topSetLayers));
            continue;
          }
          const sectionData = lookupSeismicSection(well.data, layer.dataId);
          if (!sectionData) {
            continue;
          }
          sectionLayers.push({
            kind: "seismic-section",
            id: layer.id,
            name: layer.name ?? sectionData.name,
            section: sectionData.section,
            panelDepths: new Float32Array(sectionData.panelDepths),
            nativeDepths: sectionData.nativeDepths ? new Float32Array(sectionData.nativeDepths) : undefined,
            wellTraceIndex: sectionData.wellTraceIndex,
            style: { ...layer.style.transform }
          });
        }
        return {
          kind: "seismic-section",
          id: track.id,
          title: track.title,
          width: track.width,
          layers: sectionLayers
        } satisfies NormalizedSeismicSectionTrack;
      });

      return {
        id: well.id,
        name: well.name,
        nativeDepthDatum: well.nativeDepthDatum,
        panelDepthMapping: well.panelDepthMapping.map((sample) => ({ ...sample })),
        headerNote: well.headerNote,
        tracks
      } satisfies NormalizedWellColumn;
    })
  };
}

function curveDataToSeries(
  curve: WellPanelCurveData,
  defaultAxis: TrackAxis,
  color: string,
  lineWidth: number | undefined,
  _lineDash: number[] | undefined,
  fill: {
    mode: "baseline" | "between-curves";
    color: string;
    gradientStops?: Array<{ offset: number; color: string }>;
    baseline?: number;
    targetCurveId?: string;
    fillWhen?: "leftOf" | "rightOf" | "greaterThan" | "lessThan";
    visible?: boolean;
    opacity?: number;
  } | undefined
): CurveSeries {
  return {
    id: curve.id,
    name: curve.name,
    color,
    values: new Float32Array(curve.values),
    nativeDepths: new Float32Array(curve.nativeDepths),
    lineWidth,
    axis: curve.axis ? { ...curve.axis } : { ...defaultAxis },
    fill: fill && fill.mode === "baseline"
      ? {
          direction: fill.fillWhen === "leftOf" ? "left" : "right",
          baseline: fill.baseline ?? defaultAxis.min,
          color: fill.color,
          gradientStops: fill.gradientStops?.map((stop) => ({ ...stop }))
        }
      : undefined
  };
}

function resolveTopOverlayLayer(
  dataId: string,
  catalog: WellPanelDataCatalog,
  layerId: string,
  layerName: string | undefined,
  style: TopOverlayStyle,
  cache: Map<string, NormalizedTopOverlayLayer>
): NormalizedTopOverlayLayer[] {
  const data = lookupTopSet(catalog, dataId);
  if (!data) {
    return [];
  }
  const existing = cache.get(layerId);
  if (existing) {
    return [existing];
  }
  const layer: NormalizedTopOverlayLayer = {
    kind: "top-overlay",
    id: layerId,
    name: layerName ?? data.name,
    tops: data.tops.map((top) => ({ ...top })),
    style: { ...style }
  };
  cache.set(layerId, layer);
  return [layer];
}

function lookupCurve(data: WellPanelDataCatalog, id: string): WellPanelCurveData | undefined {
  return data.curves?.find((item) => item.id === id);
}

function lookupPointObservation(data: WellPanelDataCatalog, id: string): WellPanelPointObservationData | undefined {
  return data.pointObservations?.find((item) => item.id === id);
}

function lookupTopSet(data: WellPanelDataCatalog, id: string): WellPanelTopSetData | undefined {
  return data.topSets?.find((item) => item.id === id);
}

function lookupSeismicTrace(data: WellPanelDataCatalog, id: string): WellPanelSeismicTraceData | undefined {
  return data.seismicTraces?.find((item) => item.id === id);
}

function lookupSeismicSection(data: WellPanelDataCatalog, id: string): WellPanelSeismicSectionData | undefined {
  return data.seismicSections?.find((item) => item.id === id);
}

function cloneSeries(series: CurveSeries): CurveSeries {
  return {
    ...series,
    axis: series.axis ? { ...series.axis } : undefined,
    fill: series.fill ? cloneCurveFill(series.fill) : undefined,
    values: new Float32Array(series.values),
    nativeDepths: new Float32Array(series.nativeDepths)
  };
}

function cloneCurveFill(fill: CurveFillStyle): CurveFillStyle {
  return {
    ...fill,
    gradientStops: fill.gradientStops?.map((stop) => ({ ...stop }))
  };
}

function defaultTraceColor(index: number): string {
  const palette = ["#db2f2f", "#111111", "#2a66b8", "#c17a1e", "#11825d"];
  return palette[index % palette.length]!;
}
