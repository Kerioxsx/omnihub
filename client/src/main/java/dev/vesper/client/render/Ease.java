package dev.vesper.client.render;

/** Easing curves (t in 0..1) and small time helpers. */
public final class Ease {
    private Ease() {
    }

    public static float clamp01(float t) {
        return t < 0 ? 0 : t > 1 ? 1 : t;
    }

    public static float outCubic(float t) {
        t = clamp01(t) - 1;
        return t * t * t + 1;
    }

    public static float inOutCubic(float t) {
        t = clamp01(t);
        return t < 0.5f ? 4 * t * t * t : 1 - (float) Math.pow(-2 * t + 2, 3) / 2;
    }

    public static float outQuint(float t) {
        t = clamp01(t) - 1;
        return t * t * t * t * t + 1;
    }

    public static float outBack(float t) {
        t = clamp01(t);
        float c1 = 1.70158f, c3 = c1 + 1;
        return 1 + c3 * (float) Math.pow(t - 1, 3) + c1 * (float) Math.pow(t - 1, 2);
    }

    public static float inQuad(float t) {
        t = clamp01(t);
        return t * t;
    }

    public static float outExpo(float t) {
        t = clamp01(t);
        return t >= 1 ? 1 : 1 - (float) Math.pow(2, -10 * t);
    }

    /** 0→1→0 over the span (a smooth pulse). */
    public static float bell(float t) {
        t = clamp01(t);
        return (float) Math.sin(t * Math.PI);
    }

    /** Linear ramp from a to b, clamped (where does now sit between a and b). */
    public static float span(float now, float a, float b) {
        return clamp01((now - a) / (b - a));
    }

    public static float lerp(float a, float b, float t) {
        return a + (b - a) * t;
    }

    /** Seconds since start (monotonic, independent of frame rate). */
    public static float seconds() {
        return (System.nanoTime() - START) / 1e9f;
    }

    private static final long START = System.nanoTime();

    /** Moves `cur` toward `target` at a frame-rate independent speed. */
    public static float approach(float cur, float target, float speed, float dt) {
        return cur + (target - cur) * (1 - (float) Math.exp(-speed * dt));
    }
}
