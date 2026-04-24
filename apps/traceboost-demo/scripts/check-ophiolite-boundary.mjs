import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const frontendRoot = path.resolve(scriptDir, "..");
const sourceRoot = path.join(frontendRoot, "src");

const allowedImports = new Set([
  "@ophiolite/charts",
  "@ophiolite/charts/extras",
  "@ophiolite/charts-toolbar",
  "@ophiolite/charts-data-models",
  "@ophiolite/contracts"
]);

const sourceExtensions = new Set([".ts", ".js", ".svelte"]);
const disallowedPathFragments = [
  "/src/",
  "\\src\\",
  "../ophiolite/",
  "..\\ophiolite\\",
  "/charts/packages/",
  "\\charts\\packages\\"
];

const filePattern = /import\s+(?:type\s+)?(?:[^"'`]+\s+from\s+)?["'`]([^"'`]+)["'`]/g;

async function collectFiles(rootDir) {
  const entries = await readdir(rootDir, { recursive: true });
  return entries
    .map((entry) => path.join(rootDir, entry))
    .filter((entry) => sourceExtensions.has(path.extname(entry)));
}

const violations = [];

for (const filePath of await collectFiles(sourceRoot)) {
  const content = await readFile(filePath, "utf8");
  for (const match of content.matchAll(filePattern)) {
    const specifier = match[1];
    if (!specifier.includes("@ophiolite")) {
      continue;
    }

    if (disallowedPathFragments.some((fragment) => specifier.includes(fragment))) {
      violations.push({
        filePath,
        specifier,
        reason: "imports raw Ophiolite source instead of a public package"
      });
      continue;
    }

    if (!specifier.startsWith("@ophiolite/")) {
      violations.push({
        filePath,
        specifier,
        reason: "uses an unexpected Ophiolite import shape"
      });
      continue;
    }

    if (!allowedImports.has(specifier)) {
      violations.push({
        filePath,
        specifier,
        reason: "imports an Ophiolite package that is outside the approved consumer surface"
      });
    }
  }
}

if (violations.length > 0) {
  console.error("TraceBoost must consume Ophiolite through the approved package boundary.");
  for (const violation of violations) {
    console.error(`- ${path.relative(frontendRoot, violation.filePath)} -> ${violation.specifier}`);
    console.error(`  ${violation.reason}`);
  }
  process.exit(1);
}

console.log("TraceBoost only imports the approved Ophiolite consumer packages.");
