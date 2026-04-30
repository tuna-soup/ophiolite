import { expect, test, type Page } from "@playwright/test";

test("svelte playground seismic wiggle renderer stays visually stable", async ({ page }) => {
  await page.goto("http://127.0.0.1:4174/?mode=playground#/seismic", { waitUntil: "networkidle" });
  await waitForPlaygroundSeismicReady(page);

  await page.locator("select").first().selectOption("local-webgl");
  await page.getByRole("button", { name: "Switch To Wiggle" }).click();
  await expect(page.getByText("render mode: wiggle")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("base renderer requested: local-webgl")).toBeVisible({ timeout: 15_000 });

  const seismicViewer = page.locator(".viewer-seismic").first();
  await expect(seismicViewer).toHaveScreenshot("svelte-playground-seismic-wiggle-local-webgl.png", {
    timeout: 15_000,
    maxDiffPixelRatio: 0.02
  });
});

async function waitForPlaygroundSeismicReady(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { name: "Seismic Section Wrapper" })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByRole("button", { name: "Switch To Wiggle" })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("section loaded: yes")).toBeVisible({ timeout: 15_000 });
}
