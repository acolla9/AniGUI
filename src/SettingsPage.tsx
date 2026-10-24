import { useEffect, useState } from 'react';
import type { PlayerChoice, PlayersInfo, Settings } from './types';
import { QUALITIES } from './types';
import { ACCENT_PRESETS, BACKGROUNDS, DEFAULT_THEME, POSTER_MIN, normalizeHex, type Background, type PosterSize, type Theme } from './theme';

type Props = {
  settings: Settings;
  players: PlayersInfo | null;
  historyCount: number;
  onSave: (next: Settings) => Promise<void>;
  onClearHistory: () => Promise<void>;
  theme: Theme;
  onTheme: (next: Theme) => void;
};

const PLAYER_LABEL: Record<PlayerChoice, string> = { mpv: 'mpv', vlc: 'VLC' };

export function SettingsPage({ settings, players, historyCount, onSave, onClearHistory, theme, onTheme }: Props) {
  const [error, setError] = useState('');
  const [mpvPath, setMpvPath] = useState(settings.mpvPath);
  const [vlcPath, setVlcPath] = useState(settings.vlcPath);
  const [confirmClear, setConfirmClear] = useState(false);
  const [hex, setHex] = useState(theme.accent);
  const [hexInvalid, setHexInvalid] = useState(false);
  useEffect(() => { setHex(theme.accent); setHexInvalid(false); }, [theme.accent]);

  // Keep the text boxes in sync if settings are reloaded or cleaned up by the backend.
  useEffect(() => { setMpvPath(settings.mpvPath); setVlcPath(settings.vlcPath); }, [settings.mpvPath, settings.vlcPath]);

  async function update(patch: Partial<Settings>) {
    setError('');
    try { await onSave({ ...settings, ...patch }); }
    catch (e) { setError(typeof e === 'string' ? e : 'Could not save settings.'); }
  }

  const found = (id: PlayerChoice) => (id === 'mpv' ? players?.mpv : players?.vlc) ?? null;
  const commitOnEnter = (e: React.KeyboardEvent<HTMLInputElement>) => e.key === 'Enter' && e.currentTarget.blur();

  function typeHex(value: string) {
    setHex(value);
    const parsed = normalizeHex(value);
    setHexInvalid(!parsed && value.trim() !== '');
    if (parsed) onTheme({ ...theme, accent: parsed });
  }
  const isDefault = theme.accent === DEFAULT_THEME.accent && theme.background === DEFAULT_THEME.background && theme.posterSize === DEFAULT_THEME.posterSize;

  return <section className="page settings">
    <h1 className="page-title">Settings</h1>
    <p className="page-sub">Changes are saved as you make them.</p>
    {error && <div className="notice error">{error}</div>}

    <h2 className="section-title">Appearance</h2>
    <div className="panel">
      <div className="row stacked top">
        <label>Accent color</label>
        <div className="swatches" role="group" aria-label="Accent color presets">
          {ACCENT_PRESETS.map(c => <button key={c.value} className="swatch" style={{ '--c': c.value } as React.CSSProperties} title={c.name} aria-label={c.name} aria-pressed={theme.accent === c.value} onClick={() => onTheme({ ...theme, accent: c.value })} />)}
        </div>
        <div className="custom-color">
          <input type="color" value={theme.accent} aria-label="Pick any color" onChange={e => onTheme({ ...theme, accent: e.target.value })} />
          <input type="text" value={hex} aria-label="Hex color" aria-invalid={hexInvalid} spellCheck={false} maxLength={7} placeholder="#ff6a00" onChange={e => typeHex(e.target.value)} onBlur={() => { setHex(theme.accent); setHexInvalid(false); }} />
          <span className="hint" style={{ margin: 0 }}>{hexInvalid ? 'Use a hex color like #ff6a00.' : 'Pick any color, or type a hex code.'}</span>
        </div>
        <div className="preview" aria-hidden="true"><span className="btn primary">Play episode 1</span><span className="ep now" style={{ width: 44 }}>5</span></div>
      </div>
      <div className="row">
        <div className="row-text"><strong>Background</strong><small>The base tone of every screen.</small></div>
        <div className="segmented" role="group" aria-label="Background">
          {(Object.keys(BACKGROUNDS) as Background[]).map(id => <button key={id} aria-pressed={theme.background === id} onClick={() => onTheme({ ...theme, background: id })}>{BACKGROUNDS[id].label}</button>)}
        </div>
      </div>
      <div className="row">
        <div className="row-text"><strong>Poster size</strong><small>How large covers are in grids.</small></div>
        <div className="segmented" role="group" aria-label="Poster size">
          {(Object.keys(POSTER_MIN) as PosterSize[]).map(id => <button key={id} aria-pressed={theme.posterSize === id} onClick={() => onTheme({ ...theme, posterSize: id })}>{POSTER_MIN[id].label}</button>)}
        </div>
      </div>
      <div className="row">
        <div className="row-text"><strong>Reset appearance</strong><small>Back to orange on black.</small></div>
        <button className="btn" disabled={isDefault} onClick={() => onTheme(DEFAULT_THEME)}>Reset</button>
      </div>
    </div>

    <h2 className="section-title">Player</h2>
    <p className="hint">Episodes open in an external player. mpv is the default and plays the next episode on its own; VLC works too.</p>
    <div className="player-options" role="radiogroup" aria-label="Video player">
      {(['mpv', 'vlc'] as const).map(id => {
        const path = found(id);
        const on = settings.player === id;
        return <button key={id} role="radio" aria-checked={on} className={`player-option ${on ? 'on' : ''}`} onClick={() => update({ player: id })}>
          <strong>{PLAYER_LABEL[id]}</strong>
          <small className={path ? 'ok' : 'missing'}>{players ? (path ?? 'Not found') : 'Checking…'}</small>
        </button>;
      })}
    </div>
    {players && !players.selectedFound && <div className="notice warn">{players.message}</div>}
    <div className="panel">
      <div className="row stacked">
        <label htmlFor="mpv-path">mpv location <span>(optional)</span></label>
        <input id="mpv-path" value={mpvPath} placeholder="Detected automatically, or C:\Tools\mpv\mpv.exe" onChange={e => setMpvPath(e.target.value)} onBlur={() => mpvPath !== settings.mpvPath && update({ mpvPath })} onKeyDown={commitOnEnter} />
      </div>
      <div className="row stacked">
        <label htmlFor="vlc-path">VLC location <span>(optional)</span></label>
        <input id="vlc-path" value={vlcPath} placeholder="Detected automatically, or C:\Program Files\VideoLAN\VLC\vlc.exe" onChange={e => setVlcPath(e.target.value)} onBlur={() => vlcPath !== settings.vlcPath && update({ vlcPath })} onKeyDown={commitOnEnter} />
      </div>
    </div>

    <h2 className="section-title">Playback</h2>
    <div className="panel">
      <div className="row">
        <div className="row-text"><strong>Audio</strong><small>Subtitled or dubbed by default. You can switch on each title.</small></div>
        <div className="segmented" role="group" aria-label="Default audio">
          <button aria-pressed={!settings.defaultDub} onClick={() => update({ defaultDub: false })}>Sub</button>
          <button aria-pressed={settings.defaultDub} onClick={() => update({ defaultDub: true })}>Dub</button>
        </div>
      </div>
      <div className="row">
        <div className="row-text"><strong>Quality</strong><small>If an episode doesn’t offer it, the best available is used.</small></div>
        <select value={settings.defaultQuality} onChange={e => update({ defaultQuality: e.target.value })} aria-label="Default quality">{QUALITIES.map(q => <option key={q} value={q}>{q}</option>)}</select>
      </div>
      <div className="row">
        <div className="row-text"><strong>Play next episode automatically</strong><small>Starts the next episode when one plays to the end. Works with mpv only; with VLC use the Next button.</small></div>
        <label className="switch"><input type="checkbox" checked={settings.autoPlayNext} onChange={e => update({ autoPlayNext: e.target.checked })} /><span className="switch-track" /><span className="visually-hidden">Play next episode automatically</span></label>
      </div>
    </div>

    <h2 className="section-title">Watch history</h2>
    <div className="panel">
      <div className="row">
        <div className="row-text"><strong>{historyCount} {historyCount === 1 ? 'title' : 'titles'} saved</strong><small>Stored on this computer only.</small></div>
        {confirmClear
          ? <div className="btn-group"><button className="btn danger" onClick={async () => { await onClearHistory(); setConfirmClear(false); }}>Clear history</button><button className="btn" onClick={() => setConfirmClear(false)}>Cancel</button></div>
          : <button className="btn" disabled={historyCount === 0} onClick={() => setConfirmClear(true)}>Clear history</button>}
      </div>
    </div>
  </section>;
}
