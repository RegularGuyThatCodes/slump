import { app, BrowserWindow, ipcMain, shell, net } from 'electron';
import * as path from 'path';
import * as http from 'http';
import { URL } from 'url';
import 'dotenv/config';
import { AppState, OAuthTokens } from './types';

let mainWindow: BrowserWindow | null = null;
let oauthServer: http.Server | null = null;
let appState: AppState = { isAuthenticated: false };

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  });

  // Always load bundled HTML from dist
  mainWindow.loadFile(path.join(__dirname, 'index.html'));

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

app.whenReady().then(() => {
  createWindow();

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', function () {
  if (process.platform !== 'darwin') app.quit();
});

ipcMain.handle('oauth:start', async () => {
  const clientId = process.env.META_CLIENT_ID || '';
  const clientSecret = process.env.META_CLIENT_SECRET || '';
  const authUrlBase = process.env.META_OAUTH_AUTH_URL || '';
  const tokenUrl = process.env.META_OAUTH_TOKEN_URL || '';
  const redirectPort = Number(process.env.META_REDIRECT_PORT || '53123');
  const scopes = (process.env.OAUTH_SCOPES || 'public_profile').split(',').join('%20');

  if (!clientId || !clientSecret || !authUrlBase || !tokenUrl) {
    throw new Error('Missing OAuth configuration in .env');
  }

  await stopOAuthServer();

  const redirectUri = `http://localhost:${redirectPort}/callback`;

  const state = Math.random().toString(36).slice(2);
  const authUrl = `${authUrlBase}?client_id=${encodeURIComponent(clientId)}&redirect_uri=${encodeURIComponent(redirectUri)}&response_type=code&scope=${scopes}&state=${state}`;

  await startOAuthServer(redirectPort, async (url) => {
    const u = new URL(url, `http://localhost:${redirectPort}`);
    if (u.pathname !== '/callback') return;
    const code = u.searchParams.get('code');
    const returnedState = u.searchParams.get('state');
    if (!code || returnedState !== state) return;

    const formData = new URLSearchParams();
    formData.append('client_id', clientId);
    formData.append('client_secret', clientSecret);
    formData.append('redirect_uri', redirectUri);
    formData.append('code', code);

    const request = net.request({ method: 'POST', url: tokenUrl, headers: { 'Content-Type': 'application/x-www-form-urlencoded' } });
    const tokenPromise: Promise<OAuthTokens> = new Promise((resolve, reject) => {
      let body = '';
      request.on('response', (response) => {
        response.on('data', (chunk) => (body += chunk.toString()));
        response.on('end', () => {
          try {
            const json = JSON.parse(body);
            resolve(json);
          } catch (e) {
            reject(e);
          }
        });
      });
      request.on('error', reject);
    });
    request.write(formData.toString());
    request.end();

    const tokens = await tokenPromise;
    appState = { isAuthenticated: true, tokens };
    await stopOAuthServer();
    if (mainWindow) mainWindow.webContents.send('oauth:success');
  });

  await shell.openExternal(authUrl);
  return true;
});

ipcMain.handle('oauth:logout', async () => {
  appState = { isAuthenticated: false };
  return true;
});

ipcMain.handle('auth:status', async () => {
  return appState.isAuthenticated;
});

async function startOAuthServer(port: number, onUrl: (url: string) => void) {
  await stopOAuthServer();
  oauthServer = http.createServer((req, res) => {
    if (!req.url) return;
    onUrl(req.url);
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end('<html><body><script>window.close()</script>Authenticated. You can close this window.</body></html>');
  });
  await new Promise<void>((resolve) => oauthServer!.listen(port, resolve));
}

async function stopOAuthServer() {
  if (!oauthServer) return;
  await new Promise<void>((resolve) => oauthServer!.close(() => resolve()));
  oauthServer = null;
}
