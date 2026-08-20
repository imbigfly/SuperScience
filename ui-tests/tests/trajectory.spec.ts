import { test, expect, type Page } from "@playwright/test";
import { tauriMock } from "./mock-tauri";

// Trajectory (轨迹) tab: the thread area swaps the chat transcript for the
// per-turn event timeline returned by `load_session_trajectory` (mocked in
// mock-tauri.ts with two turns, tool details, usage cells, and stats).

test.beforeEach(async ({ page }) => {
  // Install the Tauri bridge mock before the page's wasm runs.
  await page.addInitScript(tauriMock);
});

async function enterApp(page: Page) {
  await page.goto("/");
  await page.locator(".proj-card-main").first().click();
  await expect(page.locator("#composer-input")).toBeVisible();
}

test("trajectory tab renders turns, expandable tool rows, usage lines, and stats", async ({ page }) => {
  await enterApp(page);
  await page.locator("#composer-input").fill("analyze ESR1");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText("Hello from mock wisp-science.")).toBeVisible({ timeout: 10_000 });

  await page.getByTestId("thread-tab-trajectory").click();
  const view = page.getByTestId("trajectory-view");
  await expect(view).toBeVisible();

  // Turn groups and their headers.
  await expect(view.getByText("Turn 1", { exact: true })).toBeVisible();
  await expect(view.getByText("Turn 2", { exact: true })).toBeVisible();

  // Badges + one-line summaries.
  const toolRow = view.getByTestId("traj-row-tool").first();
  await expect(toolRow).toContainText("TOOL");
  await expect(toolRow).toContainText("python · df.describe()");
  await expect(toolRow).toContainText("3s");
  await expect(view.getByTestId("traj-row-user").first()).toContainText("Analyze the ESR1 dataset");

  // Usage rows stay compact single lines.
  await expect(view.getByText("round 1 · in 12.3k · out 1.4k · cached 75%")).toBeVisible();

  // Tool rows expand to full args JSON + full result.
  await expect(view.getByTestId("traj-detail-input")).toHaveCount(0);
  await toolRow.click();
  await expect(view.getByTestId("traj-detail-input")).toContainText('"code": "df.describe()"');
  await expect(view.getByTestId("traj-detail-output")).toContainText("count  612.0");
  // Click again to collapse.
  await toolRow.click();
  await expect(view.getByTestId("traj-detail-input")).toHaveCount(0);

  // Error rows carry the red accent class.
  await expect(view.getByTestId("traj-row-tool").nth(1)).toHaveClass(/error/);

  // Footer stats line.
  const footer = view.getByTestId("trajectory-footer");
  await expect(footer).toContainText("2 turns · 4 steps");
  await expect(footer).toContainText("LLM 3s · Tools 4s");
  await expect(footer).toContainText("12.5 tok/s");
  await expect(footer).toContainText("cache hit 75%");
  await expect(footer).toContainText("in 27.3k tok · out 2.3k tok");

  // Client-side search filters cells (Turn 1 has no "volcano" cell).
  await view.getByPlaceholder("Search events").fill("volcano");
  await expect(view.getByText("Turn 1", { exact: true })).toHaveCount(0);
  await expect(view.getByText("Turn 2", { exact: true })).toBeVisible();
  await view.getByPlaceholder("Search events").fill("");
  await expect(view.getByText("Turn 1", { exact: true })).toBeVisible();

  // Switching back restores the chat thread.
  await page.getByTestId("thread-tab-chat").click();
  await expect(view).toHaveCount(0);
  await expect(page.getByText("Hello from mock wisp-science.")).toBeVisible();
});

test("trajectory tab shows the empty state when a session has no turns", async ({ page }) => {
  await enterApp(page);
  await page.evaluate(() => {
    (window as any).__trajectorySnapshot = {
      frame_id: "",
      model: null,
      turns: [],
      stats: {
        turns: 0,
        steps: 0,
        llm_ms: 0,
        tool_ms: 0,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        cache_hit_pct: null,
        tokens_per_sec: null,
      },
    };
  });
  await page.getByTestId("thread-tab-trajectory").click();
  await expect(page.getByTestId("trajectory-view")).toContainText("No trajectory yet");
});
