import { Play, X } from 'lucide-react';
import type { HistoryEntry } from './types';
import { Poster, PosterCard } from './ui';

export function formatWhen(unixSeconds: number): string {
  if (!unixSeconds) return '';
  const date = new Date(unixSeconds * 1000);
  const days = Math.round((startOfDay(new Date()) - startOfDay(date)) / 86_400_000);
  if (days <= 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return date.toLocaleDateString(undefined, { weekday: 'long' });
  return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** What the entry will do when clicked: a finished episode moves on, an unfinished one resumes. */
export function describeEntry(entry: HistoryEntry) {
  return {
    status: entry.completed ? `Finished episode ${entry.episodeNumber}` : `Stopped in episode ${entry.episodeNumber}`,
    action: entry.completed ? 'Play next episode' : `Resume episode ${entry.episodeNumber}`,
  };
}

type Props = {
  entry: HistoryEntry;
  busy: boolean;
  onContinue: (entry: HistoryEntry) => void;
  onRemove: (entry: HistoryEntry) => void;
};

/** Poster card for the history page and the "Recently watched" row. */
export function HistoryCard({ entry, busy, onContinue, onRemove }: Props) {
  const { status } = describeEntry(entry);
  const when = formatWhen(entry.updatedAt);
  return <PosterCard
    poster={entry.poster}
    title={entry.animeTitle}
    subtitle={when ? `${status}, ${when.toLowerCase()}` : status}
    disabled={busy}
    onClick={() => onContinue(entry)}
    extra={<button className="card-remove" title="Remove from history" aria-label={`Remove ${entry.animeTitle} from history`} onClick={() => onRemove(entry)}><X size={14} /></button>}
  />;
}

/** The wide card under the home banner. */
export function ContinueCard({ entry, busy, onContinue }: Omit<Props, 'onRemove'>) {
  const { status, action } = describeEntry(entry);
  const when = formatWhen(entry.updatedAt);
  return <button className="continue" disabled={busy} onClick={() => onContinue(entry)}>
    <span className="continue-media">
      <Poster src={entry.poster} title={entry.animeTitle} className="continue-poster" />
      <span className="continue-play"><Play size={22} fill="currentColor" /></span>
    </span>
    <span className="continue-title">{action}</span>
    <span className="continue-meta">{when ? `${status}, ${when.toLowerCase()}` : status}</span>
  </button>;
}
