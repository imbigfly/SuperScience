import { expect, test, type Page } from "@playwright/test";
import { tauriMock } from "./mock-tauri";

async function enterExplorationProject(page: Page) {
  await page.goto("/?mockExplorations=1");
  await page.locator(".proj-card-main").first().click();
  const mainline = page.locator('[data-session-id="exploration-mainline"]');
  await expect(mainline).toBeVisible();
  await mainline.click();
  await expect(page.getByText("Mainline result")).toBeVisible();
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(tauriMock);
});

test("exploration sidebar, banners, diff tabs, and Escape stack remain distinct from Branch", async ({ page }) => {
  await enterExplorationProject(page);

  const group = page.getByTestId("sidebar-explorations");
  await expect(group).toBeVisible();
  await expect(group.locator(".side-exploration")).toHaveCount(2);
  await expect(page.getByTestId("start-exploration")).toBeVisible();
  await expect(page.getByRole("button", { name: "Branch", exact: true })).toBeVisible();

  await group.locator('[data-exploration-id="exploration-a"]').click();
  await expect(page.getByText("Exploration A result")).toBeVisible();
  await expect(page.getByTestId("exploration-banner")).toContainText("Exploration A");

  await page.getByTestId("exploration-banner").getByRole("button", { name: "View diff" }).click();
  const diff = page.getByTestId("exploration-diff-overlay");
  await expect(diff).toBeVisible();
  await expect(diff.getByRole("tab")).toHaveCount(5);
  await diff.getByRole("tab", { name: /Artifacts/ }).click();
  await expect(page.getByTestId("exploration-diff-body")).toContainText("exploration-a/result");
  await diff.getByRole("button", { name: "Set as mainline" }).click();
  await expect(page.getByTestId("exploration-confirm-overlay")).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.getByTestId("exploration-confirm-overlay")).toBeHidden();
  await expect(diff).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(diff).toBeHidden();
});

test("latest completed turn creates an isolated exploration and freezes mainline", async ({ page }) => {
  await enterExplorationProject(page);

  await page.getByTestId("start-exploration").click();
  const start = page.getByTestId("exploration-start-overlay");
  await expect(start).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(start).toBeHidden();

  await page.getByTestId("start-exploration").click();
  await page.getByTestId("exploration-name").fill("Alternative normalization");
  await page.getByTestId("exploration-create").click();
  await expect(page.getByTestId("exploration-banner")).toContainText("Alternative normalization");
  await expect(page.getByTestId("sidebar-explorations").locator(".side-exploration")).toHaveCount(3);

  await page.locator('[data-session-id="exploration-mainline"]').click();
  await expect(page.getByTestId("mainline-exploration-banner")).toContainText("3 active explorations");
  await expect(page.getByTestId("mainline-exploration-banner")).toContainText("Mainline is frozen");
  await expect(page.locator("#composer-input")).toBeDisabled();
  await expect(page.locator("#composer-input")).toHaveAttribute("placeholder", /mainline frozen/i);
  await expect(page.locator("button.send")).toBeDisabled();
});

test("only the current completed turn can start an exploration", async ({ page }) => {
  await page.goto("/?mockExplorations=1&mockHistoricalExploration=1");
  await page.locator(".proj-card-main").first().click();
  await page.locator('[data-session-id="exploration-mainline"]').click();

  const actions = page.getByTestId("start-exploration");
  await expect(actions).toHaveCount(3);
  await expect(actions.nth(0)).toBeDisabled();
  await expect(actions.nth(1)).toBeDisabled();
  await expect(actions.nth(0)).toHaveAttribute("title", /current completed turn/i);
  await expect(actions.nth(1)).toHaveAttribute("title", /current completed turn/i);
  await expect(actions.nth(2)).toBeEnabled();

  await actions.nth(2).click();
  await page.getByTestId("exploration-name").fill("From latest turn");
  await page.getByTestId("exploration-create").click();
  await expect.poll(async () => page.evaluate(() => (window as any).__startExplorationCalls)).toEqual([
    { sourceFrameId: "exploration-mainline", turnIndex: 2, name: "From latest turn" },
  ]);
});

test("promotion adopts one exploration, archives its sibling, and discard leaves mainline intact", async ({ page }) => {
  await enterExplorationProject(page);
  const group = page.getByTestId("sidebar-explorations");

  await group.locator('[data-exploration-id="exploration-a"]').click();
  await page.getByTestId("exploration-banner").getByRole("button", { name: "View diff" }).click();
  await page.getByTestId("exploration-diff-overlay").getByRole("button", { name: "Set as mainline" }).click();
  await page.getByTestId("exploration-confirm-action").click();

  await expect(page.getByText("Exploration A result")).toBeVisible();
  await expect(page.getByTestId("mainline-exploration-banner")).toBeHidden();
  await expect(page.locator("#composer-input")).toBeEnabled();
  const adoptedGroup = page.getByTestId("sidebar-explorations");
  await expect(adoptedGroup.locator('[data-exploration-id="exploration-b"]')).toHaveAttribute("data-exploration-status", "archived");
  await adoptedGroup.locator('[data-exploration-id="exploration-b"]').click();
  await expect(page.getByText("Exploration B result")).toBeVisible();
  await expect(page.locator("#composer-input")).toBeDisabled();
  await page.getByTestId("exploration-banner").getByRole("button", { name: "View diff" }).click();
  await expect(page.getByTestId("exploration-promotion-blocked")).toContainText("mainline no longer matches");
  await expect(page.getByTestId("exploration-promote")).toBeDisabled();

  await page.getByRole("button", { name: "Discard", exact: true }).last().click();
  await expect(page.getByTestId("exploration-confirm-overlay")).toBeVisible();
  await page.getByTestId("exploration-confirm-action").click();
  await expect(page.getByText("Exploration A result")).toBeVisible();
  await expect(page.locator('[data-exploration-id="exploration-b"]')).toHaveAttribute("data-exploration-status", "discarded");
});
