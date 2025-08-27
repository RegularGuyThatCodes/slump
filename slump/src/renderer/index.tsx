import React from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import './styles.css';
import Login from './screens/Login';
import Dashboard from './screens/Dashboard';

const App = () => {
  const [isAuthed, setAuthed] = React.useState<boolean>(false);

  React.useEffect(() => {
    window.slump.authStatus().then(setAuthed);
    const off = window.slump.onOAuthSuccess(() => setAuthed(true));
    return () => off();
  }, []);

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<Login onAuthed={() => setAuthed(true)} />} />
        <Route path="/app" element={isAuthed ? <Dashboard /> : <Navigate to="/login" replace />} />
        <Route path="*" element={<Navigate to={isAuthed ? '/app' : '/login'} replace />} />
      </Routes>
    </BrowserRouter>
  );
};

const root = createRoot(document.getElementById('root')!);
root.render(<App />);
