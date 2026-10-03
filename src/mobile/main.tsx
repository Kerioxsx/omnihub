import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@shared/theme.css';

function App() {
  return <div className="p-8 text-fg">OmniHub phone</div>;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
