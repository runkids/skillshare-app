import { describe, expect, it } from 'vitest';
import { detectPlatform } from './platform';

const MAC_UA =
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)';
const WIN_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko)';
const LINUX_UA = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko)';

describe('detectPlatform', () => {
  it('detects macOS from the user agent', () => {
    expect(detectPlatform({ userAgent: MAC_UA })).toBe('macos');
  });

  it('detects Windows from the user agent', () => {
    expect(detectPlatform({ userAgent: WIN_UA })).toBe('windows');
  });

  it('falls back to Linux', () => {
    expect(detectPlatform({ userAgent: LINUX_UA })).toBe('linux');
  });

  it('prefers userAgentData.platform when available', () => {
    expect(detectPlatform({ userAgent: MAC_UA, userAgentData: { platform: 'Windows' } })).toBe(
      'windows'
    );
  });
});
