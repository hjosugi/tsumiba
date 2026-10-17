import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  extensionForMime,
  imageFileName,
  isDataUrl,
  looksLikeImage,
  mimeBase,
  mimeFromDataUrl,
} from '../../src/lib/images.js';

test('the name comes from the last path segment', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/a/b/cat.png' }), 'cat.png');
  assert.equal(imageFileName({ src: 'https://cdn.test/a/b/cat.png?w=100&h=200#x' }), 'cat.png');
});

test('percent-escapes and Japanese names are decoded', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/%E7%8C%AB.jpg' }), '猫.jpg');
  assert.equal(imageFileName({ src: 'https://cdn.test/my%20photo.webp' }), 'my photo.webp');
});

test('a broken escape does not stop the upload', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/100%.png' }), '100%.png');
});

test('the content type corrects a wrong or missing extension', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/photo.php?id=3', contentType: 'image/jpeg' }), 'photo.jpg');
  assert.equal(imageFileName({ src: 'https://cdn.test/render', contentType: 'image/png' }), 'render.png');
  assert.equal(imageFileName({ src: 'https://cdn.test/pic.jpg', contentType: 'image/webp' }), 'pic.webp');
  assert.equal(imageFileName({ src: 'https://cdn.test/pic.PNG', contentType: 'image/png; charset=binary' }), 'pic.png');
});

test('an extension that already matches the type is kept as written', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/pic.jpeg', contentType: 'image/jpeg' }), 'pic.jpeg');
  assert.equal(imageFileName({ src: 'https://cdn.test/pic.JPG', contentType: 'image/jpeg' }), 'pic.jpg');
});

test('without a known type the url extension is left alone', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/pic.gif' }), 'pic.gif');
  assert.equal(imageFileName({ src: 'https://cdn.test/pic.gif', contentType: 'application/octet-stream' }), 'pic.gif');
  assert.equal(imageFileName({ src: 'https://cdn.test/noext' }), 'noext');
});

test('an address with no file name gets the fallback', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/', contentType: 'image/png' }), 'image.png');
  assert.equal(imageFileName({ src: 'https://cdn.test', contentType: 'image/gif' }), 'image.gif');
  assert.equal(imageFileName({ src: 'not a url', contentType: 'image/png' }), 'image.png');
  assert.equal(imageFileName({ src: 'https://cdn.test/', fallback: 'picture' }), 'picture');
});

test('data URLs are named from their media type', () => {
  assert.equal(imageFileName({ src: 'data:image/png;base64,AAAA' }), 'image.png');
  assert.equal(imageFileName({ src: 'data:image/svg+xml;utf8,<svg/>' }), 'image.svg');
  assert.equal(imageFileName({ src: 'data:image/jpeg;base64,AAAA', contentType: 'image/webp' }), 'image.webp');
  assert.equal(mimeFromDataUrl('data:image/png;base64,AAAA'), 'image/png');
  assert.equal(mimeFromDataUrl('data:,hello'), '');
  assert.ok(isDataUrl('data:image/png;base64,AAAA'));
  assert.ok(isDataUrl('DATA:image/png;base64,AAAA'));
  assert.ok(!isDataUrl('https://x.test/data:'));
});

test('a blob: address is an identifier, not a name', () => {
  const src = 'blob:https://example.com/0b8c6f0e-0000-4000-8000-000000000000';
  assert.equal(imageFileName({ src }), 'image');
  assert.equal(imageFileName({ src, contentType: 'image/png' }), 'image.png');
});

test('names are cleaned of path and control characters', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/a%2Fb%3Fc.png' }), 'a_b_c.png');
  assert.equal(imageFileName({ src: 'https://cdn.test/..%2F..%2Fevil.png' }), '_.._evil.png');
  assert.ok(!imageFileName({ src: 'https://cdn.test/x%00y.png' }).includes('\u0000'));
});

test('very long names are shortened but keep their extension', () => {
  const name = imageFileName({ src: `https://cdn.test/${'あ'.repeat(300)}.png` });
  assert.ok([...name].length <= 100, `${[...name].length} characters`);
  assert.ok(name.endsWith('.png'));
});

test('dotfiles and trailing dots are not mistaken for extensions', () => {
  assert.equal(imageFileName({ src: 'https://cdn.test/.hidden', contentType: 'image/png' }), 'hidden.png');
  assert.equal(imageFileName({ src: 'https://cdn.test/file.', contentType: 'image/png' }), 'file.png');
});

test('mime helpers', () => {
  assert.equal(mimeBase('Image/PNG; charset=binary'), 'image/png');
  assert.equal(mimeBase(undefined), '');
  assert.equal(extensionForMime('image/jpeg'), 'jpg');
  assert.equal(extensionForMime('image/svg+xml'), 'svg');
  assert.equal(extensionForMime('text/html'), null);
});

test('only plausible bodies count as images', () => {
  assert.ok(looksLikeImage('image/png'));
  assert.ok(looksLikeImage('image/x-unheard-of'));
  assert.ok(looksLikeImage(''));
  assert.ok(looksLikeImage('application/octet-stream'));
  assert.ok(!looksLikeImage('text/html; charset=utf-8'));
  assert.ok(!looksLikeImage('application/json'));
});
