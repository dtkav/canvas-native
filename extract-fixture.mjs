#!/usr/bin/env node
// Open a canvas in Obsidian and record what its renderer produced: every text
// line, in node-local coordinates, with the style it was drawn in.
//
// Node-local means (rect - nodeRect) / scale, so the result is independent of
// the zoom and pan the window happens to be at.
//
// usage: node extract-fixture.mjs <tab-substring> <vault-relative.canvas> [out.json]

import { writeFileSync } from "node:fs";

const [, , tab, path, out = "fixture.json"] = process.argv;
const PORT = process.env.OBSIDIAN_DEBUG_PORT ?? "9222";

const list = await (await fetch(`http://localhost:${PORT}/json/list`)).json();
const target = list.find((t) => t.type === "page" && (t.title || "").includes(tab));
if (!target) {
  console.error("no tab matching", tab);
  process.exit(1);
}

const ws = new WebSocket(target.webSocketDebuggerUrl);
let seq = 0;
const pending = new Map();
const send = (method, params) =>
  new Promise((r) => {
    const id = ++seq;
    pending.set(id, r);
    ws.send(JSON.stringify({ id, method, params }));
  });
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (pending.has(m.id)) {
    pending.get(m.id)(m.result);
    pending.delete(m.id);
  }
};

const evaluate = async (expression) => {
  const r = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (r?.exceptionDetails) throw new Error(r.exceptionDetails.text ?? "eval failed");
  return r?.result?.value;
};

ws.onopen = async () => {
  await evaluate(`(async () => {
    const file = app.vault.getAbstractFileByPath(${JSON.stringify(path)});
    if (!file) throw new Error("not found: " + ${JSON.stringify(path)});
    await app.workspace.getLeaf(false).openFile(file);
    await new Promise(r => setTimeout(r, 1200));
    return true;
  })()`);

  const data = await evaluate(`(() => {
    const view = app.workspace.activeLeaf.view;
    const canvas = view.canvas;
    const out = { nodes: [], edges: [] };

    for (const node of canvas.nodes.values()) {
      const el = node.nodeEl;
      const box = el.getBoundingClientRect();
      const scale = box.width / node.width;
      const local = (r) => ({
        x: +((r.left - box.left) / scale).toFixed(2),
        y: +((r.top - box.top) / scale).toFixed(2),
        w: +(r.width / scale).toFixed(2),
        h: +(r.height / scale).toFixed(2),
      });

      const record = { id: node.id, type: node.getData().type, runs: [], blocks: [] };

      // Block geometry: spacing rules are far easier to read off the boxes
      // than to infer from where the glyphs landed.
      for (const b of el.querySelectorAll(
        ".markdown-preview-sizer > *, li, blockquote, .callout, table, pre, hr"
      )) {
        const cs = getComputedStyle(b);
        record.blocks.push({
          cls: b.className || b.tagName.toLowerCase(),
          tag: b.tagName.toLowerCase(),
          marginTop: parseFloat(cs.marginTop),
          marginBottom: parseFloat(cs.marginBottom),
          paddingLeft: parseFloat(cs.paddingLeft),
          lineHeight: cs.lineHeight,
          ...local(b.getBoundingClientRect()),
        });
      }

      const root = el.querySelector(".markdown-preview-sizer") || el;
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      for (let t = walker.nextNode(); t; t = walker.nextNode()) {
        if (!t.textContent.trim()) continue;
        const parent = t.parentElement;
        const cs = getComputedStyle(parent);
        // getClientRects gives one rect per line but textContent is the whole
        // node, so walk character offsets and group them into lines. Without
        // this, wrapped text has positions that cannot be matched to words.
        const range = document.createRange();
        const lines = [];
        for (let i = 0; i < t.textContent.length; i++) {
          range.setStart(t, i);
          range.setEnd(t, i + 1);
          const r = range.getBoundingClientRect();
          if (!r.width && !r.height) continue;
          const key = Math.round(r.top);
          const last = lines[lines.length - 1];
          if (last && last.key === key) {
            last.text += t.textContent[i];
            last.right = r.right;
          } else {
            lines.push({ key, text: t.textContent[i], rect: r, right: r.right });
          }
        }
        for (const line of lines) {
          if (!line.text.trim()) continue;
          record.runs.push({
            text: line.text,
            tag: parent.tagName.toLowerCase(),
            cls: parent.className || "",
            fontSize: +parseFloat(cs.fontSize).toFixed(2),
            fontWeight: cs.fontWeight,
            fontStyle: cs.fontStyle,
            color: cs.color,
            decoration: cs.textDecorationLine,
            ...local(line.rect),
            w: +((line.right - line.rect.left) / scale).toFixed(2),
          });
        }
      }
      const label = el.querySelector(".canvas-group-label");
      if (label) record.label = { text: label.textContent, ...local(label.getBoundingClientRect()) };
      out.nodes.push(record);
    }

    for (const edge of canvas.edges.values()) {
      out.edges.push({
        id: edge.id,
        d: edge.path.display.getAttribute("d"),
        label: edge.label ?? null,
        stroke: getComputedStyle(edge.path.display).stroke,
      });
    }
    return out;
  })()`);

  writeFileSync(out, JSON.stringify(data, null, 1));
  const runs = data.nodes.reduce((n, x) => n + x.runs.length, 0);
  console.log(`${out}: ${data.nodes.length} nodes, ${runs} text runs, ${data.edges.length} edges`);
  ws.close();
  process.exit(0);
};
