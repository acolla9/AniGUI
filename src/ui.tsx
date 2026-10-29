import { FormEvent, ReactNode, RefObject, useEffect, useState } from 'react';
import { Film, Search, Sparkles } from 'lucide-react';

/**
 * Cover image. Falls back to a plain tile when there is no URL or the image fails to load.
 * `no-referrer` because some image hosts reject requests that carry the app's own origin.
 */
export function Poster({ src, className = '' }: { src?: string | null; className?: string }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  if (!src || failed) {
    return <div className={`poster poster-empty ${className}`} aria-hidden="true"><Film size={22} strokeWidth={1.5} /></div>;
  }
  return <img className={`poster ${className}`} src={src} alt="" loading="lazy" referrerPolicy="no-referrer" onError={() => setFailed(true)} />;
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

/** Modern Anime Poster card with rounded corners, subtle badge, and smooth lift on hover */
export function PosterCard({ poster, title, subtitle, selected, disabled, onClick, extra }: CardProps) {
  return (
    <div className={`card ${selected ? 'selected' : ''}`}>
      <button className="card-main" onClick={onClick} disabled={disabled} title={title}>
        <div className="card-poster-wrap">
          <Poster src={poster} className="card-poster" />
          <div className="card-overlay-badge">
            <Sparkles size={13} />
          </div>
        </div>
        <span className="card-body">
          <span className="card-title">{title}</span>
          {subtitle && <span className="card-sub">{subtitle}</span>}
        </span>
      </button>
      {extra}
    </div>
  );
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
