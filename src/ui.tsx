import { FormEvent, ReactNode, RefObject, useEffect, useState } from 'react';
import { Film, Search } from 'lucide-react';

function hueOf(text: string): number {
  let h = 0;
  for (const ch of text) h = (h * 31 + ch.codePointAt(0)!) % 360;
  return h;
}

function initialsOf(title: string): string {
  const words = title.replace(/[^\p{L}\p{N}\s]/gu, ' ').split(/\s+/).filter(Boolean);
  return (words.length > 1 ? words[0][0] + words[1][0] : (words[0] ?? '?').slice(0, 2)).toUpperCase();
}

/**
 * Cover image. Falls back to a tile (the title's initials when `title` is given) when there is no URL,
 * the image fails to load, or it is a tiny placeholder pixel, which some sites serve for lazy-loaded covers.
 * `no-referrer` because some image hosts reject requests that carry the app's own origin.
 */
export function Poster({ src, title, className = '' }: { src?: string | null; title?: string; className?: string }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  if (!src?.trim() || failed) {
    if (!title) return <div className={`poster poster-empty ${className}`} aria-hidden="true"><Film size={22} strokeWidth={1.5} /></div>;
    const hue = hueOf(title);
    return <div className={`poster poster-empty has-initials ${className}`} aria-hidden="true" style={{ background: `hsl(${hue} 26% 19%)`, color: `hsl(${hue} 38% 74%)` }}>{initialsOf(title)}</div>;
  }
  return <img className={`poster ${className}`} src={src} alt="" loading="lazy" referrerPolicy="no-referrer" onError={() => setFailed(true)} onLoad={e => { if (e.currentTarget.naturalWidth < 24) setFailed(true); }} />;
}

type CardProps = {
  poster?: string | null;
  title: string;
  subtitle?: string;
  selected?: boolean;
  disabled?: boolean;
  onClick: () => void;
  /** Rendered next to the main button (e.g. a remove button), so buttons are never nested. */
  extra?: ReactNode;
};

/** Poster on top, title in a dark panel underneath. */
export function PosterCard({ poster, title, subtitle, selected, disabled, onClick, extra }: CardProps) {
  return <div className={`card ${selected ? 'selected' : ''}`}>
    <button className="card-main" onClick={onClick} disabled={disabled} title={title}>
      <Poster src={poster} title={title} className="card-poster" />
      <span className="card-body">
        <span className="card-title">{title}</span>
        {subtitle && <span className="card-sub">{subtitle}</span>}
      </span>
    </button>
    {extra}
  </div>;
}

export function Spinner() {
  return <span className="spinner" role="progressbar" aria-label="Loading" />;
}

type SearchBoxProps = {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void;
  busy: boolean;
  inputRef?: RefObject<HTMLInputElement>;
  autoFocus?: boolean;
};

export function SearchBox({ value, onChange, onSubmit, busy, inputRef, autoFocus }: SearchBoxProps) {
  return <form className="searchbox" onSubmit={onSubmit} role="search">
    <Search size={18} />
    <input ref={inputRef} value={value} onChange={e => onChange(e.target.value)} placeholder="Search anime" aria-label="Search anime" autoFocus={autoFocus} />
    {busy && <Spinner />}
    <button type="submit" disabled={busy || !value.trim()}>Search</button>
  </form>;
}
