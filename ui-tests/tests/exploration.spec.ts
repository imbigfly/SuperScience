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

test("conversation branches appear at their checkpoint and expose merge-back actions", async ({ page }) => {
  await page.goto("/?mockExplorations=1&mockBranches=1");
  await page.locator(".proj-card-main").first().click();

  const branch = page.locator('.sidebar [data-session-id="conversation-branch"]');
  const exploration = page.locator('[data-exploration-id="exploration-a"]');
  await expect(branch.locator(".session-branch-icon svg")).toHaveCount(1);
  await expect(exploration.locator(".exploration-kind-icon svg")).toHaveCount(1);
  expect(await branch.locator(".session-branch-icon").innerHTML())
    .not.toBe(await exploration.locator(".exploration-kind-icon").innerHTML());

  await branch.click({ button: "right" });
  await expect(page.getByRole("button", { name: "Merge back", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Compare branches", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Make independent", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Delete branch", exact: true })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.locator(".ctx-menu")).toBeHidden();
  await expect(branch).toBeVisible();

  await branch.click({ button: "right" });
  await page.getByRole("button", { name: "Merge back", exact: true }).click();
  const merge = page.getByTestId("branch-merge-overlay");
  await expect(merge).toBeVisible();
  await expect(merge.getByTestId("branch-merge-delta")).toContainText("alternate analysis result");
  await expect(merge.locator("textarea")).toHaveValue(/completed its focused analysis/);
  await page.keyboard.press("Escape");
  await expect(merge).toBeHidden();
  await expect(branch).toBeVisible();

  const main = page.locator('.sidebar [data-session-id="exploration-mainline"]');
  await main.click();
  const inlineBranch = page.getByTestId("message-branch-link");
  await expect(inlineBranch).toHaveCount(1);
  await expect(inlineBranch).toContainText("alternate analysis");
});

test("an edited branch-only summary appends to the current main tail and keeps the branch", async ({ page }) => {
  await page.goto("/?mockBranches=1");
  await page.locator(".proj-card-main").first().click();

  const branch = page.locator('.sidebar [data-session-id="conversation-branch"]');
  await branch.click({ button: "right" });
  await page.getByRole("button", { name: "Merge back", exact: true }).click();
  const merge = page.getByTestId("branch-merge-overlay");
  await merge.locator("textarea").fill("Edited branch result ready for main.");
  await merge.getByTestId("branch-merge-action").click();
  await expect.poll(() => lastInvokeArgs(page, "merge_session_branch_summary")).toMatchObject({
    id: "conversation-branch",
    expectedGuardHash: "mock-branch-merge-guard",
    summary: "Edited branch result ready for main.",
  });
  await expect(merge).toBeHidden();
  await expect(page.getByText("Main current result", { exact: true })).toBeVisible();
  await expect(page.getByText("Edited branch result ready for main.", { exact: true })).toHaveCount(0);
  const mergedCard = page.getByTestId("branch-merge-card");
  await expect(mergedCard).toContainText("Merged branch result");
  await expect(mergedCard).toContainText("alternate analysis");
  await mergedCard.click();
  const detail = page.getByTestId("branch-merge-detail-overlay");
  await expect(detail).toContainText("Edited branch result ready for main.");
  await page.keyboard.press("Escape");
  await expect(detail).toBeHidden();
  await expect(mergedCard).toBeVisible();
  await expect(branch).toBeVisible();
});

test("branch summaries can be regenerated or revised with explicit guidance", async ({ page }) => {
  await page.goto("/?mockBranches=1");
  await page.locator(".proj-card-main").first().click();

  const branch = page.locator('.sidebar [data-session-id="conversation-branch"]');
  await branch.click({ button: "right" });
  await page.getByRole("button", { name: "Merge back", exact: true }).click();
  const merge = page.getByTestId("branch-merge-overlay");
  const draft = merge.locator("textarea");
  await expect(draft).toHaveValue(/completed its focused analysis/);

  await merge.getByTestId("branch-regenerate").click();
  await expect.poll(() => lastInvokeArgs(page, "summarize_session_branch_merge")).toMatchObject({
    id: "conversation-branch",
    expectedGuardHash: "mock-branch-merge-guard",
  });
  const regenerateArgs = await lastInvokeArgs(page, "summarize_session_branch_merge");
  expect(regenerateArgs.currentVersion).toBeUndefined();
  expect(regenerateArgs.userGuidance).toBeUndefined();

  await draft.fill("Current edited version for main.");
  await merge.getByTestId("branch-guided-generate").click();
  const guidance = page.getByTestId("branch-guidance-overlay");
  await expect(guidance).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(guidance).toBeHidden();
  await expect(merge).toBeVisible();
  await expect(draft).toHaveValue("Current edited version for main.");

  await merge.getByTestId("branch-guided-generate").click();
  await guidance.locator("textarea").fill("Emphasize the evidence and shorten the conclusion.");
  await guidance.getByTestId("branch-guidance-action").click();
  await expect.poll(() => lastInvokeArgs(page, "summarize_session_branch_merge")).toMatchObject({
    id: "conversation-branch",
    expectedGuardHash: "mock-branch-merge-guard",
    currentVersion: "Current edited version for main.",
    userGuidance: "Emphasize the evidence and shorten the conclusion.",
  });
  await expect(guidance).toBeHidden();
  await expect(draft).toHaveValue(
    "Guided version: Emphasize the evidence and shorten the conclusion.",
  );
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
  await expect(page.locator("#composer-input")).toHaveValue("");
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
