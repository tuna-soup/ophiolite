export type CartesianPresentationProfileId = "avo" | "rockPhysics";

export interface CartesianTextStyle {
  family: string;
  sizePx: number;
  weight: number;
  lineHeight: number;
}

export interface CartesianPresentationPlotInsets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface CartesianPresentationTypography {
  tick: CartesianTextStyle;
  axisLabel: CartesianTextStyle;
  subtitle: CartesianTextStyle;
  title: CartesianTextStyle;
}

export interface CartesianPresentationFrame {
  plotInsets: CartesianPresentationPlotInsets;
  titleY: number;
  subtitleY: number;
  xTickOffset: number;
  xAxisLabelOffset: number;
  yTickOffset: number;
  yAxisLabelX: number;
  legendTopPadding: number;
  legendBandPaddingRight: number;
  probePanelPadding: number;
}

export interface CartesianPresentationProfile {
  id: CartesianPresentationProfileId;
  frame: CartesianPresentationFrame;
  typography: CartesianPresentationTypography;
}

export interface CartesianStageLayout {
  plotRect: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  title: {
    x: number;
    y: number;
  };
  subtitle: {
    x: number;
    y: number;
  };
  xTickY: number;
  xAxisLabelY: number;
  yTickX: number;
  yAxisLabelX: number;
  legendTop: number;
  legendRight: number;
  probePanelInset: number;
}

export const CARTESIAN_FONT_FAMILY = 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';

const SHARED_CARTESIAN_TYPOGRAPHY: CartesianPresentationTypography = {
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
  subtitle: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 11,
    weight: 500,
    lineHeight: 1.1
  },
  title: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 14,
    weight: 600,
    lineHeight: 1.1
  }
};

const CARTESIAN_PRESENTATION_PROFILES: Record<CartesianPresentationProfileId, CartesianPresentationProfile> = {
  avo: {
    id: "avo",
    frame: {
      plotInsets: {
        top: 56,
        right: 228,
        bottom: 56,
        left: 72
      },
      titleY: 30,
      subtitleY: 48,
      xTickOffset: 22,
      xAxisLabelOffset: 46,
      yTickOffset: 10,
      yAxisLabelX: 20,
      legendTopPadding: 16,
      legendBandPaddingRight: 12,
      probePanelPadding: 10
    },
    typography: SHARED_CARTESIAN_TYPOGRAPHY
  },
  rockPhysics: {
    id: "rockPhysics",
    frame: {
      plotInsets: {
        top: 56,
        right: 28,
        bottom: 56,
        left: 72
      },
      titleY: 30,
      subtitleY: 48,
      xTickOffset: 22,
      xAxisLabelOffset: 46,
      yTickOffset: 10,
      yAxisLabelX: 20,
      legendTopPadding: 16,
      legendBandPaddingRight: 12,
      probePanelPadding: 10
    },
    typography: SHARED_CARTESIAN_TYPOGRAPHY
  }
};

export function resolveCartesianPresentationProfile(
  profileId: CartesianPresentationProfileId
): CartesianPresentationProfile {
  return CARTESIAN_PRESENTATION_PROFILES[profileId];
}

export function resolveCartesianStageLayout(
  width: number,
  height: number,
  profile: CartesianPresentationProfile | CartesianPresentationProfileId
): CartesianStageLayout {
  const resolved = typeof profile === "string" ? resolveCartesianPresentationProfile(profile) : profile;
  const { plotInsets } = resolved.frame;
  const plotRect = {
    x: plotInsets.left,
    y: plotInsets.top,
    width: Math.max(1, width - plotInsets.left - plotInsets.right),
    height: Math.max(1, height - plotInsets.top - plotInsets.bottom)
  };

  return {
    plotRect,
    title: {
      x: plotRect.x,
      y: resolved.frame.titleY
    },
    subtitle: {
      x: plotRect.x,
      y: resolved.frame.subtitleY
    },
    xTickY: plotRect.y + plotRect.height + resolved.frame.xTickOffset,
    xAxisLabelY: plotRect.y + plotRect.height + resolved.frame.xAxisLabelOffset,
    yTickX: plotRect.x - resolved.frame.yTickOffset,
    yAxisLabelX: resolved.frame.yAxisLabelX,
    legendTop: plotRect.y + resolved.frame.legendTopPadding,
    legendRight: Math.max(0, plotInsets.right - resolved.frame.legendBandPaddingRight),
    probePanelInset: resolved.frame.probePanelPadding
  };
}

export function formatCartesianCssFont(style: CartesianTextStyle): string {
  return `${style.weight} ${style.sizePx}px/${style.lineHeight} ${style.family}`;
}

export function formatCartesianCanvasFont(style: CartesianTextStyle): string {
  return `${style.weight} ${style.sizePx}px ${style.family}`;
}
