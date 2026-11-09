// Behind everything: the photo wallpaper filling the screen, or the look's
// colours. A soft veil keeps text readable on busy photos.

import { useLook } from '../lib/look';

export function WallLayer() {
  const { wallUrl, resolved } = useLook();
  return (
    <div className="m-wall" aria-hidden>
      {wallUrl && resolved.wall ? (
        <>
          <img src={wallUrl} alt="" draggable={false} decoding="async" />
          <i className={resolved.palette.light ? 'veil is-light' : 'veil'} />
        </>
      ) : (
        <i className="tint" />
      )}
    </div>
  );
}
