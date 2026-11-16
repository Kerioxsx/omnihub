package dev.vesper.client.fx;

import dev.vesper.client.module.General;
import dev.vesper.client.render.Colors;

/**
 * Pooled, time-based particles (struct of arrays, no per-frame allocation).
 * A particle's position is evaluated analytically from its spawn time, so
 * the motion is identical at any frame rate:
 * <pre>
 *   f = (1 - e^(-k*a)) / k            (drag k per second; f = a when k = 0)
 *   p = p0 + v*f + (0, drift*a - g*a^2/2, 0)
 * </pre>
 * Alpha fades in over the first 6% of life and out as 1 - u^2 (easeInQuad).
 * Two spaces: {@link #WORLD} (absolute world coordinates, drawn relative to
 * the viewer) and {@link #STAGE} (the showroom's own coordinates). One global
 * cap: {@link General#particleBudget()} across both spaces.
 */
public final class Particles {
    public static final int WORLD = 0, STAGE = 1;
    public static final int TEX_GLOW = 0, TEX_TWINKLE = 1, TEX_SQUARE = 2;
    private static final int CAP = 2048;
    private static final long START = System.nanoTime();

    private static final double[] px = new double[CAP], py = new double[CAP], pz = new double[CAP], t0 = new double[CAP];
    private static final float[] vx = new float[CAP], vy = new float[CAP], vz = new float[CAP];
    private static final float[] drag = new float[CAP], grav = new float[CAP], drift = new float[CAP], life = new float[CAP];
    private static final float[] s0 = new float[CAP], s1 = new float[CAP], rot0 = new float[CAP], rotV = new float[CAP];
    private static final float[] flick = new float[CAP], phase = new float[CAP], glowHalo = new float[CAP];
    private static final int[] c0 = new int[CAP], c1 = new int[CAP], c2 = new int[CAP];
    private static final byte[] tex = new byte[CAP], space = new byte[CAP];
    private static final boolean[] three = new boolean[CAP];
    private static int n;

    private Particles() {
    }

    /** Monotonic seconds used for every particle and effect clock. */
    public static double now() {
        return (System.nanoTime() - START) / 1e9;
    }

    public static int count() {
        return n;
    }

    public static int count(int sp) {
        int k = 0;
        for (int i = 0; i < n; i++) if (space[i] == sp) k++;
        return k;
    }

    public static int budget() {
        return Math.min(CAP, Math.max(16, General.particleBudget()));
    }

    /**
     * Reserves a particle (still, white, glow texture, size 0.1) or returns -1
     * when the global budget is used up. Configure it with the setters.
     */
    public static int spawn(int sp, int texture, double start, double x, double y, double z, float lifeSec) {
        if (n >= budget() || lifeSec <= 0) return -1;
        int i = n++;
        space[i] = (byte) sp;
        tex[i] = (byte) texture;
        t0[i] = start;
        px[i] = x;
        py[i] = y;
        pz[i] = z;
        life[i] = lifeSec;
        vx[i] = vy[i] = vz[i] = 0;
        drag[i] = grav[i] = drift[i] = 0;
        s0[i] = s1[i] = 0.1f;
        rot0[i] = rotV[i] = 0;
        flick[i] = 0;
        phase[i] = 0;
        glowHalo[i] = 0;
        c0[i] = c1[i] = c2[i] = 0xFFFFFFFF;
        three[i] = false;
        return i;
    }

    public static void vel(int i, float x, float y, float z, float dragPerSec) {
        if (i < 0) return;
        vx[i] = x;
        vy[i] = y;
        vz[i] = z;
        drag[i] = dragPerSec;
    }

    public static void gravity(int i, float g, float driftY) {
        if (i < 0) return;
        grav[i] = g;
        drift[i] = driftY;
    }

    public static void size(int i, float from, float to) {
        if (i < 0) return;
        s0[i] = from;
        s1[i] = to;
    }

    public static void color(int i, int from, int to) {
        if (i < 0) return;
        c0[i] = from;
        c1[i] = to;
        three[i] = false;
    }

    public static void color3(int i, int a, int b, int c) {
        if (i < 0) return;
        c0[i] = a;
        c1[i] = b;
        c2[i] = c;
        three[i] = true;
    }

    public static void spin(int i, float rot, float perSec) {
        if (i < 0) return;
        rot0[i] = rot;
        rotV[i] = perSec;
    }

    /** Alpha flicker at hz (decays with life); ph = phase offset in radians. */
    public static void flicker(int i, float hz, float ph) {
        if (i < 0) return;
        flick[i] = hz;
        phase[i] = ph;
    }

    /** Adds a faint halo sprite at this multiple of the size (0 = none); glow texture only. */
    public static void halo(int i, float mult) {
        if (i < 0) return;
        glowHalo[i] = mult;
    }

    private static void remove(int i) {
        int j = --n;
        if (i == j) return;
        px[i] = px[j];
        py[i] = py[j];
        pz[i] = pz[j];
        t0[i] = t0[j];
        vx[i] = vx[j];
        vy[i] = vy[j];
        vz[i] = vz[j];
        drag[i] = drag[j];
        grav[i] = grav[j];
        drift[i] = drift[j];
        life[i] = life[j];
        s0[i] = s0[j];
        s1[i] = s1[j];
        rot0[i] = rot0[j];
        rotV[i] = rotV[j];
        flick[i] = flick[j];
        phase[i] = phase[j];
        glowHalo[i] = glowHalo[j];
        c0[i] = c0[j];
        c1[i] = c1[j];
        c2[i] = c2[j];
        tex[i] = tex[j];
        space[i] = space[j];
        three[i] = three[j];
    }

    /** Drops expired particles (called every client tick and before drawing). */
    public static void prune() {
        double t = now();
        for (int i = 0; i < n; ) {
            if (t - t0[i] >= life[i]) remove(i);
            else i++;
        }
    }

    public static void clear() {
        n = 0;
    }

    public static void clear(int sp) {
        for (int i = 0; i < n; ) {
            if (space[i] == sp) remove(i);
            else i++;
        }
    }

    /**
     * Draws one space's particles. (ox,oy,oz) is subtracted from every
     * position (the viewer for WORLD, 0 for STAGE). Call inside
     * Gl.beginGlow/endGlow after View.capture().
     */
    public static void render(int sp, double ox, double oy, double oz) {
        prune();
        if (n == 0) return;
        double t = now();
        for (int texture = 0; texture < 3; texture++) {
            boolean any = false;
            for (int i = 0; i < n; i++) {
                if (space[i] == sp && tex[i] == texture && t >= t0[i]) {
                    any = true;
                    break;
                }
            }
            if (!any) continue;
            FxDraw.sprites(FxTextures.get(texture));
            for (int i = 0; i < n; i++) {
                if (space[i] != sp || tex[i] != texture) continue;
                float a = (float) (t - t0[i]);
                if (a < 0) continue;
                float u = a / life[i];
                double f = drag[i] > 0 ? (1 - Math.exp(-drag[i] * a)) / drag[i] : a;
                double x = px[i] + vx[i] * f - ox;
                double y = py[i] + vy[i] * f + drift[i] * a - 0.5 * grav[i] * a * a - oy;
                double z = pz[i] + vz[i] * f - oz;
                int c;
                if (three[i]) c = u < 0.5f ? Colors.mix(c0[i], c1[i], u * 2) : Colors.mix(c1[i], c2[i], u * 2 - 1);
                else c = Colors.mix(c0[i], c1[i], u);
                float al = Math.min(1, u / 0.06f) * (1 - u * u);
                if (flick[i] > 0) {
                    float fl = 0.5f + 0.5f * (float) Math.sin(a * flick[i] * Math.PI * 2 + phase[i]);
                    al *= 1 - (1 - u * 0.5f) * 0.75f * (1 - fl);
                }
                c = Colors.alpha(c, al);
                double s = s0[i] + (s1[i] - s0[i]) * u;
                double rot = rot0[i] + rotV[i] * a;
                if (glowHalo[i] > 0) FxDraw.sprite(x, y, z, s * glowHalo[i], 0, Colors.alpha(c, 0.3f));
                FxDraw.sprite(x, y, z, s, rot, c);
            }
            FxDraw.flush();
        }
    }
}
