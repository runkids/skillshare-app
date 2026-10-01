import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './index.css';
import App from './App.tsx';
import { ThemeProvider } from './context/ThemeContext';
import QuickActionsPalette from './desktop/components/quick-actions/QuickActionsPalette';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {new URLSearchParams(window.location.search).has('quick-actions') ? (
      <ThemeProvider>
        <QuickActionsPalette />
      </ThemeProvider>
    ) : (
      <App />
    )}
  </StrictMode>
);
