import { expect, test, type Page } from "@playwright/test";

test("public docs hero and getting-started sections stay visually stable", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await waitForDocsReady(page);

  await expect(page.locator("[data-testid='hero']")).toHaveScreenshot("public-docs-hero.png", { timeout: 15_000 });
  await expect(page.locator("[data-testid='getting-started']")).toHaveScreenshot("public-docs-getting-started.png", {
    timeout: 15_000
  });
});

test("public docs launch-family sections stay visually stable", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await waitForDocsReady(page);

  await expect(page.locator("[data-testid='seismic-showcase']")).toHaveScreenshot("public-docs-seismic-showcase.png", {
    timeout: 15_000
  });
  await expect(page.locator("[data-testid='family-previews']")).toHaveScreenshot("public-docs-family-previews.png", {
    timeout: 15_000
  });
});

test("public docs support-tier and docs-map sections stay visually stable", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await waitForDocsReady(page);

  await expect(page.locator("#support-tiers")).toHaveScreenshot("public-docs-support-tiers.png", { timeout: 15_000 });
  await expect(page.locator("[data-testid='examples']")).toHaveScreenshot("public-docs-examples.png", {
    timeout: 15_000
  });
});

test("public docs traceboost and manifest sections stay visually stable", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await waitForDocsReady(page);

  await expect(page.locator("#traceboost")).toHaveScreenshot("public-docs-traceboost.png", {
    timeout: 15_000,
    maxDiffPixelRatio: 0.02
  });
  await expect(page.locator("#manifests")).toHaveScreenshot("public-docs-manifests.png", { timeout: 15_000 });
});

test("public docs navigation anchors remain visible", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await waitForDocsReady(page);

  await expect(page.locator("#support-tiers")).toBeVisible();
  await expect(page.locator("[data-testid='examples']")).toBeVisible();
  await expect(page.locator("#traceboost")).toBeVisible();
  await expect(page.locator("#manifests")).toBeVisible();
});

async function waitForDocsReady(page: Page): Promise<void> {
  await expect(page.locator("[data-testid='hero']")).toBeVisible({ timeout: 15_000 });
  await expect(page.locator("[data-testid='seismic-showcase']")).toBeVisible({ timeout: 15_000 });
  await expect(page.locator("#manifests")).toBeVisible({ timeout: 15_000 });
}
