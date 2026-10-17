// Turning what you captured in the browser into note text.
//
// Notes are plain Markdown so they read well in the app and in any editor:
//
//   page       # Title\nhttps://example.com/\n
//   selection  > quoted\n> lines\n\n[Title](https://example.com/)\n
//   link       # Link text\nhttps://target/\n\nFrom: [Title](https://example.com/)\n

/** Selections longer than this (in UTF-8 bytes) are cut and say so. */
export const MAX_SELECTION_BYTES = 1024 * 1024;
export const TRUNCATION_MARKER = '… [truncated by tsumiba]';

export function normalizeNewlines(text) {
  return String(text ?? '').replace(/\r\n?/g, '\n');
}

/** Collapse all whitespace, newlines included, into single spaces. */
export function singleLine(text) {
  return normalizeNewlines(text).replace(/\s+/g, ' ').trim();
}

/** Text safe to put between the brackets of a Markdown link. */
export function escapeLinkText(text) {
  return singleLine(text).replace(/([\\[\]])/g, '\\$1');
}

/** A URL safe to put between the parentheses of a Markdown link. */
export function escapeLinkUrl(url) {
  return String(url ?? '')
    .trim()
    .replace(/ /g, '%20')
    .replace(/\(/g, '%28')
    .replace(/\)/g, '%29')
    .replace(/</g, '%3C')
    .replace(/>/g, '%3E');
}

export function markdownLink(text, url) {
  const target = escapeLinkUrl(url);
  return `[${escapeLinkText(text || url)}](${target})`;
}

/** Cut `text` to at most `maxBytes` of UTF-8 without splitting a character. */
export function truncateUtf8(text, maxBytes) {
  const encoder = new TextEncoder();
  if (encoder.encode(text).length <= maxBytes) return { text, truncated: false };
  let used = 0;
  let end = 0;
  for (const char of text) {
    const size = encoder.encode(char).length;
    if (used + size > maxBytes) break;
    used += size;
    end += char.length;
  }
  return { text: text.slice(0, end), truncated: true };
}

function quote(text) {
  return text
    .split('\n')
    .map((line) => (line.trim() === '' ? '>' : `> ${line}`))
    .join('\n');
}

function trimBlankLines(text) {
  return text.replace(/^\s*\n/, '').replace(/\s+$/, '');
}

/** The "where from" line shared by selection, link and image notes. */
function sourceLine(title, url) {
  return url ? markdownLink(title || url, url) : '';
}

export function pageNote({ title, url }) {
  const heading = singleLine(title);
  const address = String(url ?? '').trim();
  if (!heading) return `${address}\n`;
  return `# ${heading}\n${address}\n`;
}

export function selectionNote({ selection, title, url, maxBytes = MAX_SELECTION_BYTES }) {
  const cleaned = trimBlankLines(normalizeNewlines(selection));
  const { text, truncated } = truncateUtf8(cleaned, maxBytes);
  let body = quote(text);
  if (truncated) body += `\n> ${TRUNCATION_MARKER}`;
  const source = sourceLine(title, url);
  return source ? `${body}\n\n${source}\n` : `${body}\n`;
}

export function linkNote({ href, text, pageTitle, pageUrl }) {
  const link = String(href ?? '').trim();
  const heading = singleLine(text) || link;
  let note = `# ${heading}\n${link}\n`;
  if (pageUrl) note += `\nFrom: ${markdownLink(pageTitle || pageUrl, pageUrl)}\n`;
  return note;
}

/** A note holding an image's address, for when the image itself cannot be saved. */
export function imageNote({ src, fileName, pageTitle, pageUrl }) {
  const address = String(src ?? '').trim();
  let note = `# ${singleLine(fileName) || 'Image'}\n${address}\n`;
  if (pageUrl) note += `\nFrom: ${markdownLink(pageTitle || pageUrl, pageUrl)}\n`;
  return note;
}

/** Text typed into the quick-memo box. */
export function memoNote(text) {
  const cleaned = trimBlankLines(normalizeNewlines(text));
  return cleaned === '' ? '' : `${cleaned}\n`;
}
