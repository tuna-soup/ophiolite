import type { DepthMappingSample, WellCorrelationPanelModel } from "../../packages/data-models/src/well-correlation";

export function createDepthMappingSamples(): DepthMappingSample[] {
  return [
    { nativeDepth: 1000, panelDepth: 0 },
    { nativeDepth: 1100, panelDepth: 80 },
    { nativeDepth: 1250, panelDepth: 200 }
  ];
}

export function createWellCorrelationPanelModel(): WellCorrelationPanelModel {
  return {
    id: "fixture-panel",
    name: "Fixture Well Correlation",
    depthDomain: {
      start: 1000,
      end: 1250,
      unit: "m",
      label: "MD"
    },
    background: "#f8fbfd",
    wells: [
      {
        id: "well-a",
        name: "Well A",
        nativeDepthDatum: "md",
        panelDepthMapping: createDepthMappingSamples(),
        tracks: [
          {
            kind: "reference",
            id: "well-a-reference",
            title: "Reference",
            width: 80
          },
          {
            kind: "multi-curve",
            id: "well-a-curves",
            title: "Logs",
            width: 120,
            xAxis: {
              min: 0,
              max: 160,
              label: "GR / RT",
              unit: "API"
            },
            series: [
              {
                id: "well-a-gr",
                name: "GR",
                color: "#1f2937",
                values: Float32Array.from([72, 84, 96]),
                nativeDepths: Float32Array.from([1000, 1100, 1250])
              },
              {
                id: "well-a-rt",
                name: "RT",
                color: "#0f766e",
                values: Float32Array.from([18, 24, 31]),
                nativeDepths: Float32Array.from([1000, 1100, 1250])
              }
            ]
          },
          {
            kind: "tops",
            id: "well-a-tops",
            title: "Tops",
            width: 70
          }
        ],
        tops: [
          {
            id: "well-a-top",
            name: "Reservoir",
            nativeDepth: 1100,
            color: "#b45309",
            source: "picked"
          }
        ]
      },
      {
        id: "well-b",
        name: "Well B",
        nativeDepthDatum: "md",
        panelDepthMapping: createDepthMappingSamples(),
        tracks: [
          {
            kind: "reference",
            id: "well-b-reference",
            title: "Reference",
            width: 80
          },
          {
            kind: "curve",
            id: "well-b-gr",
            title: "GR",
            width: 110,
            xAxis: {
              min: 0,
              max: 160,
              label: "GR",
              unit: "API"
            },
            series: [
              {
                id: "well-b-gr-series",
                name: "GR",
                color: "#1d4ed8",
                values: Float32Array.from([66, 78, 90]),
                nativeDepths: Float32Array.from([1000, 1100, 1250])
              }
            ]
          },
          {
            kind: "tops",
            id: "well-b-tops",
            title: "Tops",
            width: 70
          }
        ],
        tops: [
          {
            id: "well-b-top",
            name: "Reservoir",
            nativeDepth: 1110,
            color: "#9333ea",
            source: "imported"
          }
        ]
      }
    ]
  };
}
