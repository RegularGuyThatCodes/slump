import React from 'react';
import { Link, Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import Settings from './Settings';
import Connect from './Connect';

export default function Dashboard() {
  const nav = useNavigate();

  const logout = async () => {
    await window.slump.logout();
    nav('/login', { replace: true });
  };

  return (
    <div className="h-screen grid grid-rows-[auto,1fr]">
      <header className="flex items-center justify-between px-6 py-4 border-b border-gray-700 bg-gray-800">
        <div className="font-bold">Slump</div>
        <nav className="space-x-4">
          <Link to="/app/connect" className="hover:underline">Connect</Link>
          <Link to="/app/settings" className="hover:underline">Settings</Link>
          <button onClick={logout} className="ml-4 px-3 py-1 rounded bg-red-600 hover:bg-red-500">Logout</button>
        </nav>
      </header>
      <main className="p-6 overflow-auto">
        <Routes>
          <Route path="/connect" element={<Connect />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="*" element={<Navigate to="/app/connect" replace />} />
        </Routes>
      </main>
    </div>
  );
}
