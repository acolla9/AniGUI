import { ChevronLeft, Play, TriangleAlert } from 'lucide-react';
import type { AnimeResult, Episode, PlayersInfo } from './types';
import { QUALITIES } from './types';
import { Poster, Spinner } from './ui';

type Props = {
  anime: AnimeResult;
  episodes: Episode[];
  loading: boolean;
  message: string;
  dub: boolean;
  quality: string;
  players: PlayersInfo | null;
  playingId: string | null;
  resolvingId: string | null;
  busy: boolean;
  playback: { text: string; error: boolean } | null;
  /** The main button: play episode 1, or pick up from history. */
  primary: { label: string; episode: Episode } | null;
  onDub: (dub: boolean) => void;
  onQuality: (quality: string) => void;
  onPlay: (episode: Episode) => void;
  onBack: () => void;
  onOpenSettings: () => void;
};

export function Detail(p: Props) {
  const playerMissing = p.players && !p.players.selectedFound;
  const count = p.episodes.length;
  return <section className="detail">
    <div className="detail-head">
      <Poster src={p.anime.poster} className="backdrop" />
      <button className="back" onClick={p.onBack}><ChevronLeft size={18} /> Back</button>
      <div className="detail-main">
        <Poster src={p.anime.poster} className="detail-poster" />
        <div className="detail-info">
          <h1>{p.anime.title}</h1>
          <p className="detail-meta">{p.loading ? 'Loading episodes…' : count ? `${count} ${count === 1 ? 'episode' : 'episodes'}` : ''}</p>
          {count > 0 && <div className="controls">
            {p.primary && <button className="btn primary" disabled={p.busy} onClick={() => p.onPlay(p.primary!.episode)}><Play size={15} fill="currentColor" /> {p.primary.label}</button>}
            <div className="segmented" role="group" aria-label="Audio">
              <button aria-pressed={!p.dub} onClick={() => p.onDub(false)}>Sub</button>
              <button aria-pressed={p.dub} onClick={() => p.onDub(true)}>Dub</button>
            </div>
            <label className="field-inline">Quality
              <select value={p.quality} onChange={e => p.onQuality(e.target.value)}>{QUALITIES.map(q => <option key={q} value={q}>{q}</option>)}</select>
            </label>
          </div>}
        </div>
      </div>
    </div>

    <div className="detail-body">
      {playerMissing && <div className="notice warn"><TriangleAlert size={16} /><span>{p.players!.message}</span><button className="link" onClick={p.onOpenSettings}>Open settings</button></div>}
      {p.playback && <div className={`notice ${p.playback.error ? 'error' : ''}`} role="status">{p.playback.text}</div>}
      {!p.loading && count === 0 && p.message && <div className="notice error" role="status">{p.message}</div>}
      {p.loading && <div className="loading-row"><Spinner /> Fetching episode list</div>}
      {count > 0 && <>
        <h2 className="section-title">Episodes</h2>
        <div className="episodes">
          {p.episodes.map(ep => {
            const now = p.playingId === ep.id;
            const fetching = p.resolvingId === ep.id;
            return <button key={ep.id} className={`ep ${now ? 'now' : ''}`} disabled={p.busy} onClick={() => p.onPlay(ep)} title={now ? `Episode ${ep.number}, playing` : `Play episode ${ep.number}`} aria-label={`Episode ${ep.number}${now ? ', playing' : ''}`}>
              {fetching ? <Spinner /> : ep.number}
            </button>;
          })}
        </div>
      </>}
    </div>
  </section>;
}
