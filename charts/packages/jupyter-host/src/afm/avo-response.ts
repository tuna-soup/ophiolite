import { mountAvoResponseChart } from "../index";

export default {
  render({ model, el }: { model: { get(name: string): any; on(name: string, cb: () => void): void; off(name: string, cb: () => void): void; set(name: string, value: unknown): void; save_changes(): void }; el: HTMLElement }) {
    let mounted: ReturnType<typeof mountAvoResponseChart> | null = null;

    const applyHeight = () => {
      const heightPx = Number(model.get("height_px") ?? 520);
      el.style.width = "100%";
      el.style.height = `${Math.max(240, heightPx)}px`;
      el.style.display = "block";
    };

    const syncSource = () => {
      const source = model.get("source");
      try {
        if (mounted) {
          mounted.setSource(source);
        } else {
          el.replaceChildren();
          mounted = mountAvoResponseChart(el, source, {
            chartId: model.get("chart_id") ?? undefined
          });
        }
        model.set("frontend_error", null);
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown AVO response widget error.";
        model.set("frontend_error", message);
        el.replaceChildren();
        const errorEl = document.createElement("div");
        errorEl.className = "ophiolite-jupyter-widget-error";
        errorEl.textContent = message;
        el.appendChild(errorEl);
        mounted = null;
      }
      model.save_changes();
    };

    const syncFitRequest = () => {
      if (mounted) {
        mounted.fitToData();
      }
    };

    const handleSourceChange = () => syncSource();
    const handleHeightChange = () => applyHeight();
    const handleFitChange = () => syncFitRequest();

    applyHeight();
    syncSource();

    model.on("change:source", handleSourceChange);
    model.on("change:height_px", handleHeightChange);
    model.on("change:fit_request_id", handleFitChange);

    return () => {
      model.off("change:source", handleSourceChange);
      model.off("change:height_px", handleHeightChange);
      model.off("change:fit_request_id", handleFitChange);
      void mounted?.dispose();
      mounted = null;
    };
  }
};
