import { expect, test, type Page } from "@playwright/test";
import { tauriMock } from "./mock-tauri";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(tauriMock);
});

async function enterApp(page: Page) {
  await page.goto("/");
  await page.locator(".proj-card-main").first().click();
  await expect(page.locator(".sidebar").getByRole("button", { name: "New session" })).toBeVisible();
}

async function emitTauriEvent(page: Page, event: string, payload: unknown) {
  await expect.poll(() => page.evaluate((name) =>
    Boolean((window as any).__tauriListenerReady?.(name)), event
  )).toBe(true);
  await page.evaluate(({ name, value }) => {
    (window as any).__tauriEmit(name, value);
  }, { name: event, value: payload });
}

async function lastInvokeArgs(page: Page, cmd: string) {
  return page.evaluate((name) => {
    const plain = (value: any): any => {
      if (value instanceof Map) return Object.fromEntries([...value].map(([k, v]) => [k, plain(v)]));
      if (Array.isArray(value)) return value.map(plain);
      if (value && typeof value === "object") return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, plain(v)]));
      return value;
    };
    const calls = ((window as any).__skillInvokeLog ?? []).filter((c: any) => c.cmd === name);
    return plain(calls.at(-1)?.args ?? null);
  }, cmd);
}

async function startLiveRetrievalTurn(page: Page) {
  await page.locator("#composer-input").fill("latest rustc version");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText("Hello from mock wisp-science.")).toBeVisible();
  await expect.poll(() => lastInvokeArgs(page, "send_message")).not.toBeNull();
  const sessionId = String((await lastInvokeArgs(page, "send_message")).sessionId ?? "");
  expect(sessionId).not.toBe("");
  return sessionId;
}

test("disconnected browser retrieval shows a banner that Escape dismisses without moving focus", async ({ page }) => {
  await enterApp(page);
  const sessionId = await startLiveRetrievalTurn(page);

  await emitTauriEvent(page, "agent", {
    kind: "ToolPresentation",
    frame_id: sessionId,
    presentation_kind: "browser_disconnected",
    payload: { code: "browser_extension_disconnected", live_retrieval: false },
  });

  const banner = page.getByTestId("browser-offline-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toContainText("This answer has no live web results");
  await expect(banner).toContainText("based only on the model's existing knowledge");

  await page.keyboard.press("Escape");
  await expect(banner).toBeHidden();
  await expect(page.locator("#composer-input")).toBeVisible();
});

test("browser offline banner stays under Settings in the Escape stack and can retry", async ({ page }) => {
  await enterApp(page);
  const sessionId = await startLiveRetrievalTurn(page);

  await emitTauriEvent(page, "agent", {
    kind: "ToolResult",
    frame_id: sessionId,
    name: "web_scan",
    ok: false,
    content: "real-browser bridge unavailable: browser extension is not connected. WISP_BROWSER_DISCONNECTED",
  });

  const banner = page.getByTestId("browser-offline-banner");
  await expect(banner).toBeVisible();

  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByRole("button", { name: "Back to app" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: "Back to app" })).toHaveCount(0);
  await expect(banner).toBeVisible();

  await banner.getByRole("button", { name: "Retry after connecting" }).click();
  await expect(banner).toBeHidden();
  await expect.poll(() => lastInvokeArgs(page, "send_message")).toMatchObject({
    message: "latest rustc version",
  });
});
