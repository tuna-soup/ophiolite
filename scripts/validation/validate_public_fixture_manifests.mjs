#!/usr/bin/env node

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const manifestDir = path.join(repoRoot, "test_data", "seismic", "public-fixtures");
const skippedFiles = new Set(["schema.example.json"]);

const requiredTopLevel = [
  "schema_version",
  "id",
  "source",
  "license_note",
  "adapter_hint",
  "fetch_policy",
  "subset",
  "expected"
];

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function requireObject(errors, manifest, field, file) {
  if (!isObject(manifest[field])) {
    errors.push(`${file}: ${field} must be an object`);
  }
}

function requireString(errors, value, label, file) {
  if (typeof value !== "string" || value.trim() === "") {
    errors.push(`${file}: ${label} must be a non-empty string`);
  }
}

function validateManifest(file, manifest) {
  const errors = [];

  for (const field of requiredTopLevel) {
    if (!(field in manifest)) {
      errors.push(`${file}: missing required field ${field}`);
    }
  }

  requireString(errors, manifest.schema_version, "schema_version", file);
  requireString(errors, manifest.id, "id", file);
  requireObject(errors, manifest, "source", file);
  requireString(errors, manifest.license_note, "license_note", file);
  requireObject(errors, manifest, "adapter_hint", file);
  requireObject(errors, manifest, "fetch_policy", file);
  requireObject(errors, manifest, "subset", file);
  requireObject(errors, manifest, "expected", file);

  if (isObject(manifest.source)) {
    requireString(errors, manifest.source.kind, "source.kind", file);
    requireString(errors, manifest.source.uri, "source.uri", file);
  }

  if (isObject(manifest.adapter_hint)) {
    requireString(errors, manifest.adapter_hint.format, "adapter_hint.format", file);
  }

  if (isObject(manifest.fetch_policy)) {
    requireString(errors, manifest.fetch_policy.default, "fetch_policy.default", file);
    if (manifest.fetch_policy.default !== "offline") {
      errors.push(`${file}: fetch_policy.default must be offline`);
    }
    if (manifest.fetch_policy.requires_opt_in !== true) {
      errors.push(`${file}: fetch_policy.requires_opt_in must be true`);
    }
  }

  if (isObject(manifest.expected)) {
    requireObject(errors, manifest.expected, "canonical_preview", file);
    requireObject(errors, manifest.expected, "warnings", file);
    requireObject(errors, manifest.expected, "blockers", file);

    if (isObject(manifest.expected.warnings) && manifest.expected.warnings.authoritative !== false) {
      errors.push(`${file}: expected.warnings.authoritative must be false for scaffold manifests`);
    }
    if (isObject(manifest.expected.blockers) && manifest.expected.blockers.authoritative !== false) {
      errors.push(`${file}: expected.blockers.authoritative must be false for scaffold manifests`);
    }
  }

  return errors;
}

const entries = await readdir(manifestDir, { withFileTypes: true });
const jsonFiles = entries
  .filter((entry) => entry.isFile() && entry.name.endsWith(".json") && !skippedFiles.has(entry.name))
  .map((entry) => entry.name)
  .sort();

let failures = [];

for (const file of jsonFiles) {
  const fullPath = path.join(manifestDir, file);
  let manifest;
  try {
    manifest = JSON.parse(await readFile(fullPath, "utf8"));
  } catch (error) {
    failures.push(`${file}: invalid JSON: ${error.message}`);
    continue;
  }

  failures = failures.concat(validateManifest(file, manifest));
}

if (failures.length > 0) {
  console.error(`Public fixture manifest validation failed (${failures.length} issue(s))`);
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`Validated ${jsonFiles.length} public fixture manifest(s); skipped schema.example.json`);
