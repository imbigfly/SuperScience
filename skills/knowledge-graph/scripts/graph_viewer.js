(() => {
  const DATA = window.GRAPH_DATA;
  const canvas = document.getElementById("graph");
  const tip = document.getElementById("tip");
  const search = document.getElementById("node-search");
  const ctx = canvas.getContext("2d");
  const palette = ["#3b6d9a", "#c47b3a", "#5a8f5a", "#9a4f6b", "#6b5b95", "#7a6a4a"];
  const MIN_SCALE = 0.15;
  const MAX_SCALE = 8;
  const DRAG_THRESHOLD = 5;

  const nodes = DATA.nodes.map((n, i) => ({
    ...n,
    x: Math.cos(i) * 180,
    y: Math.sin(i) * 180,
    vx: 0,
    vy: 0,
    fixed: false,
  }));
  const byId = Object.fromEntries(nodes.map((n) => [n.id, n]));
  const edges = DATA.edges.filter((e) => byId[e.source] && byId[e.target]);
  const adj = new Map();
  for (const node of nodes) adj.set(node.id, new Set());
  for (const edge of edges) {
    adj.get(edge.source).add(edge.target);
    adj.get(edge.target).add(edge.source);
  }

  let width = 0;
  let height = 0;
  let dpr = 1;
  let cam = { x: 0, y: 0, k: 1 };
  let hover = null;
  let hoverEdge = null;
  let selected = null;
  let query = "";
  let matchIndex = 0;
  let dragNode = null;
  let panning = false;
  let lastPtr = null;
  let ptrMode = null;
  let ptrStart = null;
  let dirty = true;

  function resize() {
    dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    width = Math.max(1, Math.floor(rect.width));
    height = Math.max(1, Math.floor(rect.height));
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);
    dirty = true;
  }

  function radius(node) {
    return 6 + Math.min(14, Math.sqrt(node.degree || 1) * 3);
  }

  function isPinned(node) {
    return node.fixed || node === dragNode;
  }

  function matchesQuery(node) {
    if (!query) return true;
    return node.label.toLowerCase().includes(query);
  }

  function queryMatches() {
    return query ? nodes.filter(matchesQuery) : [];
  }

  function inNeighborhood(node) {
    if (!selected) return true;
    if (node === selected) return true;
    return adj.get(selected.id).has(node.id);
  }

  function nodeActive(node) {
    if (query) return matchesQuery(node);
    return inNeighborhood(node);
  }

  function edgeActive(edge) {
    const a = byId[edge.source];
    const b = byId[edge.target];
    if (query) return matchesQuery(a) || matchesQuery(b);
    if (!selected) return true;
    return a === selected || b === selected;
  }

  function worldFromClient(clientX, clientY) {
    const rect = canvas.getBoundingClientRect();
    const sx = clientX - rect.left;
    const sy = clientY - rect.top;
    return {
      sx,
      sy,
      x: (sx - width / 2 - cam.x) / cam.k,
      y: (sy - height / 2 - cam.y) / cam.k,
    };
  }

  function zoomAt(sx, sy, factor) {
    const wx = (sx - width / 2 - cam.x) / cam.k;
    const wy = (sy - height / 2 - cam.y) / cam.k;
    cam.k = Math.min(MAX_SCALE, Math.max(MIN_SCALE, cam.k * factor));
    cam.x = sx - width / 2 - wx * cam.k;
    cam.y = sy - height / 2 - wy * cam.k;
    dirty = true;
  }

  function resetView() {
    cam = { x: 0, y: 0, k: 1 };
    dirty = true;
  }

  function fitNodes(list) {
    const targets = list.length ? list : nodes;
    if (!targets.length) {
      resetView();
      return;
    }
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const node of targets) {
      const pad = radius(node) + 24;
      minX = Math.min(minX, node.x - pad);
      minY = Math.min(minY, node.y - pad);
      maxX = Math.max(maxX, node.x + pad);
      maxY = Math.max(maxY, node.y + pad);
    }
    const boxW = Math.max(40, maxX - minX);
    const boxH = Math.max(40, maxY - minY);
    const k = Math.min((width - 48) / boxW, (height - 48) / boxH);
    cam.k = Math.min(MAX_SCALE, Math.max(MIN_SCALE, k));
    cam.x = -((minX + maxX) / 2) * cam.k;
    cam.y = -((minY + maxY) / 2) * cam.k;
    dirty = true;
  }

  function focusNode(node) {
    if (!node) return;
    cam.k = Math.max(cam.k, 1.4);
    cam.x = -node.x * cam.k;
    cam.y = -node.y * cam.k;
    dirty = true;
  }

  function step() {
    for (const edge of edges) {
      const a = byId[edge.source];
      const b = byId[edge.target];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dist = Math.max(40, Math.hypot(dx, dy));
      const force = (dist - 110) * 0.008;
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;
      if (!isPinned(a)) {
        a.vx += fx;
        a.vy += fy;
      }
      if (!isPinned(b)) {
        b.vx -= fx;
        b.vy -= fy;
      }
    }
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const a = nodes[i];
        const b = nodes[j];
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist = Math.hypot(dx, dy) || 0.1;
        const force = 420 / (dist * dist);
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;
        if (!isPinned(a)) {
          a.vx -= fx;
          a.vy -= fy;
        }
        if (!isPinned(b)) {
          b.vx += fx;
          b.vy += fy;
        }
      }
    }
    let energy = 0;
    for (const node of nodes) {
      if (isPinned(node)) {
        node.vx = 0;
        node.vy = 0;
        continue;
      }
      node.vx += -node.x * 0.002;
      node.vy += -node.y * 0.002;
      node.vx *= 0.86;
      node.vy *= 0.86;
      node.x += node.vx;
      node.y += node.vy;
      energy = Math.max(energy, Math.abs(node.vx) + Math.abs(node.vy));
    }
    if (energy > 0.02) dirty = true;
    return energy;
  }

  function hitNode(wx, wy) {
    for (let i = nodes.length - 1; i >= 0; i--) {
      const node = nodes[i];
      if (Math.hypot(node.x - wx, node.y - wy) <= radius(node) + 4 / cam.k) {
        return node;
      }
    }
    return null;
  }

  function hitEdge(wx, wy) {
    const tol = 8 / cam.k;
    let best = null;
    let bestDist = tol;
    for (const edge of edges) {
      const a = byId[edge.source];
      const b = byId[edge.target];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const len2 = dx * dx + dy * dy || 1;
      let t = ((wx - a.x) * dx + (wy - a.y) * dy) / len2;
      t = Math.max(0, Math.min(1, t));
      const px = a.x + dx * t;
      const py = a.y + dy * t;
      const dist = Math.hypot(wx - px, wy - py);
      if (dist < bestDist) {
        bestDist = dist;
        best = edge;
      }
    }
    return best;
  }

  function draw() {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);
    ctx.save();
    ctx.translate(width / 2 + cam.x, height / 2 + cam.y);
    ctx.scale(cam.k, cam.k);
    for (const edge of edges) {
      const a = byId[edge.source];
      const b = byId[edge.target];
      const active = edgeActive(edge);
      ctx.beginPath();
      ctx.setLineDash(edge.inferred ? [6, 4] : []);
      ctx.globalAlpha = active ? 1 : 0.12;
      ctx.strokeStyle = edge === hoverEdge ? "#1f1e1c" : edge.inferred ? "#9a8f82" : "#6b6560";
      ctx.lineWidth = (edge === hoverEdge ? 2.2 : 1.2) / cam.k;
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
      if (active && (cam.k >= 0.85 || edge === hoverEdge || selected)) {
        ctx.fillStyle = "#6b6560";
        ctx.font = `${11 / cam.k}px system-ui, sans-serif`;
        ctx.fillText(edge.label, (a.x + b.x) / 2 + 4 / cam.k, (a.y + b.y) / 2);
      }
    }
    ctx.setLineDash([]);
    for (const node of nodes) {
      const active = nodeActive(node);
      const r = radius(node);
      ctx.globalAlpha = active ? 1 : 0.14;
      ctx.beginPath();
      ctx.fillStyle = palette[node.community % palette.length];
      ctx.arc(node.x, node.y, r, 0, Math.PI * 2);
      ctx.fill();
      if (node === selected || node === hover) {
        ctx.strokeStyle = "#1f1e1c";
        ctx.lineWidth = 2 / cam.k;
        ctx.stroke();
      } else if (node.fixed) {
        ctx.strokeStyle = "#1f1e1c";
        ctx.lineWidth = 1.6 / cam.k;
        ctx.stroke();
      }
      ctx.fillStyle = "#1f1e1c";
      ctx.font = `${(node === selected ? 13 : 12) / cam.k}px system-ui, sans-serif`;
      ctx.fillText(node.label, node.x + r + 4 / cam.k, node.y + 4 / cam.k);
    }
    ctx.globalAlpha = 1;
    ctx.restore();
  }

  function showTip(ev, node, edge) {
    if (!node && !edge) {
      tip.style.display = "none";
      return;
    }
    tip.style.display = "block";
    tip.style.left = `${ev.clientX + 12}px`;
    tip.style.top = `${ev.clientY + 12}px`;
    if (node) {
      const ncount = adj.get(node.id).size;
      tip.textContent = `${node.label} · 度数 ${node.degree} · 邻域 ${ncount}${node.fixed ? " · 已固定" : ""}`;
      return;
    }
    tip.textContent = `${edge.source} — ${edge.label} → ${edge.target}${edge.inferred ? " · 推断" : ""}`;
  }

  function setCursor() {
    canvas.classList.toggle("dragging", !!dragNode);
    canvas.classList.toggle("panning", panning);
    canvas.classList.toggle("over-node", !dragNode && !panning && !!hover);
  }

  function applySearch(next, jump) {
    query = next.trim().toLowerCase();
    const matches = queryMatches();
    if (!matches.length) {
      matchIndex = 0;
      dirty = true;
      return;
    }
    if (jump) matchIndex = (matchIndex + 1) % matches.length;
    else matchIndex = Math.min(matchIndex, matches.length - 1);
    selected = matches[matchIndex];
    focusNode(selected);
  }

  canvas.addEventListener(
    "wheel",
    (ev) => {
      ev.preventDefault();
      const { sx, sy } = worldFromClient(ev.clientX, ev.clientY);
      zoomAt(sx, sy, ev.deltaY < 0 ? 1.12 : 1 / 1.12);
    },
    { passive: false },
  );

  canvas.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    const pos = worldFromClient(ev.clientX, ev.clientY);
    hover = hitNode(pos.x, pos.y);
    hoverEdge = hover ? null : hitEdge(pos.x, pos.y);
    ptrStart = { x: ev.clientX, y: ev.clientY, node: hover };
    ptrMode = hover ? "pending" : "pan";
    panning = ptrMode === "pan";
    lastPtr = { x: ev.clientX, y: ev.clientY };
    canvas.setPointerCapture(ev.pointerId);
    setCursor();
    showTip(ev, hover, hoverEdge);
  });

  canvas.addEventListener("pointermove", (ev) => {
    const pos = worldFromClient(ev.clientX, ev.clientY);
    if (ptrMode === "pending" && ptrStart) {
      if (Math.hypot(ev.clientX - ptrStart.x, ev.clientY - ptrStart.y) > DRAG_THRESHOLD) {
        ptrMode = "drag";
        dragNode = ptrStart.node;
        dragNode.fixed = true;
        dragNode.vx = 0;
        dragNode.vy = 0;
      }
    }
    if (dragNode) {
      dragNode.x = pos.x;
      dragNode.y = pos.y;
      dragNode.vx = 0;
      dragNode.vy = 0;
      dirty = true;
      showTip(ev, dragNode, null);
    } else if (panning && lastPtr) {
      cam.x += ev.clientX - lastPtr.x;
      cam.y += ev.clientY - lastPtr.y;
      dirty = true;
    } else {
      hover = hitNode(pos.x, pos.y);
      hoverEdge = hover ? null : hitEdge(pos.x, pos.y);
      showTip(ev, hover, hoverEdge);
      setCursor();
    }
    lastPtr = { x: ev.clientX, y: ev.clientY };
  });

  function endPointer(ev) {
    if (ptrMode === "pending" && ptrStart) {
      selected = selected === ptrStart.node ? null : ptrStart.node;
      dirty = true;
      if (ev) showTip(ev, ptrStart.node, null);
    } else if (ptrMode === "pan" && ptrStart && ev) {
      const moved = Math.hypot(ev.clientX - ptrStart.x, ev.clientY - ptrStart.y);
      if (moved <= DRAG_THRESHOLD) {
        selected = null;
        dirty = true;
      }
    }
    dragNode = null;
    panning = false;
    lastPtr = null;
    ptrMode = null;
    ptrStart = null;
    setCursor();
  }

  canvas.addEventListener("pointerup", endPointer);
  canvas.addEventListener("pointercancel", endPointer);
  canvas.addEventListener("dblclick", (ev) => {
    const pos = worldFromClient(ev.clientX, ev.clientY);
    const node = hitNode(pos.x, pos.y);
    if (node) {
      node.fixed = false;
      dirty = true;
      showTip(ev, node, null);
    }
  });

  document.getElementById("zoom-in").addEventListener("click", () => {
    zoomAt(width / 2, height / 2, 1.2);
  });
  document.getElementById("zoom-out").addEventListener("click", () => {
    zoomAt(width / 2, height / 2, 1 / 1.2);
  });
  document.getElementById("zoom-fit").addEventListener("click", () => {
    const matches = queryMatches();
    fitNodes(selected ? [selected, ...nodes.filter((n) => adj.get(selected.id).has(n.id))] : matches.length ? matches : nodes);
  });
  document.getElementById("zoom-reset").addEventListener("click", () => {
    selected = null;
    query = "";
    search.value = "";
    resetView();
  });
  search.addEventListener("input", () => applySearch(search.value, false));
  search.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") {
      ev.preventDefault();
      applySearch(search.value, true);
    }
  });
  window.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape") {
      selected = null;
      query = "";
      search.value = "";
      dirty = true;
    }
  });

  function tick() {
    if (dragNode || nodes.some((n) => !n.fixed && Math.abs(n.vx) + Math.abs(n.vy) > 0.02)) {
      for (let i = 0; i < 2; i++) step();
    }
    if (dirty) {
      draw();
      dirty = false;
    }
    requestAnimationFrame(tick);
  }

  window.addEventListener("resize", resize);
  if (window.ResizeObserver && canvas.parentElement) {
    new ResizeObserver(resize).observe(canvas.parentElement);
  }
  resize();
  for (let i = 0; i < 80; i++) step();
  fitNodes(nodes);
  tick();
})();
