#!/usr/bin/env node

import { existsSync, mkdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const resultsDir = path.join(repoRoot, "articles/benchmarking/results");
const date = "2026-04-30";

const stores = {
  poseidon: "/Users/sc/Downloads/SubsurfaceData/poseidon/far_stack_roi_inline0_xline128_sample128_v3.tbvol",
  f3Small: "/Users/sc/Downloads/SubsurfaceData/blocks/F3/seismic/tbvol/DATR12I-021.tbvol"
};

const outputs = {
  poseidonSection: path.join(resultsDir, `${date}-poseidon-section-tile-baseline.json`),
  f3SmallSection: path.join(resultsDir, `${date}-f3-small-section-tile-baseline.json`),
  transcode: path.join(resultsDir, `${date}-tbvolc-transcode-smoke.json`)
};

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    ...options
  });
  if (result.status !== 0) {
    throw new Error(
      [
        `Command failed: ${command} ${args.join(" ")}`,
        result.stdout.trim(),
        result.stderr.trim()
      ]
        .filter(Boolean)
        .join("\n")
    );
  }
  return result.stdout;
}

function timedRun(command, args) {
  const started = process.hrtime.bigint();
  run(command, args);
  const ended = process.hrtime.bigint();
  return Number(ended - started) / 1_000_000_000;
}

function directorySizeBytes(root) {
  const stats = statSync(root);
  if (!stats.isDirectory()) {
    return stats.size;
  }
  const output = run("du", ["-sk", root]).trim();
  const kib = Number.parseInt(output.split(/\s+/)[0] ?? "0", 10);
  return kib * 1024;
}

function requireStore(label, storePath) {
  if (!existsSync(storePath)) {
    throw new Error(`Missing ${label} store: ${storePath}`);
  }
}

function runSectionBaseline(storePath, outputPath) {
  const json = run("target/release/section_tile_bench", [
    "--store",
    storePath,
    "--axis",
    "both",
    "--iterations",
    "7",
    "--screen-traces",
    "1200",
    "--screen-samples",
    "900",
    "--focus-traces",
    "256",
    "--focus-samples",
    "256",
    "--focus-lod",
    "0,1",
    "--format",
    "json"
  ]);
  JSON.parse(json);
  writeFileSync(outputPath, json.endsWith("\n") ? json : `${json}\n`);
}

function compareFiles(left, right) {
  const result = spawnSync("cmp", ["-s", left, right], {
    cwd: repoRoot,
    encoding: "utf8"
  });
  return result.status === 0;
}

mkdirSync(resultsDir, { recursive: true });
requireStore("Poseidon ROI", stores.poseidon);
requireStore("F3 small", stores.f3Small);

run("cargo", [
  "build",
  "-p",
  "ophiolite-seismic-runtime",
  "--bin",
  "section_tile_bench",
  "--bin",
  "tbvolc_transcode",
  "--release"
]);

runSectionBaseline(stores.poseidon, outputs.poseidonSection);
runSectionBaseline(stores.f3Small, outputs.f3SmallSection);

const poseidonArchive = "/tmp/poseidon-v3.tbvolc";
const poseidonRoundTrip = "/tmp/poseidon-v3-roundtrip.tbvol";
const f3Archive = "/tmp/DATR12I-021.tbvolc";

for (const tempPath of [poseidonArchive, poseidonRoundTrip, f3Archive]) {
  rmSync(tempPath, { recursive: true, force: true });
}

const poseidonEncodeSeconds = timedRun("target/release/tbvolc_transcode", [
  "encode",
  stores.poseidon,
  poseidonArchive
]);
const poseidonDecodeSeconds = timedRun("target/release/tbvolc_transcode", [
  "decode",
  poseidonArchive,
  poseidonRoundTrip
]);
const f3EncodeSeconds = timedRun("target/release/tbvolc_transcode", [
  "encode",
  stores.f3Small,
  f3Archive
]);

const transcodeReport = {
  status: "local exploratory baseline",
  generatedAt: new Date().toISOString(),
  benchmark: "tbvolc-transcode-smoke",
  cases: [
    {
      name: "poseidon-roi",
      sourcePath: stores.poseidon,
      sourceBytes: directorySizeBytes(stores.poseidon),
      archivePath: poseidonArchive,
      archiveBytes: directorySizeBytes(poseidonArchive),
      encodeSeconds: poseidonEncodeSeconds,
      decodeSeconds: poseidonDecodeSeconds,
      roundTripPath: poseidonRoundTrip,
      amplitudeByteExact: compareFiles(
        path.join(stores.poseidon, "amplitude.bin"),
        path.join(poseidonRoundTrip, "amplitude.bin")
      ),
      occupancyByteExact: compareFiles(
        path.join(stores.poseidon, "occupancy.bin"),
        path.join(poseidonRoundTrip, "occupancy.bin")
      )
    },
    {
      name: "f3-small-DATR12I-021",
      sourcePath: stores.f3Small,
      sourceBytes: directorySizeBytes(stores.f3Small),
      archivePath: f3Archive,
      archiveBytes: directorySizeBytes(f3Archive),
      encodeSeconds: f3EncodeSeconds,
      decodeSeconds: null,
      amplitudeByteExact: null,
      occupancyByteExact: null
    }
  ]
};

writeFileSync(outputs.transcode, `${JSON.stringify(transcodeReport, null, 2)}\n`);

console.log("Wrote benchmark artifacts:");
for (const outputPath of Object.values(outputs)) {
  console.log(`- ${path.relative(repoRoot, outputPath)}`);
}
