import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { open } from '@tauri-apps/plugin-opener';
import { terminalOptions } from '../components/terminal/terminalTheme';
import '@xterm/xterm/css/xterm.css';

export interface TerminalInstanceHandle {
  terminal: Terminal;
  fitAddon: FitAddon;
  searchAddon: SearchAddon;
  webglAddon: WebglAddon | null;
}

/**
 * Creates a Terminal instance with theme/options from terminalTheme.ts,
 * FitAddon, SearchAddon, and WebLinksAddon loaded.
 */
export function createTerminalInstance(options?: {
  onData?: (data: string) => void;
}): TerminalInstanceHandle {
  const terminal = new Terminal(terminalOptions);

  const fitAddon = new FitAddon();
  const searchAddon = new SearchAddon();
  const webLinksAddon = new WebLinksAddon((_, url) => {
    void open(url);
  });

  terminal.loadAddon(fitAddon);
  terminal.loadAddon(searchAddon);
  terminal.loadAddon(webLinksAddon);

  if (options?.onData) {
    terminal.onData(options.onData);
  }

  return { terminal, fitAddon, searchAddon, webglAddon: null };
}

/**
 * Opens the terminal in a DOM container, fits it, lazy-loads the WebGL addon,
 * and sets up a ResizeObserver with 50ms debounce for auto-fit.
 * Returns a cleanup function.
 */
export function mountTerminal(
  handle: TerminalInstanceHandle,
  container: HTMLElement,
): () => void {
  const { terminal, fitAddon } = handle;

  terminal.open(container);
  fitAddon.fit();

  // Lazy-load WebGL addon
  let webglAddon: WebglAddon | null = null;
  try {
    webglAddon = new WebglAddon();
    webglAddon.onContextLoss(() => {
      webglAddon?.dispose();
      handle.webglAddon = null;
    });
    terminal.loadAddon(webglAddon);
    handle.webglAddon = webglAddon;
  } catch {
    // WebGL not supported; fall back to canvas renderer
    webglAddon = null;
  }

  // ResizeObserver with 50ms debounce
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  const resizeObserver = new ResizeObserver(() => {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
      fitAddon.fit();
      debounceTimer = null;
    }, 50);
  });
  resizeObserver.observe(container);

  return () => {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
    }
    resizeObserver.disconnect();
  };
}

export interface BufferedWriter {
  write: (data: string) => void;
  dispose: () => void;
}

/**
 * Batches PTY output via requestAnimationFrame to prevent backpressure.
 */
export function createBufferedWriter(terminal: Terminal): BufferedWriter {
  let buffer = '';
  let rafScheduled = false;

  function flush() {
    if (buffer.length > 0) {
      terminal.write(buffer);
      buffer = '';
    }
    rafScheduled = false;
  }

  function write(data: string) {
    buffer += data;
    if (!rafScheduled) {
      rafScheduled = true;
      requestAnimationFrame(flush);
    }
  }

  function dispose() {
    buffer = '';
    rafScheduled = false;
  }

  return { write, dispose };
}

/**
 * Disposes all addons and the terminal instance.
 */
export function disposeTerminalInstance(handle: TerminalInstanceHandle): void {
  handle.webglAddon?.dispose();
  handle.searchAddon.dispose();
  handle.fitAddon.dispose();
  handle.terminal.dispose();
}
