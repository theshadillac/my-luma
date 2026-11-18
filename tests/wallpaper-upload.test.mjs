import assert from 'node:assert/strict';
import test from 'node:test';
import { cropRect, panPosition } from '../assets/wallpaper-upload/crop.js';
import { readFile } from 'node:fs/promises';
import { loadInfo, loadStoragePath } from '../assets/wallpaper-upload/storage.js';
import { admit, imageType } from '../assets/wallpaper-upload/queue.js';

test('portrait, landscape, panorama and zoom crops stay in bounds at 16:9', () => {
  for (const [width, height] of [
    [4000, 3000],
    [3000, 4000],
    [8000, 500],
    [1920, 1080],
    [1, 12000],
  ]) {
    for (const zoom of [1, 1.5, 3]) {
      for (const x of [0, 0.5, 1]) {
        for (const y of [0, 0.5, 1]) {
          const crop = cropRect(width, height, zoom, x, y);
          assert.ok(Math.abs(crop.w / crop.h - 16 / 9) < 1e-9);
          assert.ok(crop.x >= -1e-9 && crop.y >= -1e-9);
          assert.ok(crop.x + crop.w <= width + 1e-9);
          assert.ok(crop.y + crop.h <= height + 1e-9);
        }
      }
    }
  }
});

test('dragging clamps at both edges and handles axes without overflow', () => {
  assert.equal(panPosition(0.5, 10000, 4000, 2000, 960), 0);
  assert.equal(panPosition(0.5, -10000, 4000, 2000, 960), 1);
  assert.equal(panPosition(0.5, 100, 1920, 1920, 960), 0.5);
  assert.ok(panPosition(0.5, 20, 4000, 2000, 960) < 0.5);
});

test('upload page needs no name and retains its storage-path success view', async () => {
  const html = await readFile(
    new URL('../assets/wallpaper-upload/index.html', import.meta.url),
    'utf8',
  );
  const script = await readFile(
    new URL('../assets/wallpaper-upload/app.js', import.meta.url),
    'utf8',
  );
  assert.doesNotMatch(html, /id="name"|success-name|Background name/);
  assert.match(html, /Your backgrounds were uploaded to/);
  assert.match(html, /id="success-path"/);
  assert.match(html, /id="collection"/);
  assert.match(html, /id="file"[^>]*multiple/);
  assert.match(html, /webkitdirectory/);
  assert.match(script, /fetch\('upload',/);
  assert.doesNotMatch(script, /cleanName|upload\?name/);
});

test('storage path resolves to the exact absolute directory and rejects unavailable or unsafe values', async () => {
  const path = '/media/internal/.com.ecklf.luma/wallpapers';
  assert.equal(
    await loadStoragePath(async (url, options) => {
      assert.equal(url, 'info.json');
      assert.deepEqual(options, { cache: 'no-store' });
      return { ok: true, json: async () => ({ path }) };
    }),
    path,
  );
  await assert.rejects(
    loadStoragePath(async () => ({ ok: false })),
    /unavailable/,
  );
  await assert.rejects(
    loadStoragePath(async () => ({ ok: true, json: async () => ({ path: 'relative/path' }) })),
    /Invalid storage location/,
  );
  await assert.rejects(
    loadStoragePath(async () => ({ ok: true, json: async () => ({ path: 42 }) })),
    /Invalid storage location/,
  );
});

test('info reports free gallery slots and tolerates older TVs without them', async () => {
  const path = '/media/internal/wallpapers';
  const reply = (body) => async () => ({ ok: true, json: async () => body });
  assert.deepEqual(await loadInfo(reply({ path, free: 12 })), { path, free: 12, collections: [] });
  assert.deepEqual(await loadInfo(reply({ path })), { path, free: null, collections: [] });
  assert.deepEqual(await loadInfo(reply({ path, free: -3 })), {
    path,
    free: null,
    collections: [],
  });
  assert.deepEqual(
    (await loadInfo(reply({ path, collections: ['Photos', 'Vintage Porsche', 3, ' '] })))
      .collections,
    ['Photos', 'Vintage Porsche'],
  );
});

const photo = (name, type = 'image/jpeg', size = 1000, lastModified = 1) => ({
  name,
  type,
  size,
  lastModified,
});

test('a multi-photo selection keeps images, drops the rest and explains why', () => {
  const { accepted, note } = admit(
    [
      photo('a.jpg'),
      photo('b.png', 'image/png'),
      photo('clip.mov', 'video/quicktime'),
      photo('huge.jpg', 'image/jpeg', 17 * 1024 * 1024),
      photo('.hidden.jpg'),
      photo('a.jpg'),
    ],
    [],
    null,
  );
  assert.deepEqual(
    accepted.map((file) => file.name),
    ['a.jpg', 'b.png'],
  );
  assert.match(note, /1 over 16 MiB/);
  assert.match(note, /2 not JPG, PNG or WebP/);
});

test('folder picks without a MIME type fall back to the extension', () => {
  assert.equal(imageType(photo('x.JPEG', '')), 'image/jpeg');
  assert.equal(imageType(photo('x.webp', '')), 'image/webp');
  assert.equal(imageType(photo('x.heic', '')), null);
  assert.equal(imageType(photo('x.jpg', 'image/heic')), null);
});

test('the queue stops at the free gallery slots and ignores photos already queued', () => {
  const queued = [{ file: photo('a.jpg') }];
  const { accepted, note } = admit(
    [photo('a.jpg'), photo('b.jpg'), photo('c.jpg'), photo('d.jpg')],
    queued,
    3,
  );
  assert.deepEqual(
    accepted.map((file) => file.name),
    ['b.jpg', 'c.jpg'],
  );
  assert.match(note, /1 didn’t fit/);
});
