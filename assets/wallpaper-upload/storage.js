export async function loadInfo(fetcher) {
  const response = await fetcher('info.json', { cache: 'no-store' });
  if (!response.ok) throw new Error('Storage location unavailable');
  const info = await response.json();
  if (typeof info.path !== 'string' || !info.path.startsWith('/')) {
    throw new Error('Invalid storage location');
  }
  const free = Number.isInteger(info.free) && info.free >= 0 ? info.free : null;
  return { path: info.path, free };
}

export async function loadStoragePath(fetcher) {
  return (await loadInfo(fetcher)).path;
}
