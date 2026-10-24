export type Status = { appName: string; version: string; engineConnected: boolean; platform: string; message: string };
export type AnimeResult = { id: string; title: string; poster?: string | null };
export type SearchResponse = { query: string; results: AnimeResult[]; engineConnected: boolean; message: string };
export type Episode = { id: string; number: string };
export type EpisodesResponse = { animeId: string; episodes: Episode[]; message: string };

export type PlayerChoice = 'mpv' | 'vlc';
export type Settings = {
  player: PlayerChoice;
  mpvPath: string;
  vlcPath: string;
  defaultDub: boolean;
  defaultQuality: string;
  autoPlayNext: boolean;
};
export type PlayersInfo = { selected: PlayerChoice; mpv: string | null; vlc: string | null; selectedFound: boolean; message: string };

export type HistoryEntry = {
  animeId: string;
  animeTitle: string;
  poster?: string | null;
  episodeId: string;
  episodeNumber: string;
  completed: boolean;
  updatedAt: number;
};

export type PlaybackStarted = { episodeId: string; player: string; quality: string; hasSubtitles: boolean; hasSeparateAudio: boolean; message: string };
export type PlaybackEnded = { animeId: string; episodeId: string; exitCode: number | null; failed: boolean; completed: boolean; message: string };

export type NowPlaying = {
  anime: AnimeResult;
  episode: Episode;
  /** Snapshot of the episode list, so Next/Previous work even after you search for something else. */
  episodes: Episode[];
  state: 'starting' | 'playing' | 'finished' | 'closed';
};

export const QUALITIES = ['best', '1080p', '720p', '480p', '360p', 'worst'];

export const DEFAULT_SETTINGS: Settings = {
  player: 'mpv',
  mpvPath: '',
  vlcPath: '',
  defaultDub: false,
  defaultQuality: 'best',
  autoPlayNext: true,
};