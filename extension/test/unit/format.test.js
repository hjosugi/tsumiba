import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  MAX_SELECTION_BYTES,
  TRUNCATION_MARKER,
  escapeLinkText,
  escapeLinkUrl,
  imageNote,
  linkNote,
  markdownLink,
  memoNote,
  normalizeNewlines,
  pageNote,
  selectionNote,
  singleLine,
  truncateUtf8,
} from '../../src/lib/format.js';

test('a page becomes a heading and its address', () => {
  assert.equal(pageNote({ title: 'Example Domain', url: 'https://example.com/' }), '# Example Domain\nhttps://example.com/\n');
});

test('a page without a title is just its address', () => {
  assert.equal(pageNote({ title: '', url: 'https://example.com/' }), 'https://example.com/\n');
  assert.equal(pageNote({ title: '   \n ', url: 'https://example.com/' }), 'https://example.com/\n');
});

test('page titles are flattened onto one line', () => {
  assert.equal(
    pageNote({ title: '  A\n  very   long\ttitle ', url: 'https://x.test/' }),
    '# A very long title\nhttps://x.test/\n',
  );
});

test('a selection is quoted line by line with its source below', () => {
  const note = selectionNote({
    selection: 'first line\nsecond line',
    title: 'The Page',
    url: 'https://example.com/a',
  });
  assert.equal(note, '> first line\n> second line\n\n[The Page](https://example.com/a)\n');
});

test('blank lines inside a quote stay quoted, with no trailing space', () => {
  const note = selectionNote({ selection: 'one\n\ntwo', title: 'T', url: 'https://x.test/' });
  assert.equal(note, '> one\n>\n> two\n\n[T](https://x.test/)\n');
});

test('selections are trimmed and CRLF is normalised', () => {
  const note = selectionNote({ selection: '\r\n\r\n  hello\r\nworld  \r\n\r\n', title: 'T', url: 'https://x.test/' });
  assert.equal(note, '>   hello\n> world\n\n[T](https://x.test/)\n');
  assert.equal(normalizeNewlines('a\r\nb\rc\nd'), 'a\nb\nc\nd');
});

test('a selection without a source has no dangling link', () => {
  assert.equal(selectionNote({ selection: 'text' }), '> text\n');
});

test('an oversized selection is cut at the limit and says so', () => {
  // 3 bytes per character: 400 000 characters is about 1.2 MB.
  const selection = 'あ'.repeat(400_000);
  const note = selectionNote({ selection, title: 'T', url: 'https://x.test/' });
  assert.ok(note.includes(TRUNCATION_MARKER), 'the cut is announced');
  const quotedBody = note.split('\n\n[T]')[0];
  // "> " plus the marker line is small next to the megabyte.
  assert.ok(new TextEncoder().encode(quotedBody).length < MAX_SELECTION_BYTES + 200);
  assert.ok(!note.includes('�'), 'no character was split in half');
});

test('a selection just under the limit is untouched', () => {
  const selection = 'a'.repeat(1000);
  assert.ok(!selectionNote({ selection }).includes(TRUNCATION_MARKER));
});

test('truncateUtf8 never splits a character', () => {
  assert.deepEqual(truncateUtf8('abc', 10), { text: 'abc', truncated: false });
  // "あ" is 3 bytes: 7 bytes holds two of them.
  assert.deepEqual(truncateUtf8('あいう', 7), { text: 'あい', truncated: true });
  // An astral character (4 bytes, 2 UTF-16 units) is kept whole or dropped whole.
  assert.deepEqual(truncateUtf8('a😀b', 4), { text: 'a', truncated: true });
  assert.deepEqual(truncateUtf8('a😀b', 5), { text: 'a😀', truncated: true });
  assert.deepEqual(truncateUtf8('', 0), { text: '', truncated: false });
});

test('a link note has the target, then where it was found', () => {
  assert.equal(
    linkNote({ href: 'https://target.test/x', text: 'Read this', pageTitle: 'Blog', pageUrl: 'https://blog.test/' }),
    '# Read this\nhttps://target.test/x\n\nFrom: [Blog](https://blog.test/)\n',
  );
});

test('a link without text is headed by its own address', () => {
  assert.equal(
    linkNote({ href: 'https://target.test/x', text: '', pageTitle: 'Blog', pageUrl: 'https://blog.test/' }),
    '# https://target.test/x\nhttps://target.test/x\n\nFrom: [Blog](https://blog.test/)\n',
  );
});

test('a link without a known page has no From line', () => {
  assert.equal(linkNote({ href: 'https://t.test/', text: 'x' }), '# x\nhttps://t.test/\n');
});

test('a page with no title is linked by its address', () => {
  assert.equal(
    linkNote({ href: 'https://t.test/', text: 'x', pageUrl: 'https://p.test/' }),
    '# x\nhttps://t.test/\n\nFrom: [https://p.test/](https://p.test/)\n',
  );
});

test('an image note carries the image address and its page', () => {
  assert.equal(
    imageNote({ src: 'https://cdn.test/cat.png', fileName: 'cat.png', pageTitle: 'Cats', pageUrl: 'https://cats.test/' }),
    '# cat.png\nhttps://cdn.test/cat.png\n\nFrom: [Cats](https://cats.test/)\n',
  );
  assert.equal(imageNote({ src: 'https://cdn.test/x' }), '# Image\nhttps://cdn.test/x\n');
});

test('link text and urls are escaped for Markdown', () => {
  assert.equal(escapeLinkText('a [b] c\\d'), 'a \\[b\\] c\\\\d');
  assert.equal(escapeLinkUrl('https://x.test/a b(c)'), 'https://x.test/a%20b%28c%29');
  assert.equal(markdownLink('Title [1]', 'https://x.test/(a)'), '[Title \\[1\\]](https://x.test/%28a%29)');
  // Empty text falls back to the address.
  assert.equal(markdownLink('', 'https://x.test/'), '[https://x.test/](https://x.test/)');
});

test('a title containing brackets does not break the selection source link', () => {
  const note = selectionNote({ selection: 'q', title: 'Docs [v2]', url: 'https://x.test/a_(b)' });
  assert.equal(note, '> q\n\n[Docs \\[v2\\]](https://x.test/a_%28b%29)\n');
});

test('singleLine collapses every kind of whitespace', () => {
  assert.equal(singleLine(' a\n\n b\t\tc  '), 'a b c');
  assert.equal(singleLine(undefined), '');
  assert.equal(singleLine(null), '');
});

test('memo text is trimmed and ends with one newline; empty stays empty', () => {
  // Leading blank lines go, but the first line's own indentation stays.
  assert.equal(memoNote('\n\n  buy milk  \r\n\r\n'), '  buy milk\n');
  assert.equal(memoNote('first\r\nsecond'), 'first\nsecond\n');
  assert.equal(memoNote('   \n  '), '');
  assert.equal(memoNote(''), '');
});
