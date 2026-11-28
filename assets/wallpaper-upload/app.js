import { cropRect, panPosition } from './crop.js';
import { admit, plural } from './queue.js';
import { loadInfo } from './storage.js';

const element = (id) => document.getElementById(id);
const picker = element('picker');
const queueView = element('queue');
const grid = element('grid');
const editor = element('editor');
const canvas = element('preview');
const zoom = element('zoom');
const status = element('status');
const submit = element('submit');

let items = [];
let next = 0;
let busy = false;
let editing;
let drag;
let free = null;
const token = location.pathname.split('/')[1];

const info = loadInfo(fetch).catch(() => {
  element('storage-path').textContent = 'Storage path unavailable';
  return null;
});
const NEW_COLLECTION = '\u0000new';
const picker_collection = element('collection');
function fillCollections(names, chosen) {
  const list = names.length ? names : ['Photos'];
  picker_collection.replaceChildren(
    ...list.map((name) => new Option(name, name)),
    new Option('New collection…', NEW_COLLECTION),
  );
  picker_collection.value = list.includes(chosen) ? chosen : list[0];
}
let lastCollection = 'Photos';
try {
  lastCollection = localStorage.getItem('luma-collection') || 'Photos';
} catch {
  /* private browsing */
}
picker_collection.addEventListener('change', () => {
  if (picker_collection.value !== NEW_COLLECTION) {
    lastCollection = picker_collection.value;
    return;
  }
  const name = (prompt('Name the new collection, e.g. Vintage Porsche') || '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 32);
  const names = [...picker_collection.options]
    .map((option) => option.value)
    .filter((value) => value !== NEW_COLLECTION);
  if (!name) {
    picker_collection.value = lastCollection;
    return;
  }
  const existing = names.find((item) => item.toLowerCase() === name.toLowerCase());
  if (!existing) names.splice(1, 0, name);
  lastCollection = existing || name;
  fillCollections(names, lastCollection);
});
info.then((value) => {
  if (!value) return;
  element('storage-path').textContent = value.path;
  free = value.free;
  fillCollections(value.collections, lastCollection);
  render();
});

const folder = element('folder');
if ('webkitdirectory' in folder) element('folder-picker').hidden = false;

function message(text, error = false) {
  status.textContent = text;
  status.classList.toggle('error', error);
}

async function decode(file) {
  const image = await createImageBitmap(file, { imageOrientation: 'from-image' });
  if (!image.width || !image.height || image.width * image.height > 32_000_000) {
    image.close();
    throw new Error('Larger than 32 megapixels');
  }
  return image;
}

function paint(target, image, crop) {
  const { x, y, w, h } = cropRect(image.width, image.height, crop.zoom, crop.x, crop.y);
  const context = target.getContext('2d', { alpha: false });
  context.fillStyle = '#17191c';
  context.fillRect(0, 0, target.width, target.height);
  context.drawImage(image, x, y, w, h, 0, 0, target.width, target.height);
}

// Thumbnails are made one at a time so a big selection never holds many full photos in memory.
let thumbnailing = Promise.resolve();
function thumbnail(item) {
  thumbnailing = thumbnailing.then(async () => {
    if (!items.includes(item)) return;
    try {
      const image = await decode(item.file);
      item.size = { width: image.width, height: image.height };
      paint(item.canvas, image, item.crop);
      image.close();
    } catch (error) {
      item.state = 'error';
      item.error = error.message || 'Could not open this image';
    }
    render();
  });
}

function add(files) {
  if (busy) return;
  const room = free === null ? null : Math.max(0, free);
  const pending = items.filter((item) => item.state !== 'done');
  const { accepted, note } = admit(files, pending, room);
  for (const file of accepted) {
    const preview = document.createElement('canvas');
    preview.width = 320;
    preview.height = 180;
    const item = {
      id: ++next,
      file,
      crop: { zoom: 1, x: 0.5, y: 0.5 },
      canvas: preview,
      state: 'ready',
    };
    items.push(item);
    thumbnail(item);
  }
  element('success').hidden = true;
  message(note, Boolean(note) && !accepted.length);
  render();
}

for (const input of [element('file'), element('more-file'), folder])
  input.addEventListener('change', () => {
    add([...input.files]);
    input.value = '';
  });

function render() {
  const pending = items.filter((item) => item.state !== 'done');
  const ready = pending.filter((item) => item.state === 'ready' || item.state === 'failed');
  picker.hidden = items.length > 0;
  element('folder-picker').hidden = items.length > 0 || !('webkitdirectory' in folder);
  queueView.hidden = items.length === 0;
  element('intro').hidden = items.length > 0;
  element('queue-count').textContent = plural(pending.length, 'photo');
  element('queue-room').textContent =
    free === null ? '' : `${plural(Math.max(0, free), 'free slot')} on the TV`;
  submit.textContent = busy
    ? 'Sending…'
    : ready.length
      ? `Save ${ready.length === 1 ? 'it' : `all ${ready.length}`} to TV`
      : 'Save to TV';
  submit.disabled = busy || ready.length === 0;
  element('more').classList.toggle('disabled', busy);
  element('more-file').disabled = busy;
  const shown = new Set();
  for (const item of items) {
    shown.add(String(item.id));
    let tile = grid.querySelector(`[data-id="${item.id}"]`);
    if (!tile) {
      tile = document.createElement('li');
      tile.dataset.id = item.id;
      const open = document.createElement('button');
      open.type = 'button';
      open.className = 'tile';
      open.setAttribute('aria-label', `Adjust ${item.file.name}`);
      open.append(item.canvas);
      open.addEventListener('click', () => edit(item));
      const remove = document.createElement('button');
      remove.type = 'button';
      remove.className = 'remove';
      remove.setAttribute('aria-label', `Remove ${item.file.name}`);
      remove.textContent = '×';
      remove.addEventListener('click', () => drop(item));
      const badge = document.createElement('span');
      badge.className = 'badge';
      tile.append(open, remove, badge);
      grid.append(tile);
    }
    tile.className = item.state;
    tile.title = item.error || item.file.name;
    tile.querySelector('.badge').textContent =
      { sending: '…', done: '✓', failed: '!', error: '!' }[item.state] ?? '';
    tile.querySelector('.remove').hidden = busy || item.state === 'done';
    tile.querySelector('.tile').disabled = busy || item.state === 'done' || item.state === 'error';
  }
  for (const tile of [...grid.children]) if (!shown.has(tile.dataset.id)) tile.remove();
}

function drop(item) {
  if (busy) return;
  items = items.filter((other) => other !== item);
  if (editing?.item === item) closeEditor();
  if (!items.length) message('');
  render();
}

async function edit(item) {
  if (busy || item.state === 'error' || item.state === 'done') return;
  try {
    const image = await decode(item.file);
    editing?.image.close();
    editing = { item, image, crop: { ...item.crop } };
    zoom.value = String(editing.crop.zoom);
    draw();
    editor.showModal();
  } catch (error) {
    item.state = 'error';
    item.error = error.message || 'Could not open this image';
    render();
  }
}

function draw() {
  if (editing) paint(canvas, editing.image, editing.crop);
}

function closeEditor() {
  if (editor.open) editor.close();
}

editor.addEventListener('close', () => {
  if (!editing) return;
  if (editor.returnValue === 'done') {
    editing.item.crop = editing.crop;
    paint(editing.item.canvas, editing.image, editing.crop);
  }
  editing.image.close();
  editing = undefined;
  editor.returnValue = '';
});
zoom.addEventListener('input', () => {
  if (!editing) return;
  editing.crop.zoom = Number(zoom.value);
  draw();
});
element('reset').addEventListener('click', () => {
  if (!editing) return;
  editing.crop = { zoom: 1, x: 0.5, y: 0.5 };
  zoom.value = '1';
  draw();
});
element('remove').addEventListener('click', () => {
  if (editing) drop(editing.item);
});
canvas.addEventListener('pointerdown', (event) => {
  if (!editing) return;
  canvas.setPointerCapture(event.pointerId);
  drag = { x: event.clientX, y: event.clientY };
});
canvas.addEventListener('pointermove', (event) => {
  if (!drag || !editing) return;
  const { image, crop } = editing;
  const rect = canvas.getBoundingClientRect();
  const current = cropRect(image.width, image.height, crop.zoom, crop.x, crop.y);
  crop.x = panPosition(crop.x, event.clientX - drag.x, image.width, current.w, rect.width);
  crop.y = panPosition(crop.y, event.clientY - drag.y, image.height, current.h, rect.height);
  drag = { x: event.clientX, y: event.clientY };
  draw();
});
for (const event of ['pointerup', 'pointercancel', 'lostpointercapture'])
  canvas.addEventListener(event, () => {
    drag = undefined;
  });

async function send(item) {
  const image = await decode(item.file);
  const output = document.createElement('canvas');
  output.width = 1920;
  output.height = 1080;
  paint(output, image, item.crop);
  image.close();
  const blob = await new Promise((resolve) => output.toBlob(resolve, 'image/jpeg', 0.94));
  output.width = output.height = 0;
  if (!blob) throw new Error('This photo could not be prepared');
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 45000);
  try {
    const response = await fetch('upload', {
      method: 'POST',
      headers: {
        'Content-Type': 'image/jpeg',
        'X-Luma-Upload': token,
        'X-Luma-Collection': encodeURIComponent(lastCollection),
      },
      body: blob,
      signal: controller.signal,
    });
    const result = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(result.error || 'The TV could not save this photo');
  } finally {
    clearTimeout(timeout);
  }
}

submit.addEventListener('click', async () => {
  if (busy) return;
  const destination = await info;
  if (!destination) {
    message('Could not reach the TV. Close and reopen the upload dialog.', true);
    return;
  }
  const batch = items.filter((item) => item.state === 'ready' || item.state === 'failed');
  if (!batch.length) return;
  busy = true;
  closeEditor();
  const bar = element('progress-bar');
  element('progress').hidden = false;
  let sent = 0;
  let lost;
  render();
  for (const [index, item] of batch.entries()) {
    message(`Sending ${index + 1} of ${batch.length}…`);
    bar.style.width = `${(index / batch.length) * 100}%`;
    item.state = 'sending';
    render();
    try {
      await send(item);
      item.state = 'done';
      item.error = undefined;
      sent += 1;
      if (free !== null) free -= 1;
    } catch (error) {
      item.state = 'failed';
      item.error =
        error.name === 'AbortError'
          ? 'The TV did not respond in time'
          : error instanceof TypeError
            ? 'Connection lost'
            : error.message;
      // A lost connection usually means the TV dialog closed; stop instead of failing the rest.
      if (error instanceof TypeError || error.name === 'AbortError') {
        lost = item.error;
        render();
        break;
      }
      if (/gallery is full/i.test(item.error)) {
        free = 0;
        lost = item.error;
        render();
        break;
      }
    }
    render();
  }
  bar.style.width = '100%';
  busy = false;
  element('progress').hidden = true;
  const failed = items.filter((item) => item.state === 'failed' || item.state === 'error');
  const waiting = items.filter((item) => item.state === 'ready');
  if (!failed.length && !waiting.length) {
    element('success-title').textContent = `${plural(sent, 'background')} added`;
    element('success-detail').textContent =
      lastCollection === 'Photos'
        ? 'Your backgrounds were uploaded to'
        : `Added to ${lastCollection}. Stored on the TV in`;
    try {
      localStorage.setItem('luma-collection', lastCollection);
    } catch {
      /* private browsing */
    }
    element('success-path').textContent = destination.path;
    element('success').hidden = false;
    queueView.hidden = true;
    items = [];
    grid.replaceChildren();
    message('');
    render();
    queueView.hidden = true;
    picker.hidden = true;
    element('folder-picker').hidden = true;
    element('intro').hidden = true;
    return;
  }
  items = items.filter((item) => item.state !== 'done');
  render();
  const parts = [];
  if (sent) parts.push(`Saved ${sent}.`);
  if (lost) parts.push(`${lost} — keep the TV upload dialog open, then try again.`);
  else if (failed.length)
    parts.push(
      `${plural(failed.length, 'photo')} couldn’t be saved. Tap Save to retry or × to drop.`,
    );
  message(parts.join(' '), true);
});

element('another').addEventListener('click', () => {
  element('success').hidden = true;
  message('');
  render();
  element('file').click();
});
