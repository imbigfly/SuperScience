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

async function lastInvokeArgs(page: Page, cmd: string) {
  return page.evaluate((name) => {
    const calls = ((window as any).__skillInvokeLog ?? []).filter((call: any) => call.cmd === name);
    const args = calls.at(-1)?.args;
    return args instanceof Map ? Object.fromEntries(args) : (args ?? null);
  }, cmd);
}

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

test("conversation branches have a distinct icon and branch-only context actions", async ({ page }) => {
  await page.goto("/?mockExplorations=1&mockBranches=1");
  await page.locator(".proj-card-main").first().click();

  const branch = page.locator('[data-session-id="conversation-branch"]');
  const exploration = page.locator('[data-exploration-id="exploration-a"]');
  await expect(branch.locator(".session-branch-icon svg")).toHaveCount(1);
  await expect(exploration.locator(".exploration-kind-icon svg")).toHaveCount(1);
  expect(await branch.locator(".session-branch-icon").innerHTML())
    .not.toBe(await exploration.locator(".exploration-kind-icon").innerHTML());

  await branch.click({ button: "right" });
  await expect(page.getByRole("button", { name: "Compare branches", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Make independent", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Delete branch", exact: true })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.locator(".ctx-menu")).toBeHidden();
  await expect(branch).toBeVisible();

  await branch.click({ button: "right" });
  await page.getByRole("button", { name: "Compare branches", exact: true }).click();
  const comparison = page.getByTestId("branch-comparison-overlay");
  await expect(comparison).toBeVisible();
  await expect(comparison.getByTestId("branch-candidate")).toHaveCount(2);
  await expect(comparison).toContainText("2 shared messages before divergence");
  await page.keyboard.press("Escape");
  await expect(comparison).toBeHidden();
  await expect(branch).toBeVisible();

  const main = page.locator('[data-session-id="exploration-mainline"]');
  await main.click({ button: "right" });
  await expect(page.getByRole("button", { name: "Compare branches", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Make independent", exact: true })).toHaveCount(0);
});

test("main and sibling branches compare together and converge through one selected path", async ({ page }) => {
  await page.goto("/?mockBranches=1");
  await page.locator(".proj-card-main").first().click();

  const main = page.locator('[data-session-id="conversation-main"]');
  const branch = page.locator('[data-session-id="conversation-branch"]');
  await main.click({ button: "right" });
  await page.getByRole("button", { name: "Compare branches", exact: true }).click();

  const comparison = page.getByTestId("branch-comparison-overlay");
  await expect(comparison.getByTestId("branch-candidate")).toHaveCount(4);
  await expect(comparison.getByTestId("branch-ai-analysis")).toContainText("Method B is more robust");
  await comparison.locator('[data-session-id="conversation-branch"]').click();
  await comparison.getByTestId("branch-converge").click();
  const confirm = page.getByTestId("branch-convergence-confirm");
  await expect(confirm).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(confirm).toBeHidden();
  await expect(comparison).toBeVisible();
  expect(await lastInvokeArgs(page, "converge_session_branches")).toBeNull();

  await comparison.getByTestId("branch-converge").click();
  await confirm.getByTestId("branch-converge-action").click();
  await expect.poll(() => lastInvokeArgs(page, "converge_session_branches")).toMatchObject({
    selectedSessionId: "conversation-branch",
    expectedGuardHash: "mock-branch-guard",
  });
  await expect(comparison).toBeHidden();
  await expect(main).toHaveAttribute("data-session-title", "alternate analysis");
  await expect(page.locator('[data-session-branch="true"]')).toHaveCount(0);
});

test("a branch can leave its family without changing its transcript", async ({ page }) => {
  await page.goto("/?mockBranches=1");
  await page.locator(".proj-card-main").first().click();

  const branch = page.locator('[data-session-id="conversation-branch-b"]');
  await branch.click({ button: "right" });
  await page.getByRole("button", { name: "Make independent", exact: true }).click();
  await expect.poll(() => lastInvokeArgs(page, "detach_session_branch")).toMatchObject({
    id: "conversation-branch-b",
  });
  await expect(branch).toHaveAttribute("data-session-branch", "false");
  await expect(branch).toHaveAttribute("data-session-family", "false");
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
