import {
  CARTESIAN_FONT_FAMILY,
  formatCartesianCanvasFont,
  formatCartesianCssFont,
  type CartesianTextStyle
} from "./cartesian-presentation";

export type SeismicPresentationProfileId = "standard";

export interface SeismicPresentationTypography {
  tick: CartesianTextStyle;
  axisLabel: CartesianTextStyle;
  title: CartesianTextStyle;
  overlay: CartesianTextStyle;
  annotation: CartesianTextStyle;
}

export interface SeismicPresentationFrame {
  titleY: number;
  topTickLength: number;
  topTickOffset: number;
  topAxisRowSpacing: number;
  topAxisRowLabelOffset: number;
  topAxisLabelX: number;
  leftTickLength: number;
  leftTickOffset: number;
  yAxisLabelX: number;
  annotationOffsetX: number;
}

export interface SeismicPresentationPalette {
  shellBackground: string;
  axisStroke: string;
  axisLabel: string;
  title: string;
  overlayBackground: string;
  overlayText: string;
  overlayError: string;
  scrollbarTrack: string;
  scrollbarTrackBorder: string;
  scrollbarThumbStart: string;
  scrollbarThumbEnd: string;
  scrollbarThumbActiveStart: string;
  scrollbarThumbActiveEnd: string;
  scrollbarThumbInnerBorder: string;
  scrollbarThumbOuterBorder: string;
  annotationHalo: string;
}

export interface SeismicPresentationProfile {
  id: SeismicPresentationProfileId;
  typography: SeismicPresentationTypography;
  frame: SeismicPresentationFrame;
  palette: SeismicPresentationPalette;
}

const SHARED_SEISMIC_TYPOGRAPHY: SeismicPresentationTypography = {
  tick: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 10,
    weight: 500,
    lineHeight: 1
  },
  axisLabel: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 11,
    weight: 600,
    lineHeight: 1.1
  },
  title: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 14,
    weight: 600,
    lineHeight: 1.1
  },
  overlay: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 14,
    weight: 500,
    lineHeight: 1.4
  },
  annotation: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 11,
    weight: 600,
    lineHeight: 1.1
  }
};

const SEISMIC_PRESENTATION_PROFILES: Record<SeismicPresentationProfileId, SeismicPresentationProfile> = {
  standard: {
    id: "standard",
    typography: SHARED_SEISMIC_TYPOGRAPHY,
    frame: {
      titleY: 30,
      topTickLength: 7,
      topTickOffset: 10,
      topAxisRowSpacing: 16,
      topAxisRowLabelOffset: 16,
      topAxisLabelX: 8,
      leftTickLength: 7,
      leftTickOffset: 10,
      yAxisLabelX: 18,
      annotationOffsetX: 6
    },
    palette: {
      shellBackground: "#f2f6f8",
      axisStroke: "#a4bac8",
      axisLabel: "#425567",
      title: "#324355",
      overlayBackground: "rgba(244, 247, 249, 0.88)",
      overlayText: "#284052",
      overlayError: "#8f3c3c",
      scrollbarTrack: "rgba(228, 236, 241, 0.92)",
      scrollbarTrackBorder: "rgba(176, 212, 238, 0.68)",
      scrollbarThumbStart: "rgba(245, 249, 252, 0.96)",
      scrollbarThumbEnd: "rgba(190, 208, 219, 0.94)",
      scrollbarThumbActiveStart: "rgba(186, 215, 232, 0.94)",
      scrollbarThumbActiveEnd: "rgba(149, 186, 208, 0.94)",
      scrollbarThumbInnerBorder: "rgba(255, 255, 255, 0.72)",
      scrollbarThumbOuterBorder: "rgba(69, 93, 112, 0.2)",
      annotationHalo: "rgba(244, 247, 249, 0.96)"
    }
  }
};

export function resolveSeismicPresentationProfile(
  profileId: SeismicPresentationProfileId = "standard"
): SeismicPresentationProfile {
  return SEISMIC_PRESENTATION_PROFILES[profileId];
}

export function formatSeismicCssFont(style: CartesianTextStyle): string {
  return formatCartesianCssFont(style);
}

export function formatSeismicCanvasFont(style: CartesianTextStyle): string {
  return formatCartesianCanvasFont(style);
}
