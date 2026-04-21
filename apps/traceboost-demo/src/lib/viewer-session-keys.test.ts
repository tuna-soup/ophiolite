import assert from "node:assert/strict";
import test from "node:test";
import {
  buildViewerSessionKey,
  buildViewportMemoryKey,
  resolveViewerResetReason
} from "./viewer-session-keys";

test("buildViewportMemoryKey scopes viewport memory to store, geometry, axis, and domain", () => {
  assert.equal(
    buildViewportMemoryKey({
      storePath: "/tmp/a.tbvol",
      geometryFingerprint: "geom-a",
      axis: "inline",
      domain: "time"
    }),
    "viewport:v1:/tmp/a.tbvol:geom-a:inline:time"
  );
});

test("buildViewerSessionKey normalizes missing identity parts", () => {
  assert.equal(
    buildViewerSessionKey({
      storePath: " ",
      geometryFingerprint: null,
      domain: "depth"
    }),
    "viewer:v1:no-store:none:depth"
  );
});

test("resolveViewerResetReason prioritizes store, then domain, then geometry changes", () => {
  const current = {
    storePath: "/tmp/a.tbvol",
    geometryFingerprint: "geom-a",
    domain: "time" as const
  };

  assert.equal(
    resolveViewerResetReason(current, {
      storePath: "/tmp/b.tbvol",
      geometryFingerprint: "geom-a",
      domain: "time"
    }),
    "store_switch"
  );
  assert.equal(
    resolveViewerResetReason(current, {
      storePath: "/tmp/a.tbvol",
      geometryFingerprint: "geom-a",
      domain: "depth"
    }),
    "domain_change"
  );
  assert.equal(
    resolveViewerResetReason(current, {
      storePath: "/tmp/a.tbvol",
      geometryFingerprint: "geom-b",
      domain: "time"
    }),
    "geometry_change"
  );
  assert.equal(
    resolveViewerResetReason(current, {
      storePath: "/tmp/a.tbvol",
      geometryFingerprint: "geom-a",
      domain: "time"
    }),
    null
  );
});
