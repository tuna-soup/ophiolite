import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { createDecodeStats, decodeF32Le, decodeF64Le } from "./section-adapter";

describe("section-adapter decode helpers", () => {
  test("decodeF32Le returns an aligned zero-copy view by default", () => {
    const values = new Float32Array([1, 2, 3, 4]);
    const bytes = new Uint8Array(values.buffer);
    const stats = createDecodeStats();

    const decoded = decodeF32Le(bytes, { stats });

    assert.equal(decoded.buffer, bytes.buffer);
    assert.deepEqual(Array.from(decoded), [1, 2, 3, 4]);
    assert.equal(stats.copiedBytes, 0);
    assert.equal(stats.viewedBytes, bytes.byteLength);
  });

  test("decodeF64Le copies when copy mode is requested", () => {
    const values = new Float64Array([10, 20, 30]);
    const bytes = new Uint8Array(values.buffer);
    const stats = createDecodeStats();

    const decoded = decodeF64Le(bytes, { copyMode: "copy", stats });

    assert.notEqual(decoded.buffer, bytes.buffer);
    assert.deepEqual(Array.from(decoded), [10, 20, 30]);
    assert.equal(stats.copiedBytes, bytes.byteLength);
    assert.equal(stats.viewedBytes, 0);
  });

  test("decodeF32Le copies unaligned byte views", () => {
    const values = new Float32Array([42]);
    const backing = new Uint8Array(values.byteLength + 1);
    backing.set(new Uint8Array(values.buffer), 1);
    const unaligned = backing.subarray(1);
    const stats = createDecodeStats();

    const decoded = decodeF32Le(unaligned, { stats });

    assert.notEqual(decoded.buffer, unaligned.buffer);
    assert.equal(decoded[0], 42);
    assert.equal(stats.copiedBytes, unaligned.byteLength);
    assert.equal(stats.viewedBytes, 0);
  });
});
