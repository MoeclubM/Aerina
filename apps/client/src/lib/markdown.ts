import MarkdownIt from "markdown-it";
import type Token from "markdown-it/lib/token.mjs";
import type StateInline from "markdown-it/lib/rules_inline/state_inline.mjs";
import type StateBlock from "markdown-it/lib/rules_block/state_block.mjs";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import css from "highlight.js/lib/languages/css";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import markdown from "highlight.js/lib/languages/markdown";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import sql from "highlight.js/lib/languages/sql";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import yaml from "highlight.js/lib/languages/yaml";
import DOMPurify from "dompurify";
import katex from "katex";

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("sh", bash);
hljs.registerLanguage("shell", bash);
hljs.registerLanguage("css", css);
hljs.registerLanguage("go", go);
hljs.registerLanguage("java", java);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("js", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("markdown", markdown);
hljs.registerLanguage("md", markdown);
hljs.registerLanguage("python", python);
hljs.registerLanguage("py", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("rs", rust);
hljs.registerLanguage("sql", sql);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("ts", typescript);
hljs.registerLanguage("tsx", typescript);
hljs.registerLanguage("jsx", javascript);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("html", xml);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("yml", yaml);

export interface RenderedMarkdown {
  html: string;
  htmlPreviews: string[];
}

interface MarkdownRenderEnvironment {
  htmlPreviewLabel: string;
  htmlPreviews: string[];
}

const cache = new Map<string, RenderedMarkdown>();
const MAX_CACHE = 500;

const KATEX_OPTS: katex.KatexOptions = {
  throwOnError: false,
  errorColor: "#cc0000",
  strict: "ignore",
  trust: false,
  output: "htmlAndMathml",
};

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderKatex(tex: string, displayMode: boolean): string {
  try {
    return katex.renderToString(tex, { ...KATEX_OPTS, displayMode });
  } catch {
    const cls = displayMode ? "katex-error katex-display" : "katex-error";
    return `<span class="${cls}">${escapeHtml(tex)}</span>`;
  }
}

function isValidDollarDelim(src: string, pos: number): { canOpen: boolean; canClose: boolean } {
  const prev = pos > 0 ? src.charCodeAt(pos - 1) : -1;
  const next = pos + 1 < src.length ? src.charCodeAt(pos + 1) : -1;
  let canOpen = true;
  let canClose = true;
  if (prev === 0x20 || prev === 0x09 || (next >= 0x30 && next <= 0x39)) canClose = false;
  if (next === 0x20 || next === 0x09) canOpen = false;
  return { canOpen, canClose };
}

function mathInlineDollar(state: StateInline, silent: boolean): boolean {
  const src = state.src;
  if (src.charCodeAt(state.pos) !== 0x24 /* $ */) return false;
  // $$ is block, handled elsewhere for open at line start; inline $$ also rare — skip double
  if (src.charCodeAt(state.pos + 1) === 0x24) return false;

  const open = isValidDollarDelim(src, state.pos);
  if (!open.canOpen) {
    if (!silent) state.pending += "$";
    state.pos += 1;
    return true;
  }

  const start = state.pos + 1;
  let match = start;
  while ((match = src.indexOf("$", match)) !== -1) {
    let pos = match - 1;
    while (pos >= start && src[pos] === "\\") pos -= 1;
    if ((match - pos) % 2 === 1) break;
    match += 1;
  }
  if (match === -1) {
    if (!silent) state.pending += "$";
    state.pos = start;
    return true;
  }
  if (match === start) {
    if (!silent) state.pending += "$$";
    state.pos = start + 1;
    return true;
  }
  const close = isValidDollarDelim(src, match);
  if (!close.canClose) {
    if (!silent) state.pending += "$";
    state.pos = start;
    return true;
  }
  if (!silent) {
    const token = state.push("math_inline", "math", 0);
    token.markup = "$";
    token.content = src.slice(start, match);
  }
  state.pos = match + 1;
  return true;
}

function mathInlineBracket(state: StateInline, silent: boolean): boolean {
  const src = state.src;
  if (src.charCodeAt(state.pos) !== 0x5c /* \ */) return false;
  const next = src.charCodeAt(state.pos + 1);
  if (next !== 0x28 /* ( */ && next !== 0x5b /* [ */) return false;
  const display = next === 0x5b;
  const close = display ? "\\]" : "\\)";
  const start = state.pos + 2;
  const end = src.indexOf(close, start);
  if (end < 0) return false;
  if (!silent) {
    const token = state.push(display ? "math_block" : "math_inline", "math", 0);
    token.markup = display ? "\\[\\]" : "\\(\\)";
    token.content = src.slice(start, end);
    token.block = display;
  }
  state.pos = end + close.length;
  return true;
}

function mathBlockDollar(
  state: StateBlock,
  startLine: number,
  endLine: number,
  silent: boolean,
): boolean {
  const startPos = state.bMarks[startLine] + state.tShift[startLine];
  const max = state.eMarks[startLine];
  if (startPos + 2 > max) return false;
  if (state.src.slice(startPos, startPos + 2) !== "$$") return false;

  if (silent) return true;

  let firstLine = state.src.slice(startPos + 2, max);
  let next = startLine;
  let lastLine = "";
  let found = false;

  if (firstLine.trim().endsWith("$$")) {
    firstLine = firstLine.trim().slice(0, -2);
    found = true;
  }

  while (!found) {
    next += 1;
    if (next >= endLine) break;
    const pos = state.bMarks[next] + state.tShift[next];
    const lineMax = state.eMarks[next];
    if (pos < lineMax && state.tShift[next] < state.blkIndent) break;
    const line = state.src.slice(pos, lineMax);
    if (line.trim().endsWith("$$")) {
      const lastPos = line.lastIndexOf("$$");
      lastLine = line.slice(0, lastPos);
      found = true;
    }
  }

  state.line = next + 1;
  const token = state.push("math_block", "math", 0);
  token.block = true;
  token.markup = "$$";
  token.map = [startLine, state.line];
  token.content =
    (firstLine && firstLine.trim() ? `${firstLine}\n` : "") +
    state.getLines(startLine + 1, next, state.tShift[startLine], true) +
    (lastLine && lastLine.trim() ? lastLine : "");
  return true;
}

function installKatex(md: MarkdownIt) {
  md.inline.ruler.after("escape", "math_inline_bracket", mathInlineBracket);
  md.inline.ruler.after("math_inline_bracket", "math_inline_dollar", mathInlineDollar);
  md.block.ruler.after("blockquote", "math_block_dollar", mathBlockDollar, {
    alt: ["paragraph", "reference", "blockquote", "list"],
  });
  md.renderer.rules.math_inline = (tokens: Token[], idx: number) =>
    renderKatex(tokens[idx].content, false);
  md.renderer.rules.math_block = (tokens: Token[], idx: number) =>
    `<div class="md-math-block">${renderKatex(tokens[idx].content, true)}</div>\n`;
}

function renderCodeBlock(str: string, lang: string, previewIndex?: number, previewLabel?: string): string {
  const label = lang || "text";
  if (lang === "math" || lang === "latex" || lang === "tex") {
    return `<div class="md-math-block">${renderKatex(str.trim(), true)}</div>`;
  }
  let body = "";
  if (lang && hljs.getLanguage(lang)) {
    body = hljs.highlight(str, { language: lang, ignoreIllegals: true }).value;
  } else {
    body = escapeHtml(str);
  }
  const previewButton =
    previewIndex === undefined
      ? ""
      : `<button class="md-code-preview" type="button" data-html-preview-index="${previewIndex}" aria-label="${escapeHtml(previewLabel || "Preview HTML")}">${escapeHtml(previewLabel || "Preview HTML")} <span aria-hidden="true">↗</span></button>`;
  return `<div class="md-code"><div class="md-code-head"><span>${escapeHtml(label)}</span>${previewButton}</div><pre class="hljs"><code>${body}</code></pre></div>`;
}

const md: MarkdownIt = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
});

installKatex(md);

md.renderer.rules.fence = (tokens, index, _options, env) => {
  const token = tokens[index];
  const language = token.info.trim().split(/\s+/)[0]?.toLowerCase() || "";
  const renderEnv = env as MarkdownRenderEnvironment;
  let previewIndex: number | undefined;
  if (language === "html" || language === "htm") {
    previewIndex = renderEnv.htmlPreviews.length;
    renderEnv.htmlPreviews.push(token.content);
  }
  return `${renderCodeBlock(token.content, language, previewIndex, renderEnv.htmlPreviewLabel)}\n`;
};

const PURIFY_CFG: any = {
  USE_PROFILES: { html: true, mathMl: true },
  ADD_TAGS: [
    "math",
    "semantics",
    "mrow",
    "mi",
    "mo",
    "mn",
    "msup",
    "msub",
    "msubsup",
    "mfrac",
    "msqrt",
    "mroot",
    "mtable",
    "mtr",
    "mtd",
    "mstyle",
    "mspace",
    "mtext",
    "mover",
    "munder",
    "munderover",
    "menclose",
    "mpadded",
    "mphantom",
    "annotation",
    "annotation-xml",
    "svg",
    "path",
  ],
  ADD_ATTR: [
    "class",
    "style",
    "xmlns",
    "encoding",
    "aria-hidden",
    "aria-label",
    "role",
    "mathvariant",
    "stretchy",
    "fence",
    "separator",
    "lspace",
    "rspace",
    "width",
    "height",
    "depth",
    "columnalign",
    "rowalign",
    "columnspacing",
    "rowspacing",
    "data-html-preview-index",
    "viewBox",
    "preserveAspectRatio",
    "d",
  ],
};

export function renderMarkdown(text: string, cacheKey?: string, htmlPreviewLabel = "Preview HTML"): RenderedMarkdown {
  const key = cacheKey ? `${cacheKey}:${htmlPreviewLabel}` : "";
  if (key && cache.has(key)) return cache.get(key)!;
  const env: MarkdownRenderEnvironment = { htmlPreviewLabel, htmlPreviews: [] };
  let html = "";
  try {
    html = md.render(text || "", env);
  } catch {
    html = `<pre>${escapeHtml(text || "")}</pre>`;
  }
  const safe = String(DOMPurify.sanitize(html, PURIFY_CFG));
  const rendered = { html: safe, htmlPreviews: env.htmlPreviews };
  if (key) {
    if (cache.size >= MAX_CACHE) {
      const first = cache.keys().next().value;
      if (first !== undefined) cache.delete(first);
    }
    cache.set(key, rendered);
  }
  return rendered;
}

export function invalidateMarkdownCache(prefix?: string) {
  if (!prefix) {
    cache.clear();
    return;
  }
  for (const k of [...cache.keys()]) {
    if (k.startsWith(prefix)) cache.delete(k);
  }
}
