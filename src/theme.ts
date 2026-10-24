export type Background = 'black' | 'charcoal' | 'midnight';
export type PosterSize = 'compact' | 'normal' | 'large';
export type Theme = { accent: string; background: Background; posterSize: PosterSize };

export const DEFAULT_THEME: Theme = { accent: '#ff6a00', background: 'black', posterSize: 'normal' };

export const ACCENT_PRESETS: { name: string; value: string }[] = [
  { name: 'Orange', value: '#ff6a00' },
  { name: 'Red', value: '#ef4444' },
  { name: 'Pink', value: '#ec4899' },
  { name: 'Purple', value: '#a855f7' },
  { name: 'Indigo', value: '#6366f1' },
  { name: 'Blue', value: '#3b82f6' },
  { name: 'Sky', value: '#22b8f0' },
  { name: 'Teal', value: '#14b8a6' },
  { name: 'Green', value: '#22c55e' },
  { name: 'Lime', value: '#a3e635' },
  { name: 'Yellow', value: '#facc15' },
  { name: 'White', value: '#e8e8ec' },
];

export const BACKGROUNDS: Record<Background, { label: string; vars: Record<string, string> }> = {
  black: { label: 'Black', vars: { '--bg': '#09090b', '--surface': '#131316', '--raised': '#1b1b20', '--hover': '#24242a', '--line': '#26262c' } },
  charcoal: { label: 'Charcoal', vars: { '--bg': '#131315', '--surface': '#1b1b1f', '--raised': '#242429', '--hover': '#2e2e34', '--line': '#323238' } },
  midnight: { label: 'Midnight', vars: { '--bg': '#0a0d15', '--surface': '#121723', '--raised': '#1a2132', '--hover': '#232b40', '--line': '#232c40' } },
};

export const POSTER_MIN: Record<PosterSize, { label: string; px: number }> = {
  compact: { label: 'Compact', px: 140 },
  normal: { label: 'Normal', px: 168 },
  large: { label: 'Large', px: 212 },
};

const KEY = 'anigui.theme';
const HEX = /^#[0-9a-f]{6}$/i;

/** Accepts "#f60", "f60", "#ff6600" or "ff6600" and returns "#ff6600", or null if it isn't a colour. */
export function normalizeHex(input: string): string | null {
  let v = input.trim().replace(/^#/, '');
  if (/^[0-9a-f]{3}$/i.test(v)) v = v.split('').map(c => c + c).join('');
  return /^[0-9a-f]{6}$/i.test(v) ? `#${v.toLowerCase()}` : null;
}

export function loadTheme(): Theme {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? 'null') as Partial<Theme> | null;
    return {
      accent: raw?.accent && HEX.test(raw.accent) ? raw.accent.toLowerCase() : DEFAULT_THEME.accent,
      background: raw?.background && raw.background in BACKGROUNDS ? raw.background : DEFAULT_THEME.background,
      posterSize: raw?.posterSize && raw.posterSize in POSTER_MIN ? raw.posterSize : DEFAULT_THEME.posterSize,
    };
  } catch { return DEFAULT_THEME; }
}

function channels(hex: string): [number, number, number] {
  return [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16)) as [number, number, number];
}

function luminance(hex: string): number {
  const [r, g, b] = channels(hex).map(v => { const c = v / 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Text colour for use on top of the accent: whichever of near-black or white is easier to read. */
export function inkFor(accent: string): string {
  const l = luminance(accent);
  const dark = (l + 0.05) / (luminance('#14100c') + 0.05);
  const light = (1.05) / (l + 0.05);
  return dark >= light ? '#14100c' : '#ffffff';
}

function mix(hex: string, towards: [number, number, number], amount: number): string {
  const out = channels(hex).map((v, i) => Math.round(v + (towards[i] - v) * amount));
  return `#${out.map(v => v.toString(16).padStart(2, '0')).join('')}`;
}

export function applyTheme(theme: Theme, persist = true) {
  const root = document.documentElement.style;
  root.setProperty('--accent', theme.accent);
  root.setProperty('--accent-ink', inkFor(theme.accent));
  root.setProperty('--accent-hover', mix(theme.accent, [255, 255, 255], 0.16));
  root.setProperty('--accent-soft', `${theme.accent}26`);
  for (const [name, value] of Object.entries(BACKGROUNDS[theme.background].vars)) root.setProperty(name, value);
  root.setProperty('--poster-min', `${POSTER_MIN[theme.posterSize].px}px`);
  if (persist) { try { localStorage.setItem(KEY, JSON.stringify(theme)); } catch { /* storage unavailable: the theme just won't persist */ } }
}
