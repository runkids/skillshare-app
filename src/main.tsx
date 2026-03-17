import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ShortcutsProvider } from './contexts/ShortcutsContext';
import { WorkflowExecutionProvider } from './contexts/WorkflowExecutionContext';
import { ExecutionHistoryProvider } from './contexts/ExecutionHistoryContext';
import { SettingsProvider } from './contexts/SettingsContext';
import { ThemeProvider } from './contexts/ThemeContext';
import { tauriAPI } from './lib/tauri-api';
import './styles.css';

// Expose APIs for debugging in development
if (import.meta.env.DEV) {
  (window as unknown as { tauriAPI: typeof tauriAPI }).tauriAPI = tauriAPI;
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ThemeProvider>
      <SettingsProvider>
        <WorkflowExecutionProvider>
          <ExecutionHistoryProvider>
            <ShortcutsProvider>
              <App />
            </ShortcutsProvider>
          </ExecutionHistoryProvider>
        </WorkflowExecutionProvider>
      </SettingsProvider>
    </ThemeProvider>
  </React.StrictMode>
);
