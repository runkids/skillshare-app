/**
 * Script PTY Terminal Component - Multi-tab PTY terminal manager
 * Feature 008: Interactive terminal with full PTY support
 *
 * Replaces ScriptTerminal with xterm.js + tauri-plugin-pty for full interactive support
 */

import { useRef, useEffect, useCallback, useState, forwardRef, useImperativeHandle } from 'react';
import { WebglAddon } from '@xterm/addon-webgl';
import { spawn } from 'tauri-pty';
import {
  ChevronDown,
  ChevronUp,
  GripHorizontal,
  Copy,
  Check,
  Search,
  Trash2,
  Terminal as TerminalIcon,
} from 'lucide-react';
import { scriptAPI } from '../../lib/tauri-api';
import { useSettings } from '../../contexts/SettingsContext';
import { Button } from '../ui/Button';
import { usePtySessions, terminalTheme, type PtySession } from '../../hooks/usePtySessions';
import { TerminalTab } from './TerminalTab';
import { TerminalSearchBar } from './TerminalSearchBar';
import { TerminalStatusBar } from './TerminalStatusBar';
import '@xterm/xterm/css/xterm.css';

interface ScriptPtyTerminalProps {
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  // Feature 008: PTY integration with ScriptExecutionContext
  onRegisterPtyExecution?: (
    sessionId: string,
    scriptName: string,
    projectPath: string,
    projectName?: string
  ) => void;
  onUpdatePtyOutput?: (sessionId: string, output: string) => void;
  onUpdatePtyStatus?: (
    sessionId: string,
    status: 'running' | 'completed' | 'failed',
    exitCode?: number
  ) => void;
  onRemovePtyExecution?: (sessionId: string) => void;
  // Feature 008: Kill all PTY sessions signal
  killAllPtySignal?: number;
  // Feature 008: Port info from ScriptExecutionContext for tab display
  sessionPorts?: Map<string, number | undefined>;
  // Direct kill function registration (for beforeunload sync call)
  onRegisterKillAllFn?: (fn: () => void) => void;
}

// Ref methods exposed to parent
export interface ScriptPtyTerminalRef {
  spawnSession: (
    command: string,
    args: string[],
    cwd: string,
    name: string,
    projectName?: string
  ) => Promise<string | null>;
  killSession: (sessionId: string) => void;
  killAllSessions: () => void;
  sessions: Map<string, PtySession>;
  activeSessionId: string | null;
}

// Throttle function to limit execution rate
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function throttle<T extends (...args: any[]) => void>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void {
  let lastCall = 0;
  let pendingArgs: Parameters<T> | null = null;
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  return (...args: Parameters<T>) => {
    const now = Date.now();
    const timeSinceLastCall = now - lastCall;

    if (timeSinceLastCall >= delay) {
      lastCall = now;
      fn(...args);
    } else {
      // Store latest args and schedule execution
      pendingArgs = args;
      if (!timeoutId) {
        timeoutId = setTimeout(() => {
          if (pendingArgs) {
            lastCall = Date.now();
            fn(...pendingArgs);
            pendingArgs = null;
          }
          timeoutId = null;
        }, delay - timeSinceLastCall);
      }
    }
  };
}

export const ScriptPtyTerminal = forwardRef<ScriptPtyTerminalRef, ScriptPtyTerminalProps>(
  function ScriptPtyTerminal(
    {
      isCollapsed,
      onToggleCollapse,
      onRegisterPtyExecution,
      onUpdatePtyOutput,
      onUpdatePtyStatus,
      onRemovePtyExecution,
      killAllPtySignal,
      sessionPorts,
      onRegisterKillAllFn,
    },
    ref
  ) {
    const containerRef = useRef<HTMLDivElement>(null);
    const terminalContainerRef = useRef<HTMLDivElement>(null);
    const searchInputRef = useRef<HTMLInputElement>(null);

    // Use extracted hook for session management
    const {
      sessions,
      activeSessionId,
      activeSession,
      sessionList,
      setActiveSessionId,
      setSessions,
      spawnSession,
      killSession,
      killAllSessions,
      pendingSpawnsRef,
    } = usePtySessions({
      onRegisterPtyExecution,
      onRemovePtyExecution,
    });

    const [isResizing, setIsResizing] = useState(false);
    const [copied, setCopied] = useState(false);
    const [isSearchOpen, setIsSearchOpen] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const startYRef = useRef(0);
    const startHeightRef = useRef(0);

    // Settings context for path formatting and terminal height
    const { formatPath, terminalHeight: savedHeight, setTerminalHeight } = useSettings();
    const MIN_HEIGHT = 100;
    const MAX_HEIGHT = 600;

    // Local height state for smooth dragging - only used during resize
    const [localHeight, setLocalHeight] = useState<number | null>(null);

    // Use local height during resize for instant feedback, otherwise use saved height
    const height = localHeight ?? savedHeight;

    // Feature 008: Store callback refs to avoid stale closures in useEffect
    const onUpdatePtyOutputRef = useRef(onUpdatePtyOutput);
    const onUpdatePtyStatusRef = useRef(onUpdatePtyStatus);
    useEffect(() => {
      onUpdatePtyOutputRef.current = onUpdatePtyOutput;
      onUpdatePtyStatusRef.current = onUpdatePtyStatus;
    }, [onUpdatePtyOutput, onUpdatePtyStatus]);

    // Expose methods to parent via ref
    useImperativeHandle(
      ref,
      () => ({
        spawnSession,
        killSession,
        killAllSessions,
        sessions,
        activeSessionId,
      }),
      [spawnSession, killSession, killAllSessions, sessions, activeSessionId]
    );

    // Feature 008: Listen to kill all PTY signal from "Stop All Processes"
    const prevKillSignalRef = useRef(killAllPtySignal);
    useEffect(() => {
      // Only trigger if signal changed (not on initial mount)
      if (killAllPtySignal !== undefined && prevKillSignalRef.current !== killAllPtySignal) {
        prevKillSignalRef.current = killAllPtySignal;
        killAllSessions();
      }
    }, [killAllPtySignal, killAllSessions]);

    // Register killAllSessions for direct call (beforeunload sync call)
    useEffect(() => {
      onRegisterKillAllFn?.(killAllSessions);
    }, [killAllSessions, onRegisterKillAllFn]);

    // Mount terminal to DOM when active session changes or when expanding
    useEffect(() => {
      if (!terminalContainerRef.current || !activeSession?.terminal || isCollapsed) return;

      const container = terminalContainerRef.current;
      const term = activeSession.terminal;
      const sessionId = activeSession.id;

      // Check if terminal is already mounted somewhere (has been opened before)
      const termElement = term.element;

      if (termElement) {
        // Terminal has been opened before
        if (container.contains(termElement)) {
          // Already in this container, just fit
          requestAnimationFrame(() => {
            activeSession.fitAddon?.fit();
            term.focus();
            if (activeSession.pty) {
              activeSession.pty.resize(term.cols, term.rows);
            }
          });
          return;
        } else {
          // Terminal was opened but in a different/old container
          // We need to move it to the new container
          container.innerHTML = '';
          container.appendChild(termElement);
          requestAnimationFrame(() => {
            activeSession.fitAddon?.fit();
            term.focus();
            if (activeSession.pty) {
              activeSession.pty.resize(term.cols, term.rows);
            }
          });
          return;
        }
      }

      // Fresh terminal - clear container and open
      container.innerHTML = '';
      term.open(container);

      // Load WebGL addon for better rendering performance (must be after terminal is opened)
      if (!activeSession.webglAddon) {
        try {
          const webglAddon = new WebglAddon();
          webglAddon.onContextLoss(() => {
            // If WebGL context is lost, dispose and fall back to canvas
            webglAddon.dispose();
          });
          term.loadAddon(webglAddon);
          // Update session with webglAddon
          setSessions((prev) => {
            const next = new Map(prev);
            const s = next.get(sessionId);
            if (s) {
              next.set(sessionId, { ...s, webglAddon });
            }
            return next;
          });
        } catch (e) {
          console.warn('WebGL addon failed to load, using canvas renderer:', e);
        }
      }

      // Fit and focus
      requestAnimationFrame(() => {
        activeSession.fitAddon?.fit();
        term.focus();

        // Check if there's a pending spawn for this session
        const pendingSpawn = pendingSpawnsRef.current.get(sessionId);
        if (pendingSpawn && !activeSession.pty) {
          // Remove from pending
          pendingSpawnsRef.current.delete(sessionId);

          // Now spawn PTY with correct terminal dimensions
          const { command, args, cwd } = pendingSpawn;

          (async () => {
            try {
              // Get environment variables for proper PATH, VOLTA_HOME, etc.
              const env = await scriptAPI.getPtyEnv();

              // Wrap command to unset Volta internal variables that interfere with node version
              // _VOLTA_TOOL_RECURSION causes Volta shim to skip its logic
              // Use login shell (-l) to source user's profile for consistent environment
              const userShell = env.SHELL || '/bin/zsh';

              // Escape arguments for shell - handle special characters
              const escapeArg = (arg: string): string => {
                // Use single quotes and escape any single quotes within
                // This is safer than double quotes as it prevents variable expansion
                return `'${arg.replace(/'/g, "'\\''")}'`;
              };

              const escapedArgs = args.length > 0 ? ' ' + args.map(escapeArg).join(' ') : '';

              // For fish shell, use different syntax
              const isFish = userShell.includes('fish');
              const wrappedCommand = isFish ? '/bin/bash' : userShell;
              const wrappedArgs = [
                '-l',
                '-c',
                `unset _VOLTA_TOOL_RECURSION; exec ${escapeArg(command)}${escapedArgs}`,
              ];

              const pty = await spawn(wrappedCommand, wrappedArgs, {
                cols: term.cols || 80,
                rows: term.rows || 24,
                cwd,
                env,
              });

              // PTY -> Terminal with batched writes to prevent backpressure
              // Buffer to collect PTY output
              let outputBuffer = '';
              let flushScheduled = false;

              // Throttled callback for port detection (100ms interval)
              const throttledOutputCallback = throttle((output: string) => {
                onUpdatePtyOutputRef.current?.(sessionId, output);
              }, 100);

              // Flush buffer to terminal using requestAnimationFrame for smooth rendering
              const flushBuffer = () => {
                if (outputBuffer) {
                  term.write(outputBuffer);
                  throttledOutputCallback(outputBuffer);
                  outputBuffer = '';
                }
                flushScheduled = false;
              };

              pty.onData((data: string) => {
                // Accumulate output in buffer
                outputBuffer += data;

                // Schedule flush on next animation frame (batches rapid outputs)
                if (!flushScheduled) {
                  flushScheduled = true;
                  requestAnimationFrame(flushBuffer);
                }
              });

              // PTY exit
              pty.onExit(({ exitCode }: { exitCode: number }) => {
                // Flush any remaining buffered output
                if (outputBuffer) {
                  term.write(outputBuffer);
                  outputBuffer = '';
                }
                term.write(`\r\n\x1b[90m[Process exited with code ${exitCode}]\x1b[0m\r\n`);
                const newStatus = exitCode === 0 ? 'completed' : 'failed';
                setSessions((prev) => {
                  const next = new Map(prev);
                  const s = next.get(sessionId);
                  if (s) {
                    next.set(sessionId, {
                      ...s,
                      status: newStatus,
                      exitCode,
                    });
                  }
                  return next;
                });
                // Feature 008: Update status in ScriptExecutionContext for icon state
                onUpdatePtyStatusRef.current?.(sessionId, newStatus, exitCode);
              });

              // Terminal -> PTY
              term.onData((data: string) => {
                pty.write(data);
              });

              // Update session with PTY
              setSessions((prev) => {
                const next = new Map(prev);
                const s = next.get(sessionId);
                if (s) {
                  next.set(sessionId, { ...s, pty });
                }
                return next;
              });
            } catch (err) {
              console.error('Failed to spawn PTY:', err);
              term.write(`\x1b[31mFailed to start: ${err}\x1b[0m\r\n`);
              setSessions((prev) => {
                const next = new Map(prev);
                const s = next.get(sessionId);
                if (s) {
                  next.set(sessionId, { ...s, status: 'failed' });
                }
                return next;
              });
              // Feature 008: Update status in ScriptExecutionContext
              onUpdatePtyStatusRef.current?.(sessionId, 'failed');
            }
          })();
        } else if (activeSession.pty) {
          // PTY already exists, just resize
          activeSession.pty.resize(term.cols, term.rows);
        }
      });
    }, [activeSession, activeSessionId, isCollapsed]);

    // Auto-resize terminal when container size changes
    useEffect(() => {
      if (!terminalContainerRef.current || !activeSession?.terminal || isCollapsed) return;

      const container = terminalContainerRef.current;
      const fitAddon = activeSession.fitAddon;
      const pty = activeSession.pty;
      const term = activeSession.terminal;

      // Debounced fit function to avoid excessive resizing
      let resizeTimeout: ReturnType<typeof setTimeout> | null = null;
      const debouncedFit = () => {
        if (resizeTimeout) clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(() => {
          if (fitAddon && term.element) {
            fitAddon.fit();
            if (pty) {
              pty.resize(term.cols, term.rows);
            }
          }
        }, 50);
      };

      const resizeObserver = new ResizeObserver(debouncedFit);
      resizeObserver.observe(container);

      return () => {
        if (resizeTimeout) clearTimeout(resizeTimeout);
        resizeObserver.disconnect();
      };
    }, [activeSession, isCollapsed]);

    // Handle resize drag
    const handleMouseDown = useCallback(
      (e: React.MouseEvent) => {
        e.preventDefault();
        setIsResizing(true);
        startYRef.current = e.clientY;
        startHeightRef.current = height;
        // Initialize local height for smooth dragging
        setLocalHeight(height);
      },
      [height]
    );

    useEffect(() => {
      if (!isResizing) return;

      const handleMouseMove = (e: MouseEvent) => {
        const delta = startYRef.current - e.clientY;
        const newHeight = Math.min(
          MAX_HEIGHT,
          Math.max(MIN_HEIGHT, startHeightRef.current + delta)
        );
        // Update local height immediately for smooth visual feedback
        setLocalHeight(newHeight);
      };

      const handleMouseUp = () => {
        setIsResizing(false);
        // Save final height to context (persists to DB)
        if (localHeight !== null) {
          setTerminalHeight(localHeight);
          // Clear local height - will now use saved height
          setLocalHeight(null);
        }
        // Fit terminal after resize
        if (activeSession?.fitAddon && activeSession?.pty) {
          activeSession.fitAddon.fit();
          activeSession.pty.resize(
            activeSession.terminal?.cols || 80,
            activeSession.terminal?.rows || 24
          );
        }
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);

      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }, [isResizing, localHeight, setTerminalHeight, activeSession]);

    // Copy terminal content
    const handleCopy = useCallback(async () => {
      if (!activeSession?.terminal) return;

      // Get selection or all content
      const selection = activeSession.terminal.getSelection();

      // For full content, we need to read all lines
      let fullText = '';
      if (!selection) {
        const buffer = activeSession.terminal.buffer.active;
        for (let i = 0; i < buffer.length; i++) {
          const line = buffer.getLine(i);
          if (line) {
            fullText += line.translateToString(true) + '\n';
          }
        }
      }

      try {
        await navigator.clipboard.writeText(selection || fullText);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch (err) {
        console.error('Failed to copy:', err);
      }
    }, [activeSession]);

    // Search decoration options - semi-transparent yellow highlight
    const searchDecorations = {
      matchBackground: 'rgba(234, 179, 8, 0.35)', // yellow-500 with 35% opacity
      matchBorder: 'rgba(202, 138, 4, 0.5)', // yellow-600 with 50% opacity
      matchOverviewRuler: 'rgba(234, 179, 8, 0.6)',
      activeMatchBackground: 'rgba(250, 204, 21, 0.5)', // yellow-400 with 50% opacity (current match)
      activeMatchBorder: 'rgba(234, 179, 8, 0.7)',
      activeMatchColorOverviewRuler: 'rgba(250, 204, 21, 0.8)',
    };

    // Search functions
    const handleSearch = useCallback(
      (query: string) => {
        setSearchQuery(query);
        if (activeSession?.searchAddon && query) {
          activeSession.searchAddon.findNext(query, { decorations: searchDecorations });
        }
      },
      [activeSession]
    );

    const handleSearchNext = useCallback(() => {
      if (activeSession?.searchAddon && searchQuery) {
        activeSession.searchAddon.findNext(searchQuery, { decorations: searchDecorations });
      }
    }, [activeSession, searchQuery]);

    const handleSearchPrev = useCallback(() => {
      if (activeSession?.searchAddon && searchQuery) {
        activeSession.searchAddon.findPrevious(searchQuery, { decorations: searchDecorations });
      }
    }, [activeSession, searchQuery]);

    const handleCloseSearch = useCallback(() => {
      setIsSearchOpen(false);
      setSearchQuery('');
      activeSession?.terminal?.focus();
    }, [activeSession]);

    // Close active session (kill process and remove tab)
    const handleCloseActiveSession = useCallback(() => {
      if (!activeSessionId) return;
      killSession(activeSessionId);
    }, [activeSessionId, killSession]);

    // Keyboard shortcuts
    useEffect(() => {
      const handleKeyDown = (e: KeyboardEvent) => {
        // Only respond when terminal container has focus
        if (
          !containerRef.current?.contains(document.activeElement) &&
          document.activeElement !== document.body
        ) {
          return;
        }

        // Cmd/Ctrl + F to open search
        if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
          e.preventDefault();
          setIsSearchOpen(true);
          setTimeout(() => searchInputRef.current?.focus(), 0);
        }

        // Cmd/Ctrl + Shift + C to copy
        if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'c') {
          e.preventDefault();
          handleCopy();
        }
      };

      document.addEventListener('keydown', handleKeyDown);
      return () => document.removeEventListener('keydown', handleKeyDown);
    }, [handleCopy]);

    // If no sessions, hide terminal completely
    if (sessions.size === 0) {
      return null;
    }

    return (
      <div
        ref={containerRef}
        className={`flex flex-col border-t border-border bg-card ${isResizing ? 'select-none' : ''}`}
        style={{ height: isCollapsed ? 32 : height }}
      >
        {/* Resize Handle - only show when expanded */}
        {!isCollapsed && (
          <div
            onMouseDown={handleMouseDown}
            className={`h-1.5 cursor-ns-resize bg-secondary hover:bg-blue-500/50 transition-colors flex items-center justify-center group ${
              isResizing ? 'bg-blue-500/50' : ''
            }`}
          >
            <div className="w-10 h-0.5 bg-muted-foreground group-hover:bg-blue-400 rounded-full" />
          </div>
        )}
        {/* Header */}
        <div className="h-8 flex items-center px-3 bg-card select-none">
          {isCollapsed ? (
            /* Collapsed state: minimal UI with just expand button */
            <>
              <TerminalIcon className="w-4 h-4 text-muted-foreground mr-2" />
              <span className="text-sm text-muted-foreground flex-1">
                Terminal ({sessions.size})
              </span>
              <Button
                variant="ghost"
                size="icon"
                onClick={onToggleCollapse}
                className="h-auto p-1.5"
                title="Expand"
              >
                <ChevronUp className="w-4 h-4 text-muted-foreground" />
              </Button>
            </>
          ) : (
            /* Expanded state: full UI */
            <>
              <GripHorizontal className="w-4 h-4 text-muted-foreground mr-2" />

              {/* Tabs - using memoized TerminalTab component */}
              <div className="flex-1 flex items-center gap-1 overflow-x-auto">
                {sessionList.map((session) => (
                  <TerminalTab
                    key={session.id}
                    session={session}
                    isActive={activeSessionId === session.id}
                    port={sessionPorts?.get(session.id)}
                    onSelect={setActiveSessionId}
                    onClose={killSession}
                  />
                ))}
              </div>

              {/* Toolbar buttons */}
              <div className="flex items-center gap-1 flex-shrink-0">
                {/* Search button */}
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => {
                    setIsSearchOpen(!isSearchOpen);
                    if (!isSearchOpen) {
                      setTimeout(() => searchInputRef.current?.focus(), 0);
                    }
                  }}
                  className={`h-auto p-1.5 ${isSearchOpen ? 'bg-secondary' : ''}`}
                  title="Search (Cmd+F)"
                >
                  <Search className="w-4 h-4 text-muted-foreground" />
                </Button>
                {/* Copy button */}
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleCopy}
                  disabled={!activeSession}
                  className="h-auto p-1.5"
                  title="Copy output (Cmd+Shift+C)"
                >
                  {copied ? (
                    <Check className="w-4 h-4 text-green-400" />
                  ) : (
                    <Copy className="w-4 h-4 text-muted-foreground" />
                  )}
                </Button>
                {/* Close session button */}
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleCloseActiveSession}
                  disabled={!activeSession}
                  className="h-auto p-1.5 hover:text-red-400"
                  title="Close session (kill process)"
                >
                  <Trash2 className="w-4 h-4 text-muted-foreground" />
                </Button>
                {/* Collapse button */}
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={onToggleCollapse}
                  className="h-auto p-1.5"
                  title="Collapse"
                >
                  <ChevronDown className="w-4 h-4 text-muted-foreground" />
                </Button>
              </div>
            </>
          )}
        </div>

        {/* Search bar */}
        {!isCollapsed && isSearchOpen && (
          <TerminalSearchBar
            searchQuery={searchQuery}
            searchInputRef={searchInputRef}
            onSearchChange={handleSearch}
            onSearchPrev={handleSearchPrev}
            onSearchNext={handleSearchNext}
            onClose={handleCloseSearch}
          />
        )}

        {/* Terminal content - always render container but hide when collapsed */}
        {!isCollapsed && (
          <div
            ref={terminalContainerRef}
            className="flex-1 overflow-hidden terminal-container"
            style={{
              minHeight: 100,
              // Add padding around the terminal content for better visual spacing
              padding: '8px 16px 8px 16px',
              // Use terminal background color to avoid light mode "bleeding" on padding edges
              backgroundColor: terminalTheme.background,
            }}
          />
        )}

        {/* Status bar */}
        {!isCollapsed && activeSession && (
          <TerminalStatusBar session={activeSession} formatPath={formatPath} />
        )}
      </div>
    );
  }
);

// Export spawn function for external use
export { type PtySession };
export default ScriptPtyTerminal;
