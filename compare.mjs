#!/usr/bin/env node
// Diff this renderer's text layout against what Obsidian produced for the
// same canvas, per node. Fixtures come from extract-fixture.mjs.
//
// usage: node compare.mjs fixtures/markdown.json <vault-path>/markdown.canvas

import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [, , fixturePath, canvasPath] = process.argv;
const fixture = JSON.parse(readFileSync(fixturePath, "utf8"));

// The binary reports its own layout in node-local coordinates, so this
// compares like with like instead of reconstructing tops from baselines.
const layoutPath = join(tmpdir(), "canvas-compare.layout.json");
execFileSync(new URL("./canvas-render", import.meta.url).pathname, [canvasPath, layoutPath]);
const laid = JSON.parse(readFileSync(layoutPath, "utf8"));
const byNode = new Map();
for (const r of laid) {
  if (!byNode.has(r.node)) byNode.set(r.node, []);
  byNode.get(r.node).push(r);
}

// The absolute y origin depends on how the browser places a text rect
// relative to the baseline — one unknown constant. Normalising it per node
// means this measures layout (spacing, wrapping, x positions, styles) and
// reports that constant separately rather than failing every run on it.
let total = 0, matched = 0;
const offsets = [];
const report = [];

for (const node of fixture.nodes) {
  if (!node.runs.length) continue;
  let mine = (byNode.get(node.id) ?? []).map((r) => ({
    text: r.text, x: +r.x.toFixed(2), y: +r.y.toFixed(2),
    size: r.size, weight: r.weight, italic: r.italic,
  }));

  const theirs = node.runs.map((r) => ({
    text: r.text.replace(/\n/g, "").trim(),
    x: r.x, y: r.y, size: r.fontSize,
    weight: +r.fontWeight, italic: r.fontStyle === "italic",
  })).filter((r) => r.text);

  // Group by line before ordering: runs sharing a line can differ in
  // baseline when their sizes differ (inline code sits lower).
  // List markers are drawn here but come from CSS ::marker in Obsidian,
  // so they are not text nodes and have nothing to compare against.
  const MARKER = /^([•‣▪]|\d+\.|☐|☑)$/;
  mine = mine
    .filter((m) => !MARKER.test(m.text.trim()))
    .sort((a, b) => Math.round(a.y / 8) - Math.round(b.y / 8) || a.x - b.x);
  const ordered = [...theirs];
  let shift = 0;
  if (mine.length && ordered.length) {
    shift = mine[0].y - ordered[0].y;
    offsets.push(+shift.toFixed(2));
  }
  // both sides are now text-rect tops, so any residual shift is a real
  // difference in where content starts rather than a unit mismatch

  const issues = [];
  for (const [index, want] of ordered.entries()) {
    total++;
    const got = mine[index];
    if (!got) { issues.push(`missing  ${JSON.stringify(want.text).slice(0, 34)}`); continue; }
    if (got.text.trim() !== want.text) {
      issues.push(`text     ${JSON.stringify(got.text).slice(0, 22)} vs ${JSON.stringify(want.text).slice(0, 22)}`);
      continue;
    }
    const dx = Math.abs(got.x - want.x), dy = Math.abs(got.y - shift - want.y);
    const styled = got.size.toFixed(0) === want.size.toFixed(0)
      && got.weight === want.weight && got.italic === want.italic;
    if (dx <= 2 && dy <= 2 && styled) { matched++; continue; }
    const why = [];
    if (dx > 2) why.push(`x ${got.x} vs ${want.x}`);
    if (dy > 2) why.push(`y ${(got.y - shift).toFixed(2)} vs ${want.y}`);
    if (got.size.toFixed(0) !== want.size.toFixed(0)) why.push(`size ${got.size} vs ${want.size}`);
    if (got.weight !== want.weight) why.push(`weight ${got.weight} vs ${want.weight}`);
    if (got.italic !== want.italic) why.push(`italic ${got.italic} vs ${want.italic}`);
    issues.push(`${JSON.stringify(want.text).slice(0, 26).padEnd(28)} ${why.join(", ")}`);
  }
  if (issues.length) report.push({ id: node.id, issues });
}

const spread = offsets.length ? Math.max(...offsets) - Math.min(...offsets) : 0;
console.log(`runs matching Obsidian: ${matched}/${total}`);
console.log(`baseline offset: ${offsets.length ? (offsets.reduce((a, b) => a + b, 0) / offsets.length).toFixed(2) : "n/a"} (spread ${spread.toFixed(2)})`);
for (const r of report) {
  console.log(`\n  ${r.id}`);
  for (const i of r.issues.slice(0, 6)) console.log(`    ${i}`);
  if (r.issues.length > 6) console.log(`    … ${r.issues.length - 6} more`);
}
writeFileSync(join(tmpdir(), "canvas-compare.json"), JSON.stringify(report, null, 1));
process.exit(matched === total ? 0 : 1);
