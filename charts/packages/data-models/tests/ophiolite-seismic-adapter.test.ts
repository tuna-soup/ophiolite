import assert from "node:assert/strict";
import test from "node:test";
import {
  OphioliteSeismicValidationError,
  adaptOphioliteSectionViewToPayload,
  validateSectionPayload
} from "../src/ophiolite-seismic-adapter";
import { createEncodedSectionView, createSectionPayload } from "../../../tests/fixtures/seismic";

test("section adapter preserves logical window metadata and display defaults", () => {
  const adapted = adaptOphioliteSectionViewToPayload(
    createEncodedSectionView({
      traces: 3,
      samples: 4,
      logicalDimensions: {
        traces: 12,
        samples: 20
      },
      window: {
        traceStart: 4,
        traceEnd: 7,
        sampleStart: 10,
        sampleEnd: 14,
        lod: 2
      },
      displayDefaults: {
        gain: 2.5,
        clipMin: -0.35,
        clipMax: 0.6,
        renderMode: "wiggle",
        colormap: "red_white_blue",
        polarity: "reversed"
      },
      units: {
        horizontal: "m",
        sample: "ms",
        amplitude: "amp"
      },
      metadata: {
        storeId: "store-1",
        derivedFrom: "raw-stack",
        notes: ["prefetched tile"]
      }
    })
  );

  assert.deepEqual(adapted.logicalDimensions, { traces: 12, samples: 20 });
  assert.deepEqual(adapted.window, {
    traceStart: 4,
    traceEnd: 7,
    sampleStart: 10,
    sampleEnd: 14,
    lod: 2
  });
  assert.deepEqual(adapted.displayDefaults, {
    gain: 2.5,
    clipMin: -0.35,
    clipMax: 0.6,
    renderMode: "wiggle",
    colormap: "red-white-blue",
    polarity: "reversed"
  });
  assert.deepEqual(adapted.units, {
    horizontal: "m",
    sample: "ms",
    amplitude: "amp"
  });
  assert.deepEqual(adapted.metadata, {
    storeId: "store-1",
    derivedFrom: "raw-stack",
    notes: ["prefetched tile"]
  });
});

test("validateSectionPayload reports window bounds that exceed logical dimensions", () => {
  const issues = validateSectionPayload(
    createSectionPayload({
      traces: 3,
      samples: 4,
      logicalDimensions: {
        traces: 2,
        samples: 3
      },
      window: {
        traceStart: 0,
        traceEnd: 3,
        sampleStart: 0,
        sampleEnd: 4
      }
    })
  );

  assert.deepEqual(
    issues.map((issue) => issue.code).sort(),
    ["section-window-sample-mismatch", "section-window-trace-mismatch"]
  );
});

test("section adapter throws a validation error for invalid logical windows", () => {
  assert.throws(
    () =>
      adaptOphioliteSectionViewToPayload(
        createEncodedSectionView({
          traces: 4,
          samples: 5,
          logicalDimensions: {
            traces: 3,
            samples: 5
          },
          window: {
            traceStart: 0,
            traceEnd: 4,
            sampleStart: 0,
            sampleEnd: 5
          }
        })
      ),
    (error: unknown) => {
      assert.ok(error instanceof OphioliteSeismicValidationError);
      assert.match(error.message, /section-window-trace-mismatch/);
      return true;
    }
  );
});
