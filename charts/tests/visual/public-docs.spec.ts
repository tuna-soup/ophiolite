import { expect, test } from "@playwright/test";

test("public docs hero and getting-started sections stay visually stable", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });

  await expect(page.locator("[data-testid='hero']")).toHaveScreenshot("public-docs-hero.png");
  await expect(page.locator("[data-testid='getting-started']")).toHaveScreenshot("public-docs-getting-started.png");
});

test("public docs launch-family sections stay visually stable", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });

  await expect(page.locator("[data-testid='seismic-showcase']")).toHaveScreenshot("public-docs-seismic-showcase.png");
  await expect(page.locator("[data-testid='family-previews']")).toHaveScreenshot("public-docs-family-previews.png");
});
