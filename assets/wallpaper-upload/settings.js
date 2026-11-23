// Settings backup: download the TV's saved settings, or send a saved copy back.
// The TV checks a restored file before using it and never includes the Unsplash key.
const element = (id) => document.getElementById(id);
const token = location.pathname.split('/')[1];
const status = element('status');

function message(text, error = false) {
  status.textContent = text;
  status.classList.toggle('error', error);
}

function today() {
  const date = new Date();
  const pad = (value) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

export async function download(fetcher = fetch) {
  const response = await fetcher('settings.json', { cache: 'no-store' });
  if (!response.ok) throw new Error((await response.text()) || 'The TV has no saved settings yet.');
  return new Blob([await response.text()], { type: 'application/json' });
}

export async function restore(file, fetcher = fetch) {
  if (file.size > 512 * 1024) throw new Error('That file is too large to be Luma settings.');
  const response = await fetcher('settings/restore', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Luma-Upload': token },
    body: await file.text(),
  });
  if (!response.ok) throw new Error((await response.text()) || 'The TV refused that file.');
}

const save = element('backup-save');
const file = element('backup-file');
if (save && file) {
  save.addEventListener('click', async () => {
    try {
      const blob = await download();
      const link = document.createElement('a');
      link.href = URL.createObjectURL(blob);
      link.download = `my-luma-settings-${today()}.json`;
      document.body.append(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(link.href), 10_000);
      message('Settings downloaded. Keep the file somewhere safe.');
    } catch (error) {
      message(error.message, true);
    }
  });
  file.addEventListener('change', async () => {
    const chosen = file.files?.[0];
    file.value = '';
    if (!chosen) return;
    if (!confirm("Replace all of Luma's settings on the TV with this backup?")) return;
    try {
      await restore(chosen);
      message('Settings restored on the TV.');
    } catch (error) {
      message(error.message, true);
    }
  });
}
