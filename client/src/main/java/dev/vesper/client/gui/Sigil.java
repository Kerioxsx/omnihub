package dev.vesper.client.gui;

import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;

/**
 * Vesper's original monoline sigil ("evening star split by the horizon",
 * VISUAL_SPEC §3): an elongated four-point star cut in two, the left half
 * dropped, with a small companion diamond. Unit square, y up.
 */
public final class Sigil {
    private Sigil() {
    }

    private static final float[][] RIGHT = {{0.53f, 0.95f}, {0.58f, 0.56f}, {0.86f, 0.50f}, {0.58f, 0.44f}, {0.53f, 0.09f}};
    private static final float[][] LEFT = {{0.47f, 0.91f}, {0.42f, 0.52f}, {0.14f, 0.46f}, {0.42f, 0.40f}, {0.47f, 0.05f}};
    private static final float[][] DIAMOND = {{0.80f, 0.88f}, {0.85f, 0.82f}, {0.80f, 0.76f}, {0.75f, 0.82f}, {0.80f, 0.88f}};
    private static final float[][] TICKS = {{0.62f, 0.27f, 0.74f, 0.27f}, {0.26f, 0.23f, 0.38f, 0.23f}};

    /** Number of strokes, for the draw-on animation. */
    public static final int STROKES = 4 + 4 + 4 + 2;

    /**
     * Draws the sigil in the square (x, y, size) in GUI units, top-left
     * anchored. `drawn` 0..1 reveals the strokes in order (1 = complete).
     */
    public static void draw(float x, float y, float size, int argb, float drawn) {
        float lw = Math.max(1.1f, size / 26f);
        float total = drawn * STROKES;
        int k = 0;
        k = strip(RIGHT, x, y, size, lw, argb, total, k);
        k = strip(LEFT, x, y, size, lw, argb, total, k);
        k = strip(DIAMOND, x, y, size, lw * 0.85f, argb, total, k);
        if (size >= 24) {
            for (float[] t : TICKS) {
                float f = Math.max(0, Math.min(1, total - k));
                if (f > 0) seg(x, y, size, t[0], t[1], t[0] + (t[2] - t[0]) * f, t[3], lw * 0.8f, Colors.alpha(argb, 0.7f));
                k++;
            }
        }
    }

    private static int strip(float[][] p, float x, float y, float size, float lw, int c, float total, int k) {
        for (int i = 0; i + 1 < p.length; i++) {
            float f = Math.max(0, Math.min(1, total - k));
            if (f > 0) seg(x, y, size, p[i][0], p[i][1], p[i][0] + (p[i + 1][0] - p[i][0]) * f, p[i][1] + (p[i + 1][1] - p[i][1]) * f, lw, c);
            k++;
        }
        return k;
    }

    private static void seg(float x, float y, float s, float ux1, float uy1, float ux2, float uy2, float lw, int c) {
        float x1 = x + ux1 * s, y1 = y + (1 - uy1) * s, x2 = x + ux2 * s, y2 = y + (1 - uy2) * s;
        Draw.line(x1, y1, x2, y2, lw, c);
        // Round the joints a little so corners don't notch.
        Draw.circle(x2, y2, lw * 0.5f, c);
    }

    /** Screen position of the top tip (sparkle anchor). */
    public static float tipX(float x, float size) {
        return x + 0.53f * size;
    }

    public static float tipY(float y, float size) {
        return y + 0.05f * size;
    }
}
