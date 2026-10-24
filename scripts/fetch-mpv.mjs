// Downloads the latest 64-bit Windows build of mpv (shinchiro/mpv-winbuild-cmake) into resources/mpv,
// where Tauri bundles it into the installer and the app finds it at runtime.
//
//   node scripts/fetch-mpv.mjs            download if missing; warn (don't fail) if it can't
//   node scripts/fetch-mpv.mjs --require  fail the build if mpv isn't in place (used for releases)
//   node scripts/fetch-mpv.mjs --force    re-download even if mpv is already there
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync, chmodSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import sevenZip from '7zip-bin';

const REPO = 'shinchiro/mpv-winbuild-cmake';
// Plain x86_64 (not the -v3 build, which needs a CPU with AVX2). Excludes dev, ffmpeg and aarch64 builds.
const ASSET = /^mpv-x86_64-\d{8}-git-[0-9a-f]+\.7z$/;
// Files in the archive that the app doesn't need.
const SKIP = new Set(['installer', 'doc', 'updater.bat', 'mpv-register.bat', 'mpv-unregister.bat']);

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
/** Finds the newest build. Uses the GitHub API (which also gives a checksum), and falls back to the public release page when the API is rate limited. */
async function findAsset() {
  const api = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers });
  if (api.ok) {
    const release = await api.json();
    const found = release.assets?.find(a => ASSET.test(a.name));
    if (!found) throw new Error(`no matching Windows build in release ${release.tag_name}.`);
    return { name: found.name, url: found.browser_download_url, size: found.size, sha256: found.digest?.startsWith('sha256:') ? found.digest.slice(7) : null, tag: release.tag_name };
  }
  console.log(`mpv: the GitHub API answered ${api.status}, reading the release page instead…`);
  const latest = await fetch(`https://github.com/${REPO}/releases/latest`, { headers: { 'User-Agent': 'anigui-build' } });
  const tag = latest.url.split('/tag/')[1];
  if (!latest.ok || !tag) throw new Error(`could not find the latest release (${latest.status}).`);
  const page = await (await fetch(`https://github.com/${REPO}/releases/expanded_assets/${tag}`, { headers: { 'User-Agent': 'anigui-build' } })).text();
  const href = [...page.matchAll(/href="([^"]+\.7z)"/g)].map(m => m[1]).find(h => ASSET.test(decodeURIComponent(h.split('/').pop())));
  if (!href) throw new Error(`no matching Windows build in release ${tag}.`);
  return { name: decodeURIComponent(href.split('/').pop()), url: new URL(href, 'https://github.com').href, size: null, sha256: null, tag };
}

const dest = join(root, 'resources', 'mpv');
const args = new Set(process.argv.slice(2));

function fail(message) {
  if (args.has('--require')) { console.error(`\nmpv: ${message}\n`); process.exit(1); }
  console.warn(`mpv: ${message}\nmpv: continuing without it. The installer build needs it (npm run tauri build does this check).`);
  process.exit(0);
}

if (existsSync(join(dest, 'mpv.exe')) && !args.has('--force')) {
  console.log('mpv: already in resources/mpv (use --force to update it)');
  process.exit(0);
}

const headers = { 'User-Agent': 'anigui-build', Accept: 'application/vnd.github+json' };
if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;

let work;
try {
  console.log('mpv: looking up the latest release…');
  const asset = await findAsset();

  console.log(`mpv: downloading ${asset.name}${asset.size ? ` (${(asset.size / 1e6).toFixed(0)} MB)` : ''}…`);
  const file = await fetch(asset.url, { headers: { 'User-Agent': 'anigui-build' } });
  if (!file.ok) throw new Error(`download failed with ${file.status}.`);
  const bytes = Buffer.from(await file.arrayBuffer());
  const expectedSize = asset.size ?? Number(file.headers.get('content-length'));
  if (expectedSize && bytes.length !== expectedSize) throw new Error(`downloaded ${bytes.length} bytes, expected ${expectedSize}.`);
  if (bytes.length < 10e6) throw new Error('the download is far too small to be mpv.');
  if (asset.sha256 && createHash('sha256').update(bytes).digest('hex') !== asset.sha256) throw new Error('checksum mismatch, the download is corrupt.');

  work = mkdtempSync(join(tmpdir(), 'mpv-'));
  const archive = join(work, asset.name);
  writeFileSync(archive, bytes);
  try { chmodSync(sevenZip.path7za, 0o755); } catch { /* not needed on Windows */ }
  const unpack = spawnSync(sevenZip.path7za, ['x', '-y', `-o${join(work, 'out')}`, archive], { stdio: 'pipe' });
  if (unpack.status !== 0) throw new Error(`could not extract the archive:\n${unpack.stderr || unpack.stdout}`);

  const out = join(work, 'out');
  if (!existsSync(join(out, 'mpv.exe'))) throw new Error('mpv.exe is not in the archive; the build layout may have changed.');

  mkdirSync(dest, { recursive: true });
  for (const name of readdirSync(out)) {
    if (!SKIP.has(name.toLowerCase())) cpSync(join(out, name), join(dest, name), { recursive: true });
  }
  writeFileSync(join(dest, 'VERSION.txt'), `${asset.name}\nfrom ${asset.url}\nsource: https://github.com/${REPO}\n`);
  console.log(`mpv: ready in resources/mpv (${asset.tag})`);
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
} finally {
  if (work) rmSync(work, { recursive: true, force: true });
}
