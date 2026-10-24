import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles.css';
import { applyTheme, loadTheme } from './theme';

// Apply the saved colours before the first paint so the app never flashes the default orange.
applyTheme(loadTheme(), false);

createRoot(document.getElementById('root')!).render(
  <React.StrictMode><App /></React.StrictMode>,
);
