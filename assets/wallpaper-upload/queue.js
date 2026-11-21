// Pure helpers for the multi-photo queue, kept DOM-free so they can be tested in Node.
export const TYPES = ['image/jpeg', 'image/png', 'image/webp'];
export const MAX_BYTES = 16 * 1024 * 1024;

const EXTENSIONS = { jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png', webp: 'image/webp' };

// Folder pickers often report an empty type, so fall back to the extension.
export function imageType(file) {
  if (TYPES.includes(file.type)) return file.type;
  if (file.type) return null;
  const extension = file.name.split('.').pop()?.toLowerCase() ?? '';
  return EXTENSIONS[extension] ?? null;
}

// Splits a selection into photos to queue and a short reason for what was skipped.
export function admit(files, queued, free) {
  const seen = new Set(queued.map((item) => key(item.file)));
  const accepted = [];
  let skipped = 0;
  let tooLarge = 0;
  let full = 0;
  for (const file of files) {
    if (file.name.startsWith('.') || !imageType(file)) {
      skipped += 1;
      continue;
    }
    if (file.size > MAX_BYTES) {
      tooLarge += 1;
      continue;
    }
    const id = key(file);
    if (seen.has(id)) continue;
    if (free !== null && queued.length + accepted.length >= free) {
      full += 1;
      continue;
    }
    seen.add(id);
    accepted.push(file);
  }
  const notes = [];
  if (full) notes.push(`${full} didn’t fit — the TV gallery is full`);
  if (tooLarge) notes.push(`${tooLarge} over 16 MiB`);
  if (skipped) notes.push(`${skipped} not JPG, PNG or WebP`);
  return { accepted, note: notes.length ? `Skipped ${notes.join(', ')}.` : '' };
}

function key(file) {
  return `${file.name}\u0000${file.size}\u0000${file.lastModified}`;
}

export function plural(count, word) {
  return `${count} ${word}${count === 1 ? '' : 's'}`;
}
