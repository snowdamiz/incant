import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
// Fonts are bundled from node_modules into dist/; nothing is fetched from a network.
import '@fontsource-variable/inter';
import '@fontsource-variable/jetbrains-mono';
import './styles/tokens.css';
import './styles/app.css';
import './styles/assets.css';
import { App } from './App';
import { resolveBridge } from './bridge/resolve';

const root = document.getElementById('root');
if (!root) throw new Error('Missing #root element');

// The host injects window.__INCANT_BRIDGE__ before this module runs. Without it,
// the shell shows its "no engine" state unless ?fixture=<variant> was requested.
const resolution = resolveBridge(window.__INCANT_BRIDGE__, window.location.search);

createRoot(root).render(
  <StrictMode>
    <App resolution={resolution} />
  </StrictMode>,
);
