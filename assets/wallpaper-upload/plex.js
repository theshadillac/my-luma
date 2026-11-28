// Spotlight's Plex token: checked by the TV against the Plex server and saved there only.
// The page never gets the token back.
import { loadInfo } from './storage.js';

const element = (id) => document.getElementById(id);
const token = location.pathname.split('/')[1];
const form = element('plex-form');
const forget = element('plex-forget');
const state = element('plex-state');
const status = element('status');
let busy = false;

function message(text, error = false) {
  status.textContent = text;
  status.classList.toggle('error', error);
}

function ready(saved) {
  form.hidden = saved;
  forget.hidden = !saved;
  state.textContent = saved
    ? 'Your Plex token is saved on the TV. Spotlight shows your Plex rows.'
    : 'Spotlight shows Continue watching, Up next and Recently added from your Plex server. ' +
      'Paste your Plex token; it is saved on the TV only and never shown again.';
}

async function save(body) {
  const response = await fetch('plex/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Luma-Upload': token },
    body: JSON.stringify(body),
  });
  const result = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(result.error || 'Something went wrong');
  return result;
}

loadInfo(fetch)
  .then((info) => ready(Boolean(info.plex)))
  .catch(() => ready(false));

form.addEventListener('submit', async (event) => {
  event.preventDefault();
  const input = element('plex-token');
  if (!input.value.trim() || busy) return;
  busy = true;
  message('Checking the token with your Plex server…');
  try {
    await save({ token: input.value, server: element('plex-server').value });
    input.value = '';
    ready(true);
    message('Plex token saved on the TV.');
  } catch (error) {
    message(error.message, true);
  } finally {
    busy = false;
  }
});

forget.addEventListener('click', async () => {
  if (busy || !confirm('Remove the Plex token from the TV?')) return;
  await save({ token: '', server: '' }).catch(() => {});
  ready(false);
  message('Plex token removed.');
});
