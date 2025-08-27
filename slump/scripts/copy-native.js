const fs = require('fs');
const path = require('path');

function findNode(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      const r = findNode(p);
      if (r) return r;
    } else if (e.isFile() && e.name.endsWith('.node')) {
      return p;
    }
  }
  return null;
}

function main() {
  const root = path.resolve(__dirname, '..');
  const nativeTarget = path.join(root, 'native', 'target');
  if (!fs.existsSync(nativeTarget)) {
    console.log('No native target directory; skipping copy');
    return;
  }
  const found = findNode(nativeTarget);
  if (!found) {
    console.log('No .node found in native/target; skipping copy');
    return;
  }
  const dist = path.join(root, 'dist');
  if (!fs.existsSync(dist)) fs.mkdirSync(dist, { recursive: true });
  const out = path.join(dist, 'slump_native.node');
  fs.copyFileSync(found, out);
  console.log(`Copied native module: ${found} -> ${out}`);
}

main();
