import { stat } from 'node:fs/promises';
import { join } from 'node:path';

/**
 * Card images never change under a given URL: the URL exposed to clients is versioned by the
 * image source, so Cloudflare and browsers may keep them for a year.
 */
export const CARD_IMAGE_CACHE_CONTROL = 'public, max-age=31536000, immutable';

/**
 * `{SET}_{number}_{LANGUAGE}.webp`, suffixed `_back` for a back, as written by the backend
 * (`card_image_file_name`): the collector number is escaped to `[A-Za-z0-9~-]`, so a valid name
 * can never leave the folder.
 */
const CARD_IMAGE_FILE_NAME = /^[A-Za-z0-9]+_[A-Za-z0-9~-]+_[A-Z]{2}(_back)?\.webp$/;

export function isCardImageFileName(name: string): boolean {
  return CARD_IMAGE_FILE_NAME.test(name);
}

/** The stored image `name` in `dir`, or `null` when the card is still pending. */
export async function findCardImage(
  dir: string | undefined,
  name: string,
): Promise<{ path: string; size: number } | null> {
  if (!dir || !isCardImageFileName(name)) return null;
  const path = join(dir, name);
  try {
    const stats = await stat(path);
    return stats.isFile() ? { path, size: stats.size } : null;
  } catch {
    return null;
  }
}
