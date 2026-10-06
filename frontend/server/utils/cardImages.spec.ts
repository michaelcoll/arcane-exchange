import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { findCardImage, isCardImageFileName } from './cardImages';

describe('isCardImageFileName', () => {
  it.each(['FDN_87_FR.webp', 'ISD_51_EN_back.webp', 'PLST_SLD-1~E2~98~85_EN.webp'])(
    'accepts %s',
    (name) => {
      expect(isCardImageFileName(name)).toBe(true);
    },
  );

  it.each([
    '../FDN_87_FR.webp',
    'FDN_87_FR.png',
    'FDN_87.webp',
    '.FDN_87_FR.webp.1234.tmp',
    'FDN_87_FR_front.webp',
    'FDN/87_FR.webp',
    '',
  ])('rejects %s', (name) => {
    expect(isCardImageFileName(name)).toBe(false);
  });
});

describe('findCardImage', () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), 'card-images-'));
    await writeFile(join(dir, 'FDN_87_FR.webp'), 'webp');
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it('finds a stored image', async () => {
    expect(await findCardImage(dir, 'FDN_87_FR.webp')).toEqual({
      path: join(dir, 'FDN_87_FR.webp'),
      size: 4,
    });
  });

  it('does not find a pending card image', async () => {
    expect(await findCardImage(dir, 'FDN_88_FR.webp')).toBeNull();
  });

  it('does not look outside the folder', async () => {
    expect(await findCardImage(join(dir, 'sub'), '../FDN_87_FR.webp')).toBeNull();
  });

  it('does not find anything without a folder', async () => {
    expect(await findCardImage(undefined, 'FDN_87_FR.webp')).toBeNull();
  });
});
