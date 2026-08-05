// Chat scroll follow (mirrors web-dist ConversationView pinned-at-bottom behavior).

const hooks = new Map();

// ponytail: single chat scroller, so the jump pill id is a constant.
const JUMP_PILL_ID = "chat-jump-pill";

function lastUserRow(root) {
  const rows = root.querySelectorAll("[data-user-index]");
  return rows.length ? rows[rows.length - 1] : null;
}

function bottomGap(el) {
  return Math.max(0, el.scrollHeight - el.clientHeight - el.scrollTop);
}

function atBottom(el, eps = 2) {
  return bottomGap(el) <= eps;
}

function snapBottom(el) {
  const max = el.scrollHeight - el.clientHeight;
  if (max - el.scrollTop > 2) el.scrollTop = max;
}

/** @param {string} scrollerId @param {string} contentId */
export function attach_chat_scroll(scrollerId, contentId) {
  const scroller = document.getElementById(scrollerId);
  const content = document.getElementById(contentId);
  if (!scroller || !content || hooks.has(scrollerId)) return;

  let follow = true;
  let lastHeight = content.scrollHeight;
  let lastScrollTop = scroller.scrollTop;
  // Only scroll-UP gestures unfollow. The older "any recent user scroll"
  // window treated reflow clamps right after scrolling back to the bottom as
  // another unfollow, so the view bounced away from the tail mid-stream and
  // could not stay pinned there.
  let lastUserScrollUp = -Infinity;
  // True while the user is actively dragging the scrollbar / touching the
  // scroller. Reflow clamps must NOT use a lingering "recent gesture" window —
  // after rolling back to the bottom, streaming rebuilds shrink scrollTop and
  // that looked identical to another scroll-up, bouncing the pin off again.
  let dragActive = false;
  const markScrollUp = () => {
    lastUserScrollUp = performance.now();
    follow = false;
  };
  const setDrag = (active) => {
    dragActive = active;
  };

  // Floating "Your last message" jump pill: visible only when the last user
  // turn is off-screen and the view is scrolled away from the bottom. Class
  // toggle on a static element — no reactive rebuild involved.
  const syncPill = () => {
    const pill = document.getElementById(JUMP_PILL_ID);
    if (!pill) return;
    let show = false;
    const row = lastUserRow(content);
    if (row && bottomGap(scroller) > 48) {
      const view = scroller.getBoundingClientRect();
      const top = row.getBoundingClientRect().top;
      show = top < view.top - 4 || top > view.bottom - 4;
    }
    pill.classList.toggle("visible", show);
  };

  const syncFollow = () => {
    syncPill();
    if (atBottom(scroller)) {
      // Reaching the live edge restores the pin and clears any scroll-up
      // window so a streaming reflow immediately afterward cannot bounce us
      // away again.
      follow = true;
      lastUserScrollUp = -Infinity;
      return;
    }
    // Intentional scroll-up: release the pin so the user can read history
    // while the turn keeps streaming below.
    if (performance.now() - lastUserScrollUp < 500) {
      follow = false;
      return;
    }
    // Reflow-driven clamp while following (streaming rebuilds shrink the
    // thread for a beat, yanking scrollTop up). Scroll events fire before
    // paint, so an instant snap here means the clamped position is never
    // painted — without it the view visibly bounces on every thinking delta.
    if (follow) snapBottom(scroller);
  };

  const onGrowth = () => {
    const h = content.scrollHeight;
    const grew = h > lastHeight;
    lastHeight = h;
    if (follow && grew) snapBottom(scroller);
    lastScrollTop = scroller.scrollTop;
    syncFollow();
  };

  scroller.style.overflowAnchor = "none";
  scroller.addEventListener(
    "scroll",
    () => {
      const top = scroller.scrollTop;
      // Only count scrollTop decreases while a drag/touch is held. Wheel-up is
      // handled on the wheel listener; reflow clamps never hold dragActive.
      if (dragActive && top + 1 < lastScrollTop) {
        markScrollUp();
      } else if (dragActive && top > lastScrollTop + 1) {
        lastUserScrollUp = -Infinity;
      }
      lastScrollTop = top;
      syncFollow();
    },
    { passive: true },
  );
  scroller.addEventListener(
    "wheel",
    (e) => {
      if (e.deltaY < 0) {
        markScrollUp();
      } else {
        lastUserScrollUp = -Infinity;
        if (atBottom(scroller)) follow = true;
      }
    },
    { passive: true },
  );
  scroller.addEventListener("pointerdown", () => setDrag(true), { passive: true });
  scroller.addEventListener("touchstart", () => setDrag(true), { passive: true });
  // pointer/touch may release outside the scroller
  window.addEventListener("pointerup", () => setDrag(false), { passive: true });
  window.addEventListener("pointercancel", () => setDrag(false), { passive: true });
  window.addEventListener("touchend", () => setDrag(false), { passive: true });
  window.addEventListener("touchcancel", () => setDrag(false), { passive: true });
  scroller.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "PageUp" || e.key === "ArrowUp" || e.key === "Home") {
        markScrollUp();
      } else if (e.key === "PageDown" || e.key === "ArrowDown" || e.key === "End") {
        lastUserScrollUp = -Infinity;
        if (atBottom(scroller)) follow = true;
      }
    },
    { passive: true },
  );

  const ro = new ResizeObserver(() => onGrowth());
  ro.observe(content);

  hooks.set(scrollerId, {
    ro,
    onGrowth,
    unfollow: () => {
      follow = false;
      lastHeight = content.scrollHeight;
      lastScrollTop = scroller.scrollTop;
    },
    snap: () => {
      follow = true;
      snapBottom(scroller);
      lastHeight = content.scrollHeight;
      lastScrollTop = scroller.scrollTop;
    },
    /** @returns {{ follow: boolean, gap: number }} */
    debugState: () => ({
      follow,
      gap: bottomGap(scroller),
    }),
  });

  follow = true;
  snapBottom(scroller);
  lastScrollTop = scroller.scrollTop;
}

/** @param {string} scrollerId */
export function notify_chat_scroll(scrollerId) {
  const hook = hooks.get(scrollerId);
  if (!hook) return;
  requestAnimationFrame(() => {
    requestAnimationFrame(() => hook.onGrowth());
  });
}

/** @param {string} scrollerId */
export function force_chat_scroll_bottom(scrollerId) {
  const hook = hooks.get(scrollerId);
  if (hook) {
    hook.snap();
    return;
  }
  const scroller = document.getElementById(scrollerId);
  if (scroller) snapBottom(scroller);
}

/** @param {string} scrollerId @param {string} contentId */
export function preserve_chat_scroll_on_prepend(scrollerId, contentId) {
  const scroller = document.getElementById(scrollerId);
  const content = document.getElementById(contentId);
  if (!scroller || !content) return;
  const oldHeight = content.scrollHeight;
  const oldTop = scroller.scrollTop;
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      scroller.scrollTop = oldTop + content.scrollHeight - oldHeight;
    });
  });
}

/** Scroll the latest user turn into view (the floating jump pill).
 * @param {string} scrollerId */
export function jump_chat_scroll_last_user(scrollerId) {
  const scroller = document.getElementById(scrollerId);
  const target = scroller && lastUserRow(scroller);
  if (!target) return;
  hooks.get(scrollerId)?.unfollow();
  target.scrollIntoView({ block: "start" });
}

/** @param {string} scrollerId @param {string} selector */
export function jump_chat_scroll(scrollerId, selector) {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      const target = document.querySelector(selector);
      if (!target) return;
      hooks.get(scrollerId)?.unfollow();
      target.scrollIntoView({ block: "start" });
    });
  });
}
