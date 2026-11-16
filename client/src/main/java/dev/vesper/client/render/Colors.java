package dev.vesper.client.render;

/** ARGB helpers. */
public final class Colors {
    private Colors() {
    }

    public static int a(int c) {
        return c >>> 24;
    }

    public static float r(int c) {
        return ((c >> 16) & 255) / 255f;
    }

    public static float g(int c) {
        return ((c >> 8) & 255) / 255f;
    }

    public static float b(int c) {
        return (c & 255) / 255f;
    }

    public static float af(int c) {
        return (c >>> 24) / 255f;
    }

    public static int argb(float a, float r, float g, float b) {
        return (clamp(a) << 24) | (clamp(r) << 16) | (clamp(g) << 8) | clamp(b);
    }

    private static int clamp(float v) {
        return Math.max(0, Math.min(255, Math.round(v * 255)));
    }

    /** Same colour with alpha multiplied by f. */
    public static int alpha(int c, float f) {
        return (Math.max(0, Math.min(255, Math.round(a(c) * f))) << 24) | (c & 0xFFFFFF);
    }

    public static int withAlpha(int rgb, float a) {
        return (Math.max(0, Math.min(255, Math.round(a * 255))) << 24) | (rgb & 0xFFFFFF);
    }

    public static int mix(int c1, int c2, float t) {
        return argb(Ease.lerp(af(c1), af(c2), t), Ease.lerp(r(c1), r(c2), t), Ease.lerp(g(c1), g(c2), t), Ease.lerp(b(c1), b(c2), t));
    }
}
