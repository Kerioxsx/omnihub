package dev.vesper.client.fx;

import dev.vesper.client.module.General;
import dev.vesper.client.render.Colors;

/**
 * One running kill effect: a timeline of phases over {@link #LENGTH_MS}
 * (scaled by the duration setting). Subclasses precompute their seeded
 * shapes in {@link #setup()} and draw a frame in {@link #draw(float)} from
 * nothing but the phase time, so frame rate never changes the result.
 * Coordinates handed to FxDraw are in "base space": the origin of the effect
 * is at (ox, oy, oz) for the current frame.
 */
public abstract class KillEffect {
    public static final float LENGTH_MS = 1500;
    /** Per-effect particle cap (VISUAL_SPEC 7.1). */
    public static final int CAP = 160;

    /** Origin (victim's feet) in the space's own coordinates. */
    protected double x, y, z;
    protected float height = 1.8f;
    protected int space;
    protected double start;
    /** Real seconds per timeline second (duration setting, shorter in reduced motion). */
    protected float dur = 1;
    protected int tint;
    protected boolean useTint;
    protected float intensity = 1;
    protected boolean calm;
    /** Multiplier for flash-type elements: 1 normally, 0.5 with reduced flashes / reduced motion. */
    protected float flash = 1;
    protected long seed;
    protected Rng rng;
    protected int spawned;
    private int once;

    /** Base-space origin for the frame being drawn. */
    protected double ox, oy, oz;

    public final void begin(int space, double x, double y, double z, double height, KillSettings s, double now, long seed) {
        this.space = space;
        this.x = x;
        this.y = y;
        this.z = z;
        this.height = (float) Math.max(0.6, Math.min(3.5, height <= 0 ? 1.8 : height));
        this.start = now;
        this.calm = s.calm;
        this.dur = Math.max(0.3f, s.duration * (s.calm ? 0.75f : 1f));
        this.tint = s.tint;
        this.useTint = s.useTint;
        this.intensity = Math.max(0.2f, Math.min(1.5f, s.intensity));
        this.flash = (s.reducedFlashes || s.calm) ? 0.5f : 1f;
        this.seed = seed;
        this.rng = new Rng(seed);
        setup();
    }

    protected abstract void setup();

    protected abstract void draw(float ms);

    /** @return false once finished. (offX, offY, offZ) is subtracted to get base space. */
    public final boolean render(double now, double offX, double offY, double offZ) {
        float ms = (float) ((now - start) * 1000.0 / dur);
        if (ms >= LENGTH_MS) return false;
        if (ms < 0) ms = 0;
        ox = x - offX;
        oy = y - offY;
        oz = z - offZ;
        draw(ms);
        FxDraw.flush();
        return true;
    }

    public final boolean finished(double now) {
        return (now - start) * 1000.0 / dur >= LENGTH_MS;
    }

    /** True exactly once, on the first frame at or after timeline time `at`. */
    protected final boolean once(int bit, float ms, float at) {
        if (ms < at || (once & (1 << bit)) != 0) return false;
        once |= 1 << bit;
        return true;
    }

    /** Absolute clock time of a timeline instant (particles are scheduled on it). */
    protected final double at(float ms) {
        return start + ms * dur / 1000.0;
    }

    /** Timeline milliseconds to real seconds (particle lifetimes). */
    protected final float sec(float ms) {
        return ms * dur / 1000f;
    }

    /** A count scaled by intensity, quality and reduced motion. */
    protected final int count(int base) {
        int d = General.detail();
        float q = d <= 0 ? 0.5f : d == 1 ? 0.65f : 1f;
        return Math.max(1, Math.round(base * intensity * q * (calm ? 0.6f : 1f)));
    }

    /** Overall alpha factor from intensity. */
    protected final float strength() {
        return Math.min(1f, 0.55f + 0.45f * intensity);
    }

    /** Applies the colour override (bright whites keep most of their whiteness) and intensity. */
    protected final int c(int argb) {
        int out = argb;
        if (useTint) {
            float lum = 0.3f * Colors.r(argb) + 0.59f * Colors.g(argb) + 0.11f * Colors.b(argb);
            float k = 0.8f * (1 - lum * lum * lum * lum * 0.7f);
            out = Colors.mix(argb, (argb & 0xFF000000) | (tint & 0xFFFFFF), k);
        }
        return Colors.alpha(out, strength());
    }

    protected final int c(int argb, float alpha) {
        return Colors.alpha(c(argb), alpha);
    }

    /** Spawns a particle at an offset from the origin; respects the per-effect cap. */
    protected final int spawn(int texture, float atMs, double dx, double dy, double dz, float lifeMs) {
        if (spawned >= CAP) return -1;
        int i = Particles.spawn(space, texture, at(atMs), x + dx, y + dy, z + dz, sec(lifeMs));
        if (i >= 0) spawned++;
        return i;
    }

    /** Horizontal camera-facing axis for cylindrical billboards at base-space point (bx, bz). */
    protected static double[] sideAxis(double bx, double bz, double[] out) {
        double dx = View.cx - bx, dz = View.cz - bz, l = Math.sqrt(dx * dx + dz * dz);
        if (l < 1e-6) {
            out[0] = View.rx;
            out[1] = View.rz;
        } else {
            out[0] = -dz / l;
            out[1] = dx / l;
        }
        return out;
    }
}
