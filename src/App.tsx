import { FormEvent, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { House, Search, Clock3, Settings as SettingsIcon, Play, SkipBack, SkipForward, RotateCcw, TriangleAlert } from 'lucide-react';
import { SettingsPage } from './SettingsPage';
import { ContinueCard, HistoryCard } from './History';
import { Detail } from './Detail';
import { applyTheme, loadTheme, type Theme } from './theme';
import { Poster, PosterCard, SearchBox, Spinner } from './ui';
import {
  AnimeResult, DEFAULT_SETTINGS, Episode, EpisodesResponse, HistoryEntry, NowPlaying, PlaybackEnded, PlaybackStarted,
  PlayersInfo, SearchResponse, Settings, Status,
} from './types';

type Page = 'home' | 'search' | 'history' | 'settings';

const NAV: { page: Page; label: string; icon: typeof House }[] = [
  { page: 'home', label: 'Home', icon: House },
  { page: 'search', label: 'Search', icon: Search },
  { page: 'history', label: 'Watch history', icon: Clock3 },
];

export default function App() {
  const [page, setPage] = useState<Page>('home');
  const [query, setQuery] = useState('');
  const [searchedFor, setSearchedFor] = useState('');
  const [status, setStatus] = useState<Status | null>(null);
  const [searchMessage, setSearchMessage] = useState('');
  const [searchFailed, setSearchFailed] = useState(false);
  const [results, setResults] = useState<AnimeResult[]>([]);
  const [busy, setBusy] = useState(false);
  const [selectedAnime, setSelectedAnime] = useState<AnimeResult | null>(null);
  const [episodes, setEpisodes] = useState<Episode[]>([]);
  const [episodesMessage, setEpisodesMessage] = useState('');
  const [episodesBusy, setEpisodesBusy] = useState(false);

  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [players, setPlayers] = useState<PlayersInfo | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [dub, setDub] = useState(false);
  const [quality, setQuality] = useState('best');
  const [resolvingId, setResolvingId] = useState<string | null>(null);
  const [playingId, setPlayingId] = useState<string | null>(null);
  const [nowPlaying, setNowPlaying] = useState<NowPlaying | null>(null);
  const [playback, setPlayback] = useState<{ text: string; error: boolean } | null>(null);

  const [theme, setTheme] = useState<Theme>(loadTheme);
  function changeTheme(next: Theme) { setTheme(next); applyTheme(next); }

  const searchInput = useRef<HTMLInputElement>(null);
  // Only the most recent episode request may update the screen.
  const episodesRequest = useRef(0);
  // The playback-ended listener is created once, so it reads the latest state through this ref.
  const busyRef = useRef(false);
  const latest = useRef<{ settings: Settings; nowPlaying: NowPlaying | null; startPlayback: (a: AnimeResult, e: Episode, list: Episode[]) => Promise<void> }>(
    { settings, nowPlaying, startPlayback },
  );

  function refreshHistory() {
    invoke<HistoryEntry[]>('get_history').then(setHistory).catch(() => undefined);
  }
  function refreshPlayers() {
    return invoke<PlayersInfo>('detect_players').then(setPlayers).catch(() => setPlayers(null));
  }

  useEffect(() => {
    invoke<Status>('get_app_status').then(setStatus).catch(() => setStatus({ appName: 'AniGUI', version: '0.1.0', engineConnected: false, platform: 'browser', message: 'Preview mode: run the app through Tauri to connect the Rust backend.' }));
    invoke<Settings>('get_settings').then(s => { setSettings(s); setDub(s.defaultDub); setQuality(s.defaultQuality); }).catch(() => undefined);
    refreshPlayers();
    refreshHistory();

    // Fired by the backend when the external player closes (or fails to play the stream).
    const unlisten = listen<PlaybackEnded>('playback-ended', ({ payload }) => {
      const current = latest.current;
      setPlayingId(id => (id === payload.episodeId ? null : id));
      refreshHistory();
      const np = current.nowPlaying;
      if (!np || np.episode.id !== payload.episodeId) return; // the user already moved on to another episode
      if (payload.failed) {
        setNowPlaying({ ...np, state: 'closed' });
        setPlayback({ text: payload.message, error: true });
        return;
      }
      const index = np.episodes.findIndex(e => e.id === np.episode.id);
      const next = index >= 0 ? np.episodes[index + 1] : undefined;
      if (payload.completed && current.settings.autoPlayNext && next) {
        setPlayback({ text: `Finished episode ${np.episode.number}. Starting episode ${next.number}…`, error: false });
        void current.startPlayback(np.anime, next, np.episodes);
        return;
      }
      setNowPlaying({ ...np, state: payload.completed ? 'finished' : 'closed' });
      setPlayback({ text: payload.message, error: false });
    }).catch(() => undefined); // not running inside Tauri (browser preview)
    return () => { unlisten.then(fn => fn?.()); };
  }, []);

  // Ctrl+K (or Cmd+K) jumps to search.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        go('search');
        requestAnimationFrame(() => searchInput.current?.focus());
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  function go(next: Page) {
    setPage(next);
    setSelectedAnime(null);
  }

  async function startPlayback(anime: AnimeResult, episode: Episode, list: Episode[]) {
    if (busyRef.current) return;
    busyRef.current = true;
    setResolvingId(episode.id);
    setNowPlaying({ anime, episode, episodes: list, state: 'starting' });
    setPlayback({ text: `Fetching stream for episode ${episode.number}…`, error: false });
    try {
      const started = await invoke<PlaybackStarted>('play_episode', { animeId: anime.id, animeTitle: anime.title, animePoster: anime.poster ?? null, episode, dub, quality });
      setPlayingId(started.episodeId);
      setNowPlaying({ anime, episode, episodes: list, state: 'playing' });
      setPlayback({ text: started.message, error: false });
      refreshHistory();
    } catch (error) {
      setNowPlaying({ anime, episode, episodes: list, state: 'closed' });
      setPlayback({ text: typeof error === 'string' ? error : 'Could not start playback. Check your connection and try again.', error: true });
    } finally {
      busyRef.current = false;
      setResolvingId(null);
    }
  }
  latest.current = { settings, nowPlaying, startPlayback };

  async function search(event: FormEvent) {
    event.preventDefault();
    const text = query.trim();
    if (!text) return;
    setPage('search');
    setBusy(true);
    setSearchFailed(false);
    try {
      const result = await invoke<SearchResponse>('search_anime', { query: text });
      setResults(result.results);
      setSearchedFor(result.query);
      setSearchMessage(result.message);
      setSelectedAnime(null);
      setEpisodes([]);
      setEpisodesMessage('');
      setPlayback(null);
    } catch (error) {
      setResults([]);
      setSearchedFor(text);
      setSearchFailed(true);
      setSearchMessage(typeof error === 'string' ? error : 'Search failed. Check your connection and try again.');
    } finally { setBusy(false); }
  }

  /** Loads the episode list for an anime. Returns it, or null if loading failed. */
  async function loadEpisodes(anime: AnimeResult): Promise<Episode[] | null> {
    const request = ++episodesRequest.current;
    setSelectedAnime(anime);
    setEpisodes([]);
    setPlayback(null);
    setEpisodesBusy(true);
    setEpisodesMessage('');
    try {
      const response = await invoke<EpisodesResponse>('load_episodes', { animeId: anime.id });
      if (request !== episodesRequest.current) return null;
      setEpisodes(response.episodes);
      setEpisodesMessage(response.episodes.length ? response.message : 'This title has no episodes yet.');
      return response.episodes;
    } catch (error) {
      if (request === episodesRequest.current) setEpisodesMessage(typeof error === 'string' ? error : 'Could not load episodes. Check your connection and try again.');
      return null;
    } finally { if (request === episodesRequest.current) setEpisodesBusy(false); }
  }

  /** Continue watching: play the next episode if the last one finished, otherwise resume the last one. */
  async function continueFromHistory(entry: HistoryEntry) {
    if (busyRef.current) return;
    const anime: AnimeResult = { id: entry.animeId, title: entry.animeTitle, poster: entry.poster };
    const list = await loadEpisodes(anime);
    if (!list) return;
    const target = resumeTarget(list, entry);
    if (!target) {
      const index = indexOfEntry(list, entry);
      setPlayback({ text: index < 0 ? `Couldn’t find episode ${entry.episodeNumber} in the list. Pick an episode below.` : `You’re all caught up on ${entry.animeTitle}. Pick an episode below to rewatch.`, error: false });
      return;
    }
    await startPlayback(anime, target, list);
  }

  async function removeFromHistory(entry: HistoryEntry) {
    try { setHistory(await invoke<HistoryEntry[]>('remove_history', { animeId: entry.animeId })); } catch { /* keep the list as it is */ }
  }

  async function clearHistory() {
    setHistory(await invoke<HistoryEntry[]>('clear_history'));
  }

  async function saveSettings(next: Settings) {
    const saved = await invoke<Settings>('save_settings', { settings: next });
    // A changed default also becomes the current choice; leave the session choice alone otherwise.
    if (saved.defaultDub !== settings.defaultDub) setDub(saved.defaultDub);
    if (saved.defaultQuality !== settings.defaultQuality) setQuality(saved.defaultQuality);
    setSettings(saved);
    await refreshPlayers();
  }

  const busyPlaying = resolvingId !== null;
  const playerMissing = players !== null && !players.selectedFound;

  // Main button on the detail page: continue from history if this title is in it, otherwise episode 1.
  let primary: { label: string; episode: Episode } | null = null;
  if (selectedAnime && episodes.length > 0) {
    const entry = history.find(h => h.animeId === selectedAnime.id);
    const target = entry ? resumeTarget(episodes, entry) : undefined;
    primary = target && entry
      ? { label: entry.completed ? `Play episode ${target.number}` : `Resume episode ${target.number}`, episode: target }
      : { label: `Play episode ${episodes[0].number}`, episode: episodes[0] };
  }

  // "Now playing" bar
  const npIndex = nowPlaying ? nowPlaying.episodes.findIndex(e => e.id === nowPlaying.episode.id) : -1;
  const prevEpisode = nowPlaying && npIndex > 0 ? nowPlaying.episodes[npIndex - 1] : undefined;
  const nextEpisode = nowPlaying && npIndex >= 0 ? nowPlaying.episodes[npIndex + 1] : undefined;
  const npText = !nowPlaying ? '' : playback?.error ? playback.text : {
    starting: `Starting episode ${nowPlaying.episode.number}`,
    playing: `Playing episode ${nowPlaying.episode.number}`,
    finished: `Finished episode ${nowPlaying.episode.number}`,
    closed: `Player closed in episode ${nowPlaying.episode.number}`,
  }[nowPlaying.state];

  const latestEntry = history[0];

  const home = <>
    <section className={`hero ${latestEntry ? '' : 'plain'}`}>
      {latestEntry && <Poster src={latestEntry.poster} className="hero-art" />}
      <div className="hero-text">
        {latestEntry
          ? <><h1 className="hero-kicker">Continue watching</h1><p className="hero-title">{latestEntry.animeTitle}</p></>
          : <><h1 className="hero-title">What do you want to watch?</h1><p className="hero-sub">Episodes you play show up here, so you can pick up later.</p>
            <SearchBox value={query} onChange={setQuery} onSubmit={search} busy={busy} /></>}
      </div>
    </section>
    {latestEntry && <div className="page home-body">
      <ContinueCard entry={latestEntry} busy={busyPlaying} onContinue={continueFromHistory} />
      {history.length > 1 && <>
        <div className="section-head"><h2 className="section-title">Recently watched</h2>{history.length > 7 && <button className="link" onClick={() => go('history')}>View all</button>}</div>
        <div className="grid">{history.slice(1, 7).map(entry => <HistoryCard key={entry.animeId} entry={entry} busy={busyPlaying} onContinue={continueFromHistory} onRemove={removeFromHistory} />)}</div>
      </>}
    </div>}
  </>;

  const searchPage = <section className="page">
    <h1 className="page-title">Search</h1>
    <SearchBox value={query} onChange={setQuery} onSubmit={search} busy={busy} inputRef={searchInput} autoFocus />
    {searchedFor && <p className={`page-sub ${searchFailed ? 'error' : ''}`} role="status">{searchMessage}</p>}
    {results.length > 0 && <div className="grid">{results.map(anime => <PosterCard key={anime.id} poster={anime.poster} title={anime.title} onClick={() => loadEpisodes(anime)} />)}</div>}
  </section>;

  const historyPage = <section className="page">
    <h1 className="page-title">Watch history</h1>
    <p className="page-sub">{history.length ? 'Pick up where you left off.' : 'Nothing here yet. Episodes you play will appear here.'}</p>
    <div className="grid">{history.map(entry => <HistoryCard key={entry.animeId} entry={entry} busy={busyPlaying} onContinue={continueFromHistory} onRemove={removeFromHistory} />)}</div>
  </section>;

  let content;
  if (selectedAnime) {
    content = <Detail anime={selectedAnime} episodes={episodes} loading={episodesBusy} message={episodesMessage} dub={dub} quality={quality} players={players}
      playingId={playingId} resolvingId={resolvingId} busy={busyPlaying} playback={playback} primary={primary}
      onDub={setDub} onQuality={setQuality} onPlay={ep => startPlayback(selectedAnime, ep, episodes)} onBack={() => setSelectedAnime(null)} onOpenSettings={() => go('settings')} />;
  } else if (page === 'settings') {
    content = <SettingsPage settings={settings} players={players} historyCount={history.length} onSave={saveSettings} onClearHistory={clearHistory} theme={theme} onTheme={changeTheme} />;
  } else if (page === 'history') content = historyPage;
  else if (page === 'search') content = searchPage;
  else content = home;

  return <div className="app">
    <nav className="rail" aria-label="Main">
      <div className="logo" aria-hidden="true"><Play size={16} fill="currentColor" /></div>
      <div className="rail-group">{NAV.map(({ page: p, label, icon: Icon }) =>
        <button key={p} className={`rail-btn ${page === p ? 'active' : ''}`} title={p === 'search' ? 'Search (Ctrl+K)' : label} aria-label={label} aria-current={page === p ? 'page' : undefined} onClick={() => go(p)}><Icon size={20} /></button>)}
      </div>
      <button className={`rail-btn ${page === 'settings' ? 'active' : ''}`} title="Settings" aria-label="Settings" aria-current={page === 'settings' ? 'page' : undefined} onClick={() => go('settings')}><SettingsIcon size={20} /></button>
    </nav>

    <div className="stage">
      <div className="chips">
        {status && !status.engineConnected && <span className="chip" title={status.message}>Preview mode</span>}
        {playerMissing && <button className="chip warn" onClick={() => go('settings')}><TriangleAlert size={14} /> Player not found</button>}
      </div>
      <main className={`main ${nowPlaying ? 'with-bar' : ''}`}>{content}</main>

      {nowPlaying && <div className="np" aria-live="polite">
        <Poster src={nowPlaying.anime.poster} title={nowPlaying.anime.title} className="np-poster" />
        <div className="np-text"><strong title={nowPlaying.anime.title}>{nowPlaying.anime.title}</strong><span className={playback?.error ? 'error' : ''} title={npText}>{npText}</span></div>
        {nowPlaying.state === 'starting' && <Spinner />}
        <div className="np-actions">
          <button className="icon-btn" title={prevEpisode ? `Episode ${prevEpisode.number}` : 'No previous episode'} aria-label="Previous episode" disabled={!prevEpisode || busyPlaying} onClick={() => prevEpisode && startPlayback(nowPlaying.anime, prevEpisode, nowPlaying.episodes)}><SkipBack size={17} /></button>
          <button className="icon-btn" title="Play this episode again" aria-label="Replay episode" disabled={busyPlaying} onClick={() => startPlayback(nowPlaying.anime, nowPlaying.episode, nowPlaying.episodes)}><RotateCcw size={16} /></button>
          <button className="icon-btn" title={nextEpisode ? `Episode ${nextEpisode.number}` : 'No next episode'} aria-label="Next episode" disabled={!nextEpisode || busyPlaying} onClick={() => nextEpisode && startPlayback(nowPlaying.anime, nextEpisode, nowPlaying.episodes)}><SkipForward size={17} /></button>
        </div>
      </div>}
    </div>
  </div>;
}

function indexOfEntry(list: Episode[], entry: HistoryEntry): number {
  return list.findIndex(e => e.id === entry.episodeId || e.number === entry.episodeNumber);
}

/** The episode a history entry leads to: the next one if it was finished, the same one otherwise. */
function resumeTarget(list: Episode[], entry: HistoryEntry): Episode | undefined {
  const index = indexOfEntry(list, entry);
  if (index < 0) return undefined;
  return entry.completed ? list[index + 1] : list[index];
}
