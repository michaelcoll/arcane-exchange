import { createReadStream } from 'node:fs';
import { CARD_IMAGE_CACHE_CONTROL, findCardImage } from '../../utils/cardImages';

/**
 * Serves the card images the backend downloads into the shared `CARD_IMAGES_DIR` folder.
 * A missing image (card still pending) is a 404 that must not be cached, or Cloudflare would
 * keep serving the absence once the image exists.
 */
export default defineEventHandler(async (event) => {
  const name = getRouterParam(event, 'file', { decode: true }) ?? '';
  const image = await findCardImage(process.env.CARD_IMAGES_DIR, name);

  if (!image) {
    setResponseStatus(event, 404);
    setResponseHeaders(event, { 'Content-Type': 'text/plain', 'Cache-Control': 'no-store' });
    return 'Not found';
  }

  setResponseHeaders(event, {
    'Content-Type': 'image/webp',
    'Content-Length': image.size,
    'Cache-Control': CARD_IMAGE_CACHE_CONTROL,
  });
  return sendStream(event, createReadStream(image.path));
});
