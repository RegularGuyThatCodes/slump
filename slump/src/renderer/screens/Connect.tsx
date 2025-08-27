import React from 'react';

export default function Connect() {
  const [status, setStatus] = React.useState('Idle');
  const [stats, setStats] = React.useState<{ bitrate_kbps: number; latency_ms: number } | null>(null);
  const pollRef = React.useRef<number | null>(null);

  const start = async () => {
    try {
      setStatus('Starting…');
      const ok = await window.slump.startStream(12000, 1920, 1080, 90);
      if (!ok) throw new Error('startStream returned false');
      setStatus('Connected');
      if (pollRef.current) window.clearInterval(pollRef.current);
      pollRef.current = window.setInterval(() => {
        try {
          const s = window.slump.getStats();
          setStats(s);
        } catch {}
      }, 1000);
    } catch (e: any) {
      setStatus(`Error: ${e?.message || 'Failed to start'}`);
    }
  };

  const stop = async () => {
    try {
      setStatus('Stopping…');
      await window.slump.stopStream();
    } finally {
      setStatus('Idle');
      if (pollRef.current) {
        window.clearInterval(pollRef.current);
        pollRef.current = null;
      }
      setStats(null);
    }
  };

  React.useEffect(() => () => { if (pollRef.current) window.clearInterval(pollRef.current); }, []);

  return (
    <div className="max-w-2xl">
      <h2 className="text-xl font-bold mb-4">Connect to Quest</h2>
      <div className="flex items-center gap-4 mb-4">
        <button onClick={start} className="px-4 py-2 rounded bg-green-600 hover:bg-green-500">Start</button>
        <button onClick={stop} className="px-4 py-2 rounded bg-yellow-600 hover:bg-yellow-500">Stop</button>
        <span className="text-gray-300">Status: {status}</span>
      </div>
      {stats && (
        <div className="text-sm text-gray-300 mb-2">
          Bitrate: {stats.bitrate_kbps} kbps · Latency: {stats.latency_ms} ms
        </div>
      )}
      <div className="text-sm text-gray-400">Ensure your Quest is on the same network. Slump will use WebRTC with STUN for connectivity.</div>
    </div>
  );
}
