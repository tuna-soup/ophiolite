/// <reference lib="webworker" />

import type { DisplayTransform, OverlayPayload, SectionPayload, SectionViewport } from "@ophiolite/charts-data-models";
import { prepareHeatmapData, prepareWiggleInstances, type PreparedHeatmapData, type PreparedWiggleInstances } from "./renderModel";
import type { WorkerIncomingMessage, WorkerOutgoingMessage, WorkerOverlayPayload, WorkerSectionPayload } from "./workerProtocol";

interface WorkerGlResources {
  heatmapProgram: WebGLProgram;
  heatmapQuadBuffer: WebGLBuffer;
  wiggleProgram: WebGLProgram;
  wiggleInstanceBuffer: WebGLBuffer;
  amplitudeTexture: WebGLTexture;
  overlayTexture: WebGLTexture;
  lutTexture: WebGLTexture;
  traceInstanceCount: number;
}

interface DirtyFlags {
  dataChanged: boolean;
  viewportChanged: boolean;
  styleChanged: boolean;
  overlayChanged: boolean;
  sizeChanged: boolean;
}

const DEFAULT_DISPLAY: DisplayTransform = {
  gain: 1,
  renderMode: "heatmap",
  colormap: "grayscale",
  polarity: "normal"
};

let canvas: OffscreenCanvas | null = null;
let gl: WebGL2RenderingContext | null = null;
let resources: WorkerGlResources | null = null;
let section: SectionPayload | null = null;
let overlay: OverlayPayload | null = null;
let viewport: SectionViewport | null = null;
let displayTransform: DisplayTransform = { ...DEFAULT_DISPLAY };
let width = 0;
let height = 0;
let plotRect = { x: 68, y: 72, width: 1, height: 1 };
let preparedHeatmap: PreparedHeatmapData | null = null;
let preparedWiggles: PreparedWiggleInstances | null = null;
let dirty: DirtyFlags = {
  dataChanged: false,
  viewportChanged: false,
  styleChanged: false,
  overlayChanged: false,
  sizeChanged: false
};
let frameScheduled = false;

self.onmessage = (event: MessageEvent<WorkerIncomingMessage>) => {
  try {
    handleMessage(event.data);
  } catch (error) {
    postWorkerMessage({
      type: "error",
      message: error instanceof Error ? error.message : "Unknown worker render error."
    });
  }
};

function handleMessage(message: WorkerIncomingMessage): void {
  switch (message.type) {
    case "init":
      canvas = message.canvas;
      width = message.state.width;
      height = message.state.height;
      plotRect = message.state.plotRect;
      canvas.width = width;
      canvas.height = height;
      gl = canvas.getContext("webgl2", {
        alpha: true,
        antialias: true,
        premultipliedAlpha: false
      });
      if (!gl) {
        throw new Error("Worker WebGL2 initialization failed.");
      }
      resources = createResources(gl);
      postWorkerMessage({
        type: "ready",
        offscreen: true,
        webgl2: true
      });
      markDirty({
        dataChanged: true,
        viewportChanged: true,
        styleChanged: true,
        overlayChanged: true,
        sizeChanged: true
      });
      break;
    case "resize":
      width = message.state.width;
      height = message.state.height;
      plotRect = message.state.plotRect;
      if (canvas) {
        canvas.width = width;
        canvas.height = height;
      }
      markDirty({ sizeChanged: true });
      break;
    case "setSection":
      section = reconstructSection(message.section);
      markDirty({ dataChanged: true, viewportChanged: true, styleChanged: true });
      break;
    case "setViewport":
      viewport = { ...message.viewport };
      markDirty({ viewportChanged: true });
      break;
    case "setDisplayTransform":
      displayTransform = { ...message.displayTransform };
      markDirty({ styleChanged: true });
      break;
    case "setOverlay":
      overlay = reconstructOverlay(message.overlay);
      markDirty({ overlayChanged: true });
      break;
    case "dispose":
      disposeResources();
      break;
  }
}

function markDirty(patch: Partial<DirtyFlags>): void {
  dirty = {
    dataChanged: dirty.dataChanged || Boolean(patch.dataChanged),
    viewportChanged: dirty.viewportChanged || Boolean(patch.viewportChanged),
    styleChanged: dirty.styleChanged || Boolean(patch.styleChanged),
    overlayChanged: dirty.overlayChanged || Boolean(patch.overlayChanged),
    sizeChanged: dirty.sizeChanged || Boolean(patch.sizeChanged)
  };

  if (!frameScheduled) {
    frameScheduled = true;
    scheduleFrame();
  }
}

function renderFrame(): void {
  frameScheduled = false;
  if (!gl || !resources) {
    return;
  }

  gl.viewport(0, 0, width, height);
  clearGl(gl, [0.949, 0.965, 0.973, 1]);

  if (!section || !viewport) {
    dirty = resetDirty();
    postWorkerMessage({ type: "frameRendered" });
    return;
  }

  if (dirty.dataChanged || !preparedHeatmap) {
    uploadAmplitudeTexture(gl, resources.amplitudeTexture, section);
  }

  if ((dirty.dataChanged || dirty.overlayChanged) && overlay) {
    uploadOverlayTexture(gl, resources.overlayTexture, overlay);
  }

  if (dirty.sizeChanged || dirty.viewportChanged || dirty.styleChanged) {
    gl.bindBuffer(gl.ARRAY_BUFFER, resources.heatmapQuadBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buildPlotQuadVertices(plotRect, width, height), gl.DYNAMIC_DRAW);
  }

  if (displayTransform.renderMode === "heatmap") {
    if (dirty.dataChanged || dirty.viewportChanged || dirty.styleChanged || dirty.overlayChanged || !preparedHeatmap) {
      preparedHeatmap = prepareHeatmapData(section, viewport, displayTransform, overlay);
      uploadLutTexture(gl, resources.lutTexture, preparedHeatmap.lut);
    }
    clearPlotGl(gl, plotRect, height, hexToRgbaArray("#f7fafc"));
    drawHeatmap(gl, resources, section, viewport, displayTransform, preparedHeatmap, overlay);
  } else {
    if (dirty.dataChanged || dirty.viewportChanged || dirty.styleChanged || dirty.sizeChanged || !preparedWiggles) {
      preparedWiggles = prepareWiggleInstances(section, viewport, displayTransform, plotRect, width);
      uploadWiggleInstances(gl, resources.wiggleInstanceBuffer, preparedWiggles);
      resources.traceInstanceCount = preparedWiggles.traceIndices.length;
    }
    clearPlotGl(gl, plotRect, height, hexToRgbaArray("#f8fafb"));
    drawWiggles(gl, resources, section, displayTransform, preparedWiggles);
  }

  dirty = resetDirty();
  postWorkerMessage({ type: "frameRendered" });
}

function reconstructSection(payload: WorkerSectionPayload): SectionPayload {
  return {
    axis: payload.axis,
    coordinate: payload.coordinate,
    dimensions: payload.dimensions,
    amplitudes: new Float32Array(payload.amplitudesBuffer),
    horizontalAxis: new Float64Array(payload.horizontalAxisBuffer),
    inlineAxis: payload.inlineAxisBuffer ? new Float64Array(payload.inlineAxisBuffer) : undefined,
    xlineAxis: payload.xlineAxisBuffer ? new Float64Array(payload.xlineAxisBuffer) : undefined,
    sampleAxis: new Float32Array(payload.sampleAxisBuffer),
    units: payload.units,
    metadata: payload.metadata
  };
}

function reconstructOverlay(payload: WorkerOverlayPayload | null): OverlayPayload | null {
  if (!payload) {
    return null;
  }

  return {
    kind: payload.kind,
    width: payload.width,
    height: payload.height,
    values: new Uint8Array(payload.valuesBuffer),
    opacity: payload.opacity
  };
}

function uploadWiggleInstances(glContext: WebGL2RenderingContext, buffer: WebGLBuffer, prepared: PreparedWiggleInstances): void {
  const interleaved = new Float32Array(prepared.traceIndices.length * 3);
  for (let index = 0; index < prepared.traceIndices.length; index += 1) {
    const offset = index * 3;
    interleaved[offset] = prepared.traceIndices[index]!;
    interleaved[offset + 1] = prepared.baselineClipX[index]!;
    interleaved[offset + 2] = prepared.amplitudeScaleClip[index]!;
  }

  glContext.bindBuffer(glContext.ARRAY_BUFFER, buffer);
  glContext.bufferData(glContext.ARRAY_BUFFER, interleaved, glContext.DYNAMIC_DRAW);
}

function createResources(glContext: WebGL2RenderingContext): WorkerGlResources {
  return {
    heatmapProgram: createProgram(glContext, HEATMAP_VERTEX_SHADER, HEATMAP_FRAGMENT_SHADER),
    heatmapQuadBuffer: createBuffer(glContext),
    wiggleProgram: createProgram(glContext, WIGGLE_VERTEX_SHADER, WIGGLE_FRAGMENT_SHADER),
    wiggleInstanceBuffer: createBuffer(glContext),
    amplitudeTexture: createTexture(glContext),
    overlayTexture: createTexture(glContext),
    lutTexture: createTexture(glContext),
    traceInstanceCount: 0
  };
}

function drawHeatmap(
  glContext: WebGL2RenderingContext,
  glResources: WorkerGlResources,
  seismicSection: SectionPayload,
  seismicViewport: SectionViewport,
  transform: DisplayTransform,
  prepared: PreparedHeatmapData,
  visibleOverlay: OverlayPayload | null
): void {
  glContext.enable(glContext.SCISSOR_TEST);
  glContext.scissor(plotRect.x, height - plotRect.y - plotRect.height, plotRect.width, plotRect.height);
  glContext.useProgram(glResources.heatmapProgram);
  glContext.bindBuffer(glContext.ARRAY_BUFFER, glResources.heatmapQuadBuffer);

  const positionLocation = glContext.getAttribLocation(glResources.heatmapProgram, "aPosition");
  const uvLocation = glContext.getAttribLocation(glResources.heatmapProgram, "aUv");
  glContext.enableVertexAttribArray(positionLocation);
  // Reset divisors in case the previous pass was the instanced wiggle renderer.
  glContext.vertexAttribDivisor(positionLocation, 0);
  glContext.vertexAttribPointer(positionLocation, 2, glContext.FLOAT, false, 16, 0);
  glContext.enableVertexAttribArray(uvLocation);
  glContext.vertexAttribDivisor(uvLocation, 0);
  glContext.vertexAttribPointer(uvLocation, 2, glContext.FLOAT, false, 16, 8);

  bindTexture(glContext, glResources.amplitudeTexture, 0);
  bindTexture(glContext, glResources.lutTexture, 1);
  bindTexture(glContext, glResources.overlayTexture, 2);

  glContext.uniform1i(glContext.getUniformLocation(glResources.heatmapProgram, "uAmplitude"), 0);
  glContext.uniform1i(glContext.getUniformLocation(glResources.heatmapProgram, "uLut"), 1);
  glContext.uniform1i(glContext.getUniformLocation(glResources.heatmapProgram, "uOverlay"), 2);
  glContext.uniform2f(glContext.getUniformLocation(glResources.heatmapProgram, "uSectionSize"), seismicSection.dimensions.samples, seismicSection.dimensions.traces);
  glContext.uniform4f(glContext.getUniformLocation(glResources.heatmapProgram, "uViewport"), seismicViewport.traceStart, seismicViewport.traceEnd, seismicViewport.sampleStart, seismicViewport.sampleEnd);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uGain"), transform.gain);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uClipMin"), prepared.clipMin);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uClipMax"), prepared.clipMax);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uSymmetricExtent"), prepared.symmetricExtent);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uUseDiverging"), transform.colormap === "red-white-blue" ? 1 : 0);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uPolaritySign"), transform.polarity === "reversed" ? -1 : 1);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uOverlayEnabled"), visibleOverlay ? 1 : 0);
  glContext.uniform1f(glContext.getUniformLocation(glResources.heatmapProgram, "uOverlayOpacity"), prepared.overlayOpacity);
  glContext.drawArrays(glContext.TRIANGLES, 0, 6);
  glContext.disable(glContext.SCISSOR_TEST);
}

function drawWiggles(
  glContext: WebGL2RenderingContext,
  glResources: WorkerGlResources,
  seismicSection: SectionPayload,
  transform: DisplayTransform,
  prepared: PreparedWiggleInstances | null
): void {
  if (!prepared || glResources.traceInstanceCount === 0) {
    return;
  }

  glContext.enable(glContext.SCISSOR_TEST);
  glContext.scissor(plotRect.x, height - plotRect.y - plotRect.height, plotRect.width, plotRect.height);
  glContext.enable(glContext.BLEND);
  glContext.blendFunc(glContext.SRC_ALPHA, glContext.ONE_MINUS_SRC_ALPHA);
  glContext.useProgram(glResources.wiggleProgram);
  bindTexture(glContext, glResources.amplitudeTexture, 0);
  glContext.uniform1i(glContext.getUniformLocation(glResources.wiggleProgram, "uAmplitude"), 0);
  glContext.uniform2f(glContext.getUniformLocation(glResources.wiggleProgram, "uTextureSize"), seismicSection.dimensions.samples, seismicSection.dimensions.traces);
  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uGain"), transform.gain);
  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uPolaritySign"), transform.polarity === "reversed" ? -1 : 1);
  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uPlotY"), plotRect.y);
  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uPlotHeight"), plotRect.height);
  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uCanvasHeight"), height);
  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uSampleStart"), prepared.sampleStart);
  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uSampleCount"), prepared.sampleCount);

  glContext.bindBuffer(glContext.ARRAY_BUFFER, glResources.wiggleInstanceBuffer);
  const traceIndexLocation = glContext.getAttribLocation(glResources.wiggleProgram, "aTraceIndex");
  const baselineLocation = glContext.getAttribLocation(glResources.wiggleProgram, "aBaselineClipX");
  const amplitudeScaleLocation = glContext.getAttribLocation(glResources.wiggleProgram, "aAmplitudeScaleClip");
  glContext.enableVertexAttribArray(traceIndexLocation);
  glContext.vertexAttribPointer(traceIndexLocation, 1, glContext.FLOAT, false, 12, 0);
  glContext.vertexAttribDivisor(traceIndexLocation, 1);
  glContext.enableVertexAttribArray(baselineLocation);
  glContext.vertexAttribPointer(baselineLocation, 1, glContext.FLOAT, false, 12, 4);
  glContext.vertexAttribDivisor(baselineLocation, 1);
  glContext.enableVertexAttribArray(amplitudeScaleLocation);
  glContext.vertexAttribPointer(amplitudeScaleLocation, 1, glContext.FLOAT, false, 12, 8);
  glContext.vertexAttribDivisor(amplitudeScaleLocation, 1);

  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uFillMode"), 1);
  glContext.uniform4f(glContext.getUniformLocation(glResources.wiggleProgram, "uColor"), 0.063, 0.078, 0.094, 0.7);
  glContext.drawArraysInstanced(glContext.TRIANGLE_STRIP, 0, prepared.sampleCount * 2, glResources.traceInstanceCount);

  glContext.uniform1f(glContext.getUniformLocation(glResources.wiggleProgram, "uFillMode"), 0);
  glContext.uniform4f(glContext.getUniformLocation(glResources.wiggleProgram, "uColor"), 0.063, 0.078, 0.094, 1);
  glContext.drawArraysInstanced(glContext.LINE_STRIP, 0, prepared.sampleCount, glResources.traceInstanceCount);

  glContext.vertexAttribDivisor(traceIndexLocation, 0);
  glContext.vertexAttribDivisor(baselineLocation, 0);
  glContext.vertexAttribDivisor(amplitudeScaleLocation, 0);
  glContext.disable(glContext.BLEND);
  glContext.disable(glContext.SCISSOR_TEST);
}

function uploadAmplitudeTexture(glContext: WebGL2RenderingContext, texture: WebGLTexture, seismicSection: SectionPayload): void {
  bindTexture(glContext, texture, 0);
  glContext.pixelStorei(glContext.UNPACK_ALIGNMENT, 1);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_MIN_FILTER, glContext.NEAREST);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_MAG_FILTER, glContext.NEAREST);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_WRAP_S, glContext.CLAMP_TO_EDGE);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_WRAP_T, glContext.CLAMP_TO_EDGE);
  glContext.texImage2D(glContext.TEXTURE_2D, 0, glContext.R32F, seismicSection.dimensions.samples, seismicSection.dimensions.traces, 0, glContext.RED, glContext.FLOAT, seismicSection.amplitudes);
}

function uploadOverlayTexture(glContext: WebGL2RenderingContext, texture: WebGLTexture, payload: OverlayPayload): void {
  bindTexture(glContext, texture, 0);
  glContext.pixelStorei(glContext.UNPACK_ALIGNMENT, 1);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_MIN_FILTER, glContext.NEAREST);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_MAG_FILTER, glContext.NEAREST);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_WRAP_S, glContext.CLAMP_TO_EDGE);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_WRAP_T, glContext.CLAMP_TO_EDGE);
  glContext.texImage2D(glContext.TEXTURE_2D, 0, glContext.R8, payload.height, payload.width, 0, glContext.RED, glContext.UNSIGNED_BYTE, payload.values);
}

function uploadLutTexture(glContext: WebGL2RenderingContext, texture: WebGLTexture, lut: Uint8Array): void {
  bindTexture(glContext, texture, 0);
  glContext.pixelStorei(glContext.UNPACK_ALIGNMENT, 1);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_MIN_FILTER, glContext.LINEAR);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_MAG_FILTER, glContext.LINEAR);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_WRAP_S, glContext.CLAMP_TO_EDGE);
  glContext.texParameteri(glContext.TEXTURE_2D, glContext.TEXTURE_WRAP_T, glContext.CLAMP_TO_EDGE);
  glContext.texImage2D(glContext.TEXTURE_2D, 0, glContext.RGBA, 256, 1, 0, glContext.RGBA, glContext.UNSIGNED_BYTE, lut);
}

function bindTexture(glContext: WebGL2RenderingContext, texture: WebGLTexture, unit: number): void {
  glContext.activeTexture(glContext.TEXTURE0 + unit);
  glContext.bindTexture(glContext.TEXTURE_2D, texture);
}

function buildPlotQuadVertices(rect: typeof plotRect, canvasWidth: number, canvasHeight: number): Float32Array {
  const left = (rect.x / canvasWidth) * 2 - 1;
  const right = ((rect.x + rect.width) / canvasWidth) * 2 - 1;
  const top = 1 - (rect.y / canvasHeight) * 2;
  const bottom = 1 - ((rect.y + rect.height) / canvasHeight) * 2;
  return new Float32Array([left, bottom, 0, 1, right, bottom, 1, 1, right, top, 1, 0, left, bottom, 0, 1, right, top, 1, 0, left, top, 0, 0]);
}

function clearGl(glContext: WebGL2RenderingContext, rgba: [number, number, number, number]): void {
  glContext.disable(glContext.SCISSOR_TEST);
  glContext.clearColor(rgba[0], rgba[1], rgba[2], rgba[3]);
  glContext.clear(glContext.COLOR_BUFFER_BIT);
}

function clearPlotGl(glContext: WebGL2RenderingContext, rect: typeof plotRect, canvasHeight: number, rgba: [number, number, number, number]): void {
  glContext.enable(glContext.SCISSOR_TEST);
  glContext.scissor(rect.x, canvasHeight - rect.y - rect.height, rect.width, rect.height);
  glContext.clearColor(rgba[0], rgba[1], rgba[2], rgba[3]);
  glContext.clear(glContext.COLOR_BUFFER_BIT);
  glContext.disable(glContext.SCISSOR_TEST);
}

function createProgram(glContext: WebGL2RenderingContext, vertexSource: string, fragmentSource: string): WebGLProgram {
  const program = glContext.createProgram();
  const vertexShader = compileShader(glContext, glContext.VERTEX_SHADER, vertexSource);
  const fragmentShader = compileShader(glContext, glContext.FRAGMENT_SHADER, fragmentSource);
  if (!program || !vertexShader || !fragmentShader) {
    throw new Error("Failed to create worker WebGL program.");
  }
  glContext.attachShader(program, vertexShader);
  glContext.attachShader(program, fragmentShader);
  glContext.linkProgram(program);
  glContext.deleteShader(vertexShader);
  glContext.deleteShader(fragmentShader);
  if (!glContext.getProgramParameter(program, glContext.LINK_STATUS)) {
    throw new Error(glContext.getProgramInfoLog(program) ?? "Failed to link worker WebGL program.");
  }
  return program;
}

function compileShader(glContext: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = glContext.createShader(type);
  if (!shader) {
    throw new Error("Failed to create worker shader.");
  }
  glContext.shaderSource(shader, source);
  glContext.compileShader(shader);
  if (!glContext.getShaderParameter(shader, glContext.COMPILE_STATUS)) {
    const message = glContext.getShaderInfoLog(shader) ?? "Failed to compile worker shader.";
    glContext.deleteShader(shader);
    throw new Error(message);
  }
  return shader;
}

function createBuffer(glContext: WebGL2RenderingContext): WebGLBuffer {
  const buffer = glContext.createBuffer();
  if (!buffer) {
    throw new Error("Failed to create worker buffer.");
  }
  return buffer;
}

function createTexture(glContext: WebGL2RenderingContext): WebGLTexture {
  const texture = glContext.createTexture();
  if (!texture) {
    throw new Error("Failed to create worker texture.");
  }
  return texture;
}

function hexToRgbaArray(value: string): [number, number, number, number] {
  const normalized = value.replace("#", "");
  return [
    Number.parseInt(normalized.slice(0, 2), 16) / 255,
    Number.parseInt(normalized.slice(2, 4), 16) / 255,
    Number.parseInt(normalized.slice(4, 6), 16) / 255,
    1
  ];
}

function resetDirty(): DirtyFlags {
  return {
    dataChanged: false,
    viewportChanged: false,
    styleChanged: false,
    overlayChanged: false,
    sizeChanged: false
  };
}

function postWorkerMessage(message: WorkerOutgoingMessage): void {
  self.postMessage(message);
}

function scheduleFrame(): void {
  if (typeof self.requestAnimationFrame === "function") {
    self.requestAnimationFrame(renderFrame);
    return;
  }

  setTimeout(renderFrame, 16);
}

function disposeResources(): void {
  if (gl && resources) {
    gl.deleteProgram(resources.heatmapProgram);
    gl.deleteProgram(resources.wiggleProgram);
    gl.deleteBuffer(resources.heatmapQuadBuffer);
    gl.deleteBuffer(resources.wiggleInstanceBuffer);
    gl.deleteTexture(resources.amplitudeTexture);
    gl.deleteTexture(resources.overlayTexture);
    gl.deleteTexture(resources.lutTexture);
  }
  resources = null;
  gl = null;
  canvas = null;
}

const HEATMAP_VERTEX_SHADER = `#version 300 es
in vec2 aPosition;
in vec2 aUv;
out vec2 vUv;
void main() {
  vUv = aUv;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`;

const HEATMAP_FRAGMENT_SHADER = `#version 300 es
precision highp float;
in vec2 vUv;
uniform sampler2D uAmplitude;
uniform sampler2D uLut;
uniform sampler2D uOverlay;
uniform vec2 uSectionSize;
uniform vec4 uViewport;
uniform float uGain;
uniform float uClipMin;
uniform float uClipMax;
uniform float uSymmetricExtent;
uniform float uUseDiverging;
uniform float uPolaritySign;
uniform float uOverlayEnabled;
uniform float uOverlayOpacity;
out vec4 outColor;
void main() {
  float traceIndex = mix(uViewport.x, uViewport.y - 1.0, vUv.x);
  float sampleIndex = mix(uViewport.z, uViewport.w - 1.0, vUv.y);
  vec2 sampleUv = vec2((sampleIndex + 0.5) / uSectionSize.x, (traceIndex + 0.5) / uSectionSize.y);
  float amplitude = texture(uAmplitude, sampleUv).r * uGain * uPolaritySign;
  float normalized = uUseDiverging > 0.5
    ? (amplitude / max(uSymmetricExtent, 0.000001) + 1.0) * 0.5
    : (amplitude - uClipMin) / max(uClipMax - uClipMin, 0.000001);
  normalized = clamp(normalized, 0.0, 1.0);
  vec4 color = texture(uLut, vec2(normalized, 0.5));
  if (uOverlayEnabled > 0.5) {
    float mask = texture(uOverlay, sampleUv).r;
    color.rgb = mix(color.rgb, vec3(0.976, 0.451, 0.086), mask * uOverlayOpacity);
  }
  outColor = color;
}`;

const WIGGLE_VERTEX_SHADER = `#version 300 es
precision highp float;
in float aTraceIndex;
in float aBaselineClipX;
in float aAmplitudeScaleClip;
uniform sampler2D uAmplitude;
uniform vec2 uTextureSize;
uniform float uGain;
uniform float uPolaritySign;
uniform float uPlotY;
uniform float uPlotHeight;
uniform float uCanvasHeight;
uniform float uSampleStart;
uniform float uSampleCount;
uniform float uFillMode;
void main() {
  float logicalVertex = float(gl_VertexID);
  float sampleOffset = uFillMode > 0.5 ? floor(logicalVertex / 2.0) : logicalVertex;
  float sampleIndex = uSampleStart + sampleOffset;
  float amplitude = texelFetch(uAmplitude, ivec2(int(sampleIndex), int(aTraceIndex)), 0).r;
  float offset = amplitude * uGain * aAmplitudeScaleClip * uPolaritySign;
  if (uFillMode > 0.5) {
    float side = mod(logicalVertex, 2.0);
    offset = max(offset, 0.0) * side;
  }
  float yPx = uPlotY + (sampleOffset / max(uSampleCount - 1.0, 1.0)) * uPlotHeight;
  float yClip = 1.0 - (yPx / uCanvasHeight) * 2.0;
  gl_Position = vec4(aBaselineClipX + offset, yClip, 0.0, 1.0);
}`;

const WIGGLE_FRAGMENT_SHADER = `#version 300 es
precision mediump float;
uniform vec4 uColor;
out vec4 outColor;
void main() {
  outColor = uColor;
}`;

export {};
