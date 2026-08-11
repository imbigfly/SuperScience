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
  await expect(page.getByTestId("start-exploration")).toHaveCount(0);
  await expect(page.locator(".msg-branch-btn").last()).toBeVisible();

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

test("starting a new exploration is hidden while the feature is incomplete", async ({ page }) => {
  await enterExplorationProject(page);
  await expect(page.getByTestId("start-exploration")).toHaveCount(0);
});

test("a new conversation remains available while an exploration is active", async ({ page }) => {
  await enterExplorationProject(page);
  await page
    .getByTestId("sidebar-explorations")
    .locator('[data-exploration-id="exploration-a"]')
    .click();
  await expect(page.getByTestId("exploration-banner")).toContainText("Exploration A");

  await page.getByRole("button", { name: "New session", exact: true }).click();

  await expect(page.locator("#composer-input")).toBeEnabled();
  await expect(page.getByTestId("mainline-exploration-banner")).toContainText(
    "Other conversations remain available with read-only project tools",
  );
});

test("user messages offer the mature branch flow from the context menu", async ({ page }) => {
  await page.goto("/?mockExplorations=1&mockHistoricalExploration=1");
  await page.locator(".proj-card-main").first().click();
  await page.locator('[data-session-id="exploration-mainline"]').click();

  const userMessage = page.locator(".user-bubble[data-branch-ui-index]").first();
  await userMessage.click({ button: "right" });
  const branch = page.getByRole("button", { name: "Branch to new conversation", exact: true });
  await expect(branch).toBeVisible();
  await expect(page.getByRole("button", { name: "Start exploration", exact: true })).toHaveCount(0);
  await branch.click();
  await expect(page.locator("#composer-input")).toHaveValue("First method");
  await expect(page.getByText("Legacy method", { exact: true })).toHaveCount(0);
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
