#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

let chromium;
try {
  ({ chromium } = await import("playwright-core"));
} catch {
  try {
    const explicitRoot = process.env.PLAYWRIGHT_CORE_ROOT;
    if (!explicitRoot) {
      throw new Error("missing PLAYWRIGHT_CORE_ROOT");
    }
    ({ chromium } = await import(`${explicitRoot}/playwright-core/index.mjs`));
  } catch {
    console.error(
      "Missing dependency: playwright-core. Install it first, for example:\n" +
        "  npm install --prefix /tmp/nzpam-probe playwright-core\n" +
        "Then run with PLAYWRIGHT_CORE_ROOT=/tmp/nzpam-probe/node_modules"
    );
    process.exit(2);
  }
}

const chromePath =
  process.env.PLAYWRIGHT_CHROME_BIN ||
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const username = process.env.REALME_USER;
const password = process.env.REALME_PASS;
const batchName = process.env.NZPAM_BATCH_NAME || "default";
const maxFiles = Number.parseInt(process.env.NZPAM_MAX_FILES || "0", 10);
const startIndex = Number.parseInt(process.env.NZPAM_START_INDEX || "0", 10);
const stagingDir =
  process.env.NZPAM_STAGING_DIR || "/Users/sc/Downloads/SubsurfaceData/nzpam/.staging";

if (!username || !password) {
  console.error("REALME_USER and REALME_PASS are required.");
  process.exit(2);
}

const batches = {
  default: [
    {
      label: "PR5496 velocity D_D2VINT 6.91GB",
      url: "https://geodata.nzpam.govt.nz/dataset/b72e01ea-cb29-500c-ac81-2f0a32c425a6/resource/2e26509b-7008-5870-9120-60bd42c14abd/fpx",
      outputDir: "/Users/sc/Downloads/SubsurfaceData/nzpam/seismic/PR5496",
    },
    {
      label: "PR5496 full stack PSTM FULL 17.56GB",
      url: "https://geodata.nzpam.govt.nz/dataset/b72e01ea-cb29-500c-ac81-2f0a32c425a6/resource/2ed2b0cf-7bb0-54b0-a2dc-54124cb1ac8e/fpx",
      outputDir: "/Users/sc/Downloads/SubsurfaceData/nzpam/seismic/PR5496",
    },
    {
      label: "PR5496 angle stack NAS-SW 14.44GB",
      url: "https://geodata.nzpam.govt.nz/dataset/b72e01ea-cb29-500c-ac81-2f0a32c425a6/resource/24617ceb-794f-5512-ba3a-79f6a5346ff5/fpx",
      outputDir: "/Users/sc/Downloads/SubsurfaceData/nzpam/seismic/PR5496",
    },
    {
      label: "PR5770 raw stack 6.38GB",
      url: "https://geodata.nzpam.govt.nz/dataset/4e121d48-9961-50a1-a51f-7828eac2c03c/resource/32382dd9-ae40-5b49-adb6-0fcf8b43edb0/fpx",
      outputDir: "/Users/sc/Downloads/SubsurfaceData/nzpam/seismic/PR5770",
    },
  ],
};

const targets = batches[batchName];
if (!targets) {
  console.error(`Unknown NZPAM batch: ${batchName}`);
  process.exit(2);
}

const selectedTargets = targets
  .slice(Number.isNaN(startIndex) ? 0 : startIndex)
  .slice(0, Number.isNaN(maxFiles) || maxFiles <= 0 ? undefined : maxFiles);

if (selectedTargets.length === 0) {
  console.error(
    `No targets selected for batch=${batchName} startIndex=${startIndex} maxFiles=${maxFiles}`
  );
  process.exit(2);
}

function log(message) {
  console.log(`[${new Date().toISOString()}] ${message}`);
}

async function finalizeDownload(download, outputDir) {
  const suggested = download.suggestedFilename();
  const outputPath = path.join(outputDir, suggested);
  fs.mkdirSync(outputDir, { recursive: true });

  const failure = await download.failure();
  if (failure) {
    throw new Error(`Download failed for ${suggested}: ${failure}`);
  }

  const artifactPath = await download.path();
  if (!artifactPath) {
    throw new Error(`No artifact path available for ${suggested}`);
  }

  try {
    fs.renameSync(artifactPath, outputPath);
    log(`Download completed: ${outputPath}`);
    return;
  } catch (error) {
    log(`Rename failed for ${suggested}; falling back to saveAs (${error.message})`);
  }

  await download.saveAs(outputPath);
  log(`Download completed: ${outputPath}`);
}

async function solveChallenge(page) {
  const title = await page.title();
  if (!title.includes("Human Verification")) {
    return false;
  }
  await page.waitForSelector("#moveArea", { timeout: 60_000 });
  const mathText = await page.locator("#mathQuestion").innerText();
  const colorText = await page
    .locator("#colorMatch")
    .evaluate((el) => getComputedStyle(el).color);
  const match = mathText.match(/What is (\d+) \+ (\d+)\?/);
  if (!match) {
    throw new Error(`Unexpected challenge text: ${mathText}`);
  }
  const mathAnswer = Number.parseInt(match[1], 10) + Number.parseInt(match[2], 10);
  const colorMap = {
    "rgb(255, 0, 0)": "red",
    "rgb(0, 128, 0)": "green",
    "rgb(0, 0, 255)": "blue",
    "rgb(255, 255, 0)": "yellow",
  };
  const colorAnswer = colorMap[colorText];
  if (!colorAnswer) {
    throw new Error(`Unexpected challenge color: ${colorText}`);
  }
  const box = await page.locator("#moveArea").boundingBox();
  if (!box) {
    throw new Error("Challenge move area missing");
  }
  for (let i = 0; i < 80; i += 1) {
    await page.mouse.move(box.x + 5 + (i % 200), box.y + 5 + ((i * 7) % 80));
  }
  await page.waitForTimeout(5_200);
  await page.fill("#answerInput", `${mathAnswer} ${colorAnswer}`);
  await page.click("#submitBtn");
  return true;
}

const browser = await chromium.launch({
  headless: true,
  executablePath: chromePath,
  downloadsPath: stagingDir,
});

try {
  const context = await browser.newContext({ acceptDownloads: true });
  const loginPage = await context.newPage();

  log("Opening RealMe login");
  await loginPage.goto(
    "https://geodata.nzpam.govt.nz/saml/login?sso=true&scope=RealMe&came_from=/survey/1994808920",
    { waitUntil: "load", timeout: 60_000 }
  );
  await loginPage.waitForTimeout(3_000);
  await loginPage.locator("#signInName").fill(username);
  await loginPage.locator("#password").fill(password);
  await loginPage.getByRole("button", { name: /^Log in$/ }).click();
  await loginPage.waitForURL(/geodata\.nzpam\.govt\.nz\//, { timeout: 60_000 });
  await loginPage.waitForTimeout(3_000);
  log("Authenticated to NZP&M");
  log(
    `Selected ${selectedTargets.length} file(s) from batch=${batchName} startIndex=${startIndex} maxFiles=${maxFiles || "all"}`
  );

  for (const target of selectedTargets) {
    log(`Preparing download: ${target.label}`);
    const page = await context.newPage();
    try {
      await page.goto(target.url, {
        waitUntil: "domcontentloaded",
        timeout: 60_000,
      }).catch(() => {});

      let challengeCount = 0;
      while (true) {
        const title = await page.title();
        if (title.includes("Human Verification")) {
          challengeCount += 1;
          log(`Solving download challenge (${challengeCount}) for ${target.label}`);
          const downloadPromise = page
            .waitForEvent("download", { timeout: 120_000 })
            .catch(() => null);
          await solveChallenge(page);
          const download = await downloadPromise;
          if (download) {
            const suggested = download.suggestedFilename();
            log(`Download started: ${suggested}`);
            await finalizeDownload(download, target.outputDir);
            break;
          }
          await page.waitForTimeout(3_000);
          continue;
        }

        const download = await page
          .waitForEvent("download", { timeout: 10_000 })
          .catch(() => null);
        if (download) {
          const suggested = download.suggestedFilename();
          log(`Download started without challenge: ${suggested}`);
          await finalizeDownload(download, target.outputDir);
          break;
        }

        const bodyText = await page.locator("body").innerText().catch(() => "");
        throw new Error(
          `No download event for ${target.label}; title=${title}; body=${bodyText.slice(0, 500)}`
        );
      }
    } finally {
      await page.close().catch(() => {});
    }
  }

  log("All selected downloads completed");
} finally {
  await browser.close();
}
