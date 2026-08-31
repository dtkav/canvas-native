#!/usr/bin/env node
// Compare the binary's SVG path data against a live Obsidian's own.
import { mkdtempSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [, , title, file] = process.argv;
const PORT = process.env.OBSIDIAN_DEBUG_PORT ?? "9222";
const scratch = join(mkdtempSync(join(tmpdir(), "canvas-verify-")), "verify.svg");
execFileSync(new URL("./canvas-render", import.meta.url).pathname, [file, scratch]);
const svg = readFileSync(scratch, "utf8");
const mine = [...svg.matchAll(/<path d="([^"]+)"/g)].map((m) => m[1]);

const list = await (await fetch(`http://localhost:${PORT}/json/list`)).json();
const target = list.find((t) => t.type === "page" && (t.title || "").includes(title));
if (!target) { console.error("no tab matching", title); process.exit(1); }
const ws = new WebSocket(target.webSocketDebuggerUrl);
let seq = 0; const pending = new Map();
const send = (m, p) => new Promise((r) => { const id = ++seq; pending.set(id, r); ws.send(JSON.stringify({ id, method: m, params: p })); });
ws.onmessage = (e) => { const m = JSON.parse(e.data); if (pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); } };

ws.onopen = async () => {
  const out = await send("Runtime.evaluate", {
    expression: `JSON.stringify([...app.workspace.activeLeaf.view.canvas.edges.values()].map(e => e.path.display.getAttribute("d")))`,
    returnByValue: true,
  });
  const theirs = JSON.parse(out.result.value);
  // Compare geometry, not formatting: split on command letters and round.
  const norm = (d) => d
    .replace(/([MLC])/g, " $1 ")
    .replace(/,/g, " ")
    .trim()
    .split(/\s+/)
    .map((t) => (isNaN(+t) ? t : (+t).toFixed(2)))
    .join(" ");
  const theirSet = new Set(theirs.map(norm));
  const matched = mine.filter((d) => theirSet.has(norm(d)));
  console.log(`matching paths: ${matched.length}/${theirs.length}`);
  for (const d of mine) if (!theirSet.has(norm(d))) console.log("  only in binary:", norm(d).slice(0, 90));
  ws.close(); process.exit(matched.length === theirs.length ? 0 : 1);
};
