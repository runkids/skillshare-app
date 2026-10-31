import { describe, expect, it } from 'vitest';
import { releaseNoteItems } from './release-notes';

describe('releaseNoteItems', () => {
  it('turns a Release Please body into plain bullet text', () => {
    const body = [
      '## [0.7.6](https://github.com/runkids/skillshare-app/compare/v0.7.5...v0.7.6) (2026-10-30)',
      '',
      '### Bug Fixes',
      '',
      '* **webview:** reload the CLI page that was open, not the dashboard ([#89](https://x/89)) ([291d40a](https://x/291d40a))',
      '- plain item',
    ].join('\n');
    expect(releaseNoteItems(body)).toEqual([
      'webview: reload the CLI page that was open, not the dashboard',
      'plain item',
    ]);
  });

  it('returns nothing for an empty body', () => {
    expect(releaseNoteItems(undefined)).toEqual([]);
  });
});
