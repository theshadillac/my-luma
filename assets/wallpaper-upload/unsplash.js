// The Unsplash tab: search on the TV's behalf, choose photos, and add them to a collection.
// The TV keeps the access key and talks to Unsplash; this page only ever sees results.
import { plural } from './queue.js';
import { loadInfo } from './storage.js';

const element = (id) => document.getElementById(id);
const token = location.pathname.split('/')[1];
const tabs = { photos: element('tab-photos'), unsplash: element('tab-unsplash') };
const mine = element('mine');
const panel = element('unsplash');
const keyForm = element('unsplash-key');
const searchForm = element('unsplash-search');
const query = element('unsplash-query');
const grid = element('unsplash-grid');
const more = element('unsplash-more');
const actions = element('unsplash-actions');
const add = element('unsplash-add');
const status = element('status');
const chosen = new Map();
let page = 1;
let lastQuery = '';
let busy = false;

function message(text, error = false) {
  status.textContent = text;
  status.classList.toggle('error', error);
}

function collection() {
  const value = element('collection').value;
  return value && !value.startsWith('\u0000') ? value : 'Photos';
}

async function call(route, body) {
  const response = await fetch(`unsplash/${route}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Luma-Upload': token },
    body: JSON.stringify(body),
  });
  const result = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(result.error || 'Something went wrong');
  return result;
}

function show(tab) {
  const unsplash = tab === 'unsplash';
  tabs.photos.setAttribute('aria-selected', String(!unsplash));
  tabs.unsplash.setAttribute('aria-selected', String(unsplash));
  mine.hidden = unsplash;
  panel.hidden = !unsplash;
  element('success').hidden = true;
  message('');
  if (unsplash && !query.value) {
    const name = collection();
    query.value = name === 'Photos' ? 'landscape' : name;
  }
}
tabs.photos.addEventListener('click', () => show('photos'));
tabs.unsplash.addEventListener('click', () => show('unsplash'));

function ready(hasKey) {
  keyForm.hidden = hasKey;
  searchForm.hidden = !hasKey;
  element('unsplash-footer').hidden = !hasKey;
}
loadInfo(fetch)
  .then((info) => ready(Boolean(info.unsplash)))
  .catch(() => ready(false));

keyForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const input = element('unsplash-key-input');
  if (!input.value.trim() || busy) return;
  busy = true;
  message('Checking the key with Unsplash…');
  try {
    await call('key', { key: input.value });
    input.value = '';
    ready(true);
    message('Key saved on the TV.');
  } catch (error) {
    message(error.message, true);
  } finally {
    busy = false;
  }
});

element('unsplash-forget').addEventListener('click', async () => {
  if (busy || !confirm('Remove the Unsplash key from the TV?')) return;
  await call('key', { key: '' }).catch(() => {});
  grid.replaceChildren();
  chosen.clear();
  render();
  ready(false);
});

function render() {
  const count = chosen.size;
  actions.hidden = count === 0;
  add.textContent = busy ? 'Adding…' : `Add ${count === 1 ? 'it' : count} to ${collection()}`;
  add.disabled = busy || count === 0;
  element('unsplash-tip').hidden = grid.children.length === 0;
}
element('collection').addEventListener('change', render);

function tile(photo) {
  const item = document.createElement('li');
  item.dataset.id = photo.id;
  const pick = document.createElement('button');
  pick.type = 'button';
  pick.className = 'tile';
  pick.style.background = photo.color;
  pick.setAttribute('aria-pressed', 'false');
  pick.setAttribute('aria-label', photo.alt || `Photo by ${photo.name}`);
  const image = document.createElement('img');
  image.src = photo.thumb;
  image.alt = '';
  image.loading = 'lazy';
  pick.append(image);
  pick.addEventListener('click', () => {
    if (busy || item.classList.contains('done')) return;
    if (chosen.has(photo.id)) chosen.delete(photo.id);
    else chosen.set(photo.id, photo);
    const on = chosen.has(photo.id);
    item.classList.toggle('chosen', on);
    pick.setAttribute('aria-pressed', String(on));
    render();
  });
  const badge = document.createElement('span');
  badge.className = 'badge';
  const credit = document.createElement('a');
  credit.className = 'credit';
  credit.href = photo.profile;
  credit.target = '_blank';
  credit.rel = 'noopener';
  credit.textContent = photo.name;
  item.append(pick, badge, credit);
  return item;
}

async function search(next) {
  if (busy) return;
  const text = query.value.trim();
  if (!text) return;
  busy = true;
  page = next ? page + 1 : 1;
  if (!next) {
    grid.replaceChildren();
    chosen.clear();
    lastQuery = text;
  }
  message('Searching…');
  try {
    const { photos } = await call('search', { query: lastQuery, page });
    const known = new Set([...grid.children].map((item) => item.dataset.id));
    grid.append(...photos.filter((photo) => !known.has(photo.id)).map(tile));
    more.hidden = photos.length < 24;
    message(grid.children.length ? '' : 'No photos found. Try other words.');
  } catch (error) {
    message(error.message, true);
  } finally {
    busy = false;
    render();
  }
}
searchForm.addEventListener('submit', (event) => {
  event.preventDefault();
  query.blur();
  search(false);
});
more.addEventListener('click', () => search(true));

add.addEventListener('click', async () => {
  if (busy || chosen.size === 0) return;
  busy = true;
  render();
  const target = collection();
  const queue = [...chosen.keys()];
  let added = 0;
  let failure = '';
  for (const [index, id] of queue.entries()) {
    message(`Adding ${index + 1} of ${queue.length}…`);
    const item = grid.querySelector(`[data-id="${CSS.escape(id)}"]`);
    item?.classList.add('sending');
    try {
      await call('add', { id, collection: target });
      added += 1;
      chosen.delete(id);
      item?.classList.replace('sending', 'done');
      item?.classList.remove('chosen');
    } catch (error) {
      item?.classList.remove('sending');
      failure = error.message;
      if (/limit|full|key/i.test(failure)) break;
    }
  }
  busy = false;
  render();
  const done = added ? `Added ${plural(added, 'photo')} to ${target}.` : '';
  message(failure ? `${done} ${failure}`.trim() : done, Boolean(failure));
});
