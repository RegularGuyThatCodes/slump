import React from 'react';

export default function Settings() {
  const [bitrate, setBitrate] = React.useState(12000);
  const [resolution, setResolution] = React.useState('1920x1080');
  const [framerate, setFramerate] = React.useState(90);

  return (
    <div className="max-w-2xl">
      <h2 className="text-xl font-bold mb-4">Streaming Settings</h2>
      <div className="grid gap-4">
        <label className="grid gap-2">
          <span>Bitrate (kbps)</span>
          <input type="number" className="px-3 py-2 bg-gray-800 border border-gray-700 rounded"
                 value={bitrate} onChange={(e) => setBitrate(parseInt(e.target.value))} />
        </label>
        <label className="grid gap-2">
          <span>Resolution</span>
          <select className="px-3 py-2 bg-gray-800 border border-gray-700 rounded"
                  value={resolution} onChange={(e) => setResolution(e.target.value)}>
            <option>1280x720</option>
            <option>1920x1080</option>
            <option>2560x1440</option>
          </select>
        </label>
        <label className="grid gap-2">
          <span>Framerate (FPS)</span>
          <input type="number" className="px-3 py-2 bg-gray-800 border border-gray-700 rounded"
                 value={framerate} onChange={(e) => setFramerate(parseInt(e.target.value))} />
        </label>
        <div className="text-sm text-gray-400">Settings are applied in-memory only. Slump stores no credentials or sensitive data.</div>
      </div>
    </div>
  );
}
