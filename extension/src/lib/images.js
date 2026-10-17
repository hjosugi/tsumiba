// Naming the file for an image that is about to be uploaded.
//
// The name comes from the image's URL, but URLs lie: `photo.php?id=3` serving
// a JPEG, `.jpg` addresses returning WebP. The Content-Type is the better
// witness, so the extension is corrected to match it.

const MIME_TO_EXT = new Map([
  ['image/png', 'png'],
  ['image/jpeg', 'jpg'],
  ['image/jpg', 'jpg'],
  ['image/pjpeg', 'jpg'],
  ['image/gif', 'gif'],
  ['image/webp', 'webp'],
  ['image/avif', 'avif'],
  ['image/svg+xml', 'svg'],
  ['image/bmp', 'bmp'],
  ['image/x-icon', 'ico'],
  ['image/vnd.microsoft.icon', 'ico'],
  ['image/tiff', 'tiff'],
  ['image/heic', 'heic'],
]);

/** Extensions that name the same format as the canonical one. */
const EXT_ALIASES = new Map([
  ['jpeg', 'jpg'],
  ['jpe', 'jpg'],
  ['jfif', 'jpg'],
  ['tif', 'tiff'],
  ['svgz', 'svg'],
]);

const MAX_NAME_CHARS = 100;

/** `image/png; charset=binary` -> `image/png` */
export function mimeBase(contentType) {
  return String(contentType ?? '').split(';')[0].trim().toLowerCase();
}

export function extensionForMime(contentType) {
  return MIME_TO_EXT.get(mimeBase(contentType)) ?? null;
}

/**
 * Is a fetched body plausibly an image? Real image types, yes; missing or
 * generic types are tolerated (some servers do not set them); HTML or JSON is
 * not an image (it is probably a login page or an error).
 */
export function looksLikeImage(contentType) {
  const mime = mimeBase(contentType);
  return mime.startsWith('image/') || mime === '' || mime === 'application/octet-stream';
}

export function isDataUrl(src) {
  return /^data:/i.test(String(src ?? ''));
}

/** The MIME type in a data: URL, e.g. `data:image/png;base64,...` -> `image/png`. */
export function mimeFromDataUrl(src) {
  const match = /^data:([^;,]*)/i.exec(String(src ?? ''));
  return match ? match[1].toLowerCase() : '';
}

function canonicalExtension(ext) {
  const lower = ext.toLowerCase();
  return EXT_ALIASES.get(lower) ?? lower;
}

function splitExtension(name) {
  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot === name.length - 1) return { stem: name, ext: '' };
  const ext = name.slice(dot + 1);
  return /^[A-Za-z0-9]{1,5}$/.test(ext) ? { stem: name.slice(0, dot), ext } : { stem: name, ext: '' };
}

function cleanSegment(segment) {
  let decoded = segment;
  try {
    decoded = decodeURIComponent(segment);
  } catch {
    // keep the raw text: a stray % must not stop the upload
  }
  return decoded
    .replace(/[\u0000-\u001f\u007f]/g, '')
    .replace(/[\\/:*?"<>|]/g, '_')
    .trim()
    .replace(/^\.+|\.+$/g, '');
}

function clampName(stem, ext) {
  const room = Math.max(1, MAX_NAME_CHARS - (ext ? ext.length + 1 : 0));
  const clipped = [...stem].slice(0, room).join('');
  return ext ? `${clipped}.${ext}` : clipped;
}

/**
 * The file name to upload an image under.
 * @param {{src: string, contentType?: string, fallback?: string}} input
 */
export function imageFileName({ src, contentType = '', fallback = 'image' }) {
  let stem = '';
  let ext = '';
  let mime = mimeBase(contentType);

  if (isDataUrl(src)) {
    mime = mime || mimeFromDataUrl(src);
  } else {
    try {
      const url = new URL(src);
      // A blob: address is a random identifier, not a name.
      const segments = url.protocol === 'blob:' ? [] : url.pathname.split('/').filter(Boolean);
      const last = segments.length ? cleanSegment(segments[segments.length - 1]) : '';
      ({ stem, ext } = splitExtension(last));
    } catch {
      // not a URL: fall through to the fallback name
    }
  }

  if (!stem) stem = fallback;

  // The Content-Type is the better witness: `photo.php` serving a JPEG becomes
  // `photo.jpg`, a `.jpg` address that really returns WebP becomes `.webp`.
  // Without a known type the URL's own extension (if any) is left alone.
  const wanted = extensionForMime(mime);
  if (wanted && (!ext || canonicalExtension(ext) !== wanted)) ext = wanted;
  return clampName(stem, ext.toLowerCase());
}
