import React from 'react';

export default function Login({ onAuthed }: { onAuthed: () => void }) {
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const handleLogin = async () => {
    setLoading(true);
    setError(null);
    try {
      await window.slump.startOAuth();
      onAuthed();
    } catch (e: any) {
      setError(e?.message || 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="h-screen flex items-center justify-center">
      <div className="w-full max-w-md bg-gray-800 rounded-xl p-8 shadow-xl border border-gray-700">
        <h1 className="text-3xl font-extrabold mb-6">Slump</h1>
        <p className="text-gray-300 mb-6">High-performance Meta Quest connector. Privacy-first: no credential storage.</p>
        {error && <div className="text-red-400 mb-4">{error}</div>}
        <button
          onClick={handleLogin}
          disabled={loading}
          className="w-full py-3 rounded-lg bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 font-semibold"
        >
          {loading ? 'Opening Meta Login…' : 'Continue with Meta'}
        </button>
      </div>
    </div>
  );
}
