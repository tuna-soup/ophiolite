#!/usr/bin/env node

/*
  Browser-assisted NZP&M catalogue probe.

  This is intentionally a validation helper, not a production downloader.
  It uses playwright-core with a local Chrome/Chromium binary to:

  1. Clear the public human-verification challenge.
  2. Open the public dataset search page.
  3. Enumerate visible survey result links.
  4. Open a few survey pages and list resource families / file sizes.

  Expected setup:

    npm install --prefix /tmp/nzpam-probe playwright-core
    export PLAYWRIGHT_CHROME_BIN="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    export PLAYWRIGHT_CORE_ROOT="/tmp/nzpam-probe/node_modules"
    node scripts/validation/nzpam_catalog_browser_probe.mjs

  Optional environment variables:

    NZPAM_QUERY         default: seismic
    NZPAM_DIMENSION     default: 3D
    NZPAM_FORMAT        optional, e.g. SGY
    NZPAM_LIMIT         default: 3
*/

import process from "node:process";

let chromium;
try {
  ({ chromium } = await import("playwright-core"));
} catch (error) {
  try {
    const explicitRoot = process.env.PLAYWRIGHT_CORE_ROOT;
    if (!explicitRoot) {
      throw error;
    }
    ({ chromium } = await import(`${explicitRoot}/playwright-core/index.mjs`));
  } catch {
    console.error(
      "Missing dependency: playwright-core. Install it first, for example:\n" +
        "  npm install --prefix /tmp/nzpam-probe playwright-core\n" +
        "Then run this script with PLAYWRIGHT_CORE_ROOT=/tmp/nzpam-probe/node_modules"
    );
    process.exit(2);
  }
}

const chromePath =
  process.env.PLAYWRIGHT_CHROME_BIN ||
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const query = process.env.NZPAM_QUERY || "seismic";
const dimension = process.env.NZPAM_DIMENSION || "3D";
const format = process.env.NZPAM_FORMAT || "";
const limit = Number.parseInt(process.env.NZPAM_LIMIT || "3", 10);

function buildSearchUrl() {
  const url = new URL("https://geodata.nzpam.govt.nz/dataset/");
  url.searchParams.set("q", query);
  url.searchParams.set("dimension", dimension);
  if (format.trim()) {
    url.searchParams.set("res_format", format.trim());
  }
  return url.toString();
}

async function clearHumanVerification(page) {
  const title = await page.title();
  if (!title.includes("Human Verification")) {
    return false;
  }

  await page.waitForSelector("#moveArea", { timeout: 60_000 });
  const mathText = await page.locator("#mathQuestion").innerText();
  const cssColor = await page
    .locator("#colorMatch")
    .evaluate((el) => getComputedStyle(el).color);

  const match = mathText.match(/What is (\d+) \+ (\d+)\?/);
  if (!match) {
    throw new Error(`Unexpected math challenge text: ${mathText}`);
  }

  const colorMap = new Map([
    ["rgb(255, 0, 0)", "red"],
    ["rgb(0, 128, 0)", "green"],
    ["rgb(0, 0, 255)", "blue"],
    ["rgb(255, 255, 0)", "yellow"],
  ]);
  const colorAnswer = colorMap.get(cssColor);
  if (!colorAnswer) {
    throw new Error(`Unexpected challenge color: ${cssColor}`);
  }

  const mathAnswer = Number.parseInt(match[1], 10) + Number.parseInt(match[2], 10);
  const box = await page.locator("#moveArea").boundingBox();
  if (!box) {
    throw new Error("Challenge move area has no bounding box");
  }

  for (let i = 0; i < 80; i += 1) {
    const x = box.x + 5 + (i % Math.max(20, Math.floor(box.width) - 10));
    const y = box.y + 5 + ((i * 7) % Math.max(20, Math.floor(box.height) - 10));
    await page.mouse.move(x, y);
  }

  await page.waitForTimeout(5_200);
  await page.fill("#answerInput", `${mathAnswer} ${colorAnswer}`);
  await page.click("#submitBtn");
  await page.waitForLoadState("domcontentloaded", { timeout: 60_000 });
  return true;
}

function parseDatasetCards(cards) {
  return cards.map((card) => ({
    title: card.title,
    href: card.href,
    permit: card.permit,
    formats: card.formats,
    dataType: card.dataType,
  }));
}

async function collectSearchResults(page) {
  const resultCountText = await page
    .locator("body")
    .innerText()
    .then((text) => text.match(/(\d+)\s+datasets found for "([^"]+)"/)?.[0] || null);

  const cards = await page.locator(".dataset-content").evaluateAll((els) =>
    els.map((el) => {
      const titleLink = el.querySelector("h3 a, h2 a");
      const text = (el.textContent || "").split("\n").map((v) => v.trim()).filter(Boolean);
      const formatsIndex = text.indexOf("Formats");
      const dataTypeIndex = text.indexOf("Data Type");
      const permitIndex = text.indexOf("Permit");
      return {
        title: titleLink ? titleLink.textContent.trim() : text[0] || null,
        href: titleLink ? titleLink.href : null,
        permit:
          permitIndex >= 0 && permitIndex + 1 < text.length ? text[permitIndex + 1] : null,
        formats:
          formatsIndex >= 0 && formatsIndex + 1 < text.length ? text[formatsIndex + 1] : null,
        dataType:
          dataTypeIndex >= 0 && dataTypeIndex + 1 < text.length ? text[dataTypeIndex + 1] : null,
      };
    })
  );

  return {
    resultCountText,
    cards: parseDatasetCards(cards).filter((card) => card.href),
  };
}

async function collectSurveyResources(context, url) {
  const page = await context.newPage();
  try {
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 60_000 });
    const title = await page.locator("h1").first().innerText();
    const resources = await page.locator("a").evaluateAll((els) =>
      els
        .map((a) => ({
          text: (a.textContent || "").trim().replace(/\s+/g, " "),
          href: a.href,
        }))
        .filter((entry) =>
          /download|stack|velocity|angle|azimuth|epsilon|delta|phi|time shift|segy|sgy|segd|zip/i.test(
            `${entry.text} ${entry.href}`
          )
        )
        .filter((entry) => entry.text)
    );

    return {
      url,
      title,
      resources: resources.slice(0, 40),
    };
  } finally {
    await page.close();
  }
}

const browser = await chromium.launch({
  headless: true,
  executablePath: chromePath,
});

try {
  const context = await browser.newContext();
  const page = await context.newPage();
  const searchUrl = buildSearchUrl();
  await page.goto(searchUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
  const challengeCleared = await clearHumanVerification(page);
  const results = await collectSearchResults(page);
  const surveys = [];
  for (const card of results.cards.slice(0, Math.max(0, limit))) {
    surveys.push(await collectSurveyResources(context, card.href));
  }

  console.log(
    JSON.stringify(
      {
        searchUrl,
        challengeCleared,
        resultCountText: results.resultCountText,
        topCards: results.cards.slice(0, 20),
        surveyedResources: surveys,
      },
      null,
      2
    )
  );
} finally {
  await browser.close();
}
