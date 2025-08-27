import { contextBridge, ipcRenderer } from 'electron';
import * as path from 'path';

let native: any = null;
try {
  // .node copied to dist root alongside preload.js/main.js
  const modPath = path.join(__dirname, 'slump_native.node');
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  native = require(modPath);
} catch (e) {
  native = null;
}

contextBridge.exposeInMainWorld('slump', {
  startOAuth: () => ipcRenderer.invoke('oauth:start'),
  logout: () => ipcRenderer.invoke('oauth:logout'),
  authStatus: () => ipcRenderer.invoke('auth:status'),
  onOAuthSuccess: (cb: () => void) => {
    const listener = () => cb();
    ipcRenderer.on('oauth:success', listener);
    return () => ipcRenderer.removeListener('oauth:success', listener);
  },
  startStream: (bitrateKbps: number, width: number, height: number, fps: number) => {
    if (!native) throw new Error('Native module not loaded');
    return native.start_stream(bitrateKbps, width, height, fps);
  },
  stopStream: () => {
    if (!native) throw new Error('Native module not loaded');
    return native.stop_stream();
  },
  getStats: () => {
    if (!native) throw new Error('Native module not loaded');
    return native.get_stats();
  }
});

declare global {
  interface Window {
    slump: {
      startOAuth: () => Promise<boolean>;
      logout: () => Promise<boolean>;
      authStatus: () => Promise<boolean>;
      onOAuthSuccess: (cb: () => void) => () => void;
      startStream: (bitrateKbps: number, width: number, height: number, fps: number) => boolean;
      stopStream: () => boolean;
      getStats: () => { bitrate_kbps: number; latency_ms: number };
    };
  }
}
