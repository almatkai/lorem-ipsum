import assert from "node:assert/strict";
import test from "node:test";
import { generateParagraphs, generateSentences, generateWords, safeLoremIpsumNumberArg } from "../src/lib/lorem";

test("generateParagraphs produces double-newline separated paragraphs", () => {
  const out = generateParagraphs(2);
  const parts = out.split("\r\n\r\n");
  assert.equal(parts.length, 2);
  for (const p of parts) {
    assert.ok(p.length > 0);
    // paragraphs contain spaces (words) and end with period
    assert.match(p, /\w/);
  }
});

test("generateSentences returns requested count and generateWords returns words", () => {
  const s1 = generateSentences(1);
  assert.ok(s1.length > 0);
  assert.match(s1, /\w/);

  const s3 = generateSentences(3);
  // 3 sentences concatenated; splitting by period should yield roughly 3 sentences
  assert.ok(s3.split(".").length >= 3);

  const w5 = generateWords(5);
  assert.ok(w5.length > 0);
  const tokens = w5.split(/\s+/).filter(Boolean);
  assert.equal(tokens.length, 5);
});

test("safeLoremIpsumNumberArg validates and caps at 1000", async () => {
  const defaults = await safeLoremIpsumNumberArg(undefined);
  assert.equal(defaults.error, null);
  assert.equal(defaults.safeLoremIpsumNumber, 1);

  const ok = await safeLoremIpsumNumberArg("7");
  assert.equal(ok.error, null);
  assert.equal(ok.safeLoremIpsumNumber, 7);

  const capped = await safeLoremIpsumNumberArg("1001");
  assert.notEqual(capped.error, null);
  assert.equal(capped.safeLoremIpsumNumber, null);
  assert.match(capped.error!.message, /no more than 1000/);

  const bad = await safeLoremIpsumNumberArg("not-a-number");
  assert.notEqual(bad.error, null);
  assert.equal(bad.safeLoremIpsumNumber, null);
});

test("package.json declares Windows+macOS and Tezbar metadata", async () => {
  const pkg = await import("../package.json", { with: { type: "json" } });
  const data = (pkg as unknown as { default: Record<string, unknown> }).default ??
    (pkg as unknown as Record<string, unknown>);
  assert.deepEqual((data as { platforms: string[] }).platforms, ["macOS", "Windows"]);
  const tezbar = (data as { tezbar: { platforms: string[]; repository: string; upstream: { commit: string; path: string } } }).tezbar;
  assert.deepEqual(tezbar.platforms, ["macos", "windows"]);
  assert.equal(tezbar.repository, "https://github.com/almatkai/lorem-ipsum");
  assert.equal(tezbar.upstream.path, "extensions/lorem-ipsum");
  assert.equal(tezbar.upstream.commit, "6734d85992290066fd31a7f5bee3cca3d3545a1a");
  const commands = (data as { commands: { platforms: string[] }[] }).commands;
  assert.equal(commands.length, 4);
  for (const cmd of commands) {
    assert.deepEqual(cmd.platforms, ["macOS", "Windows"]);
  }
});
