package dev.vesper.client.cosmetic;

import dev.vesper.client.module.General;

/**
 * Animation state of one place cosmetics are drawn (world, GUI preview,
 * showroom). Advancing it once per frame keeps motion frame-rate
 * independent; advancing it twice in one frame is harmless (dt is ~0).
 */
public final class Scene {
    public static final Scene WORLD = new Scene();
    public static final Scene PREVIEW = new Scene();
    public static final Scene STAGE = new Scene();

    float clock;
    float wingPhase;
    float fastBlend;
    float sneakBlend;
    private float last = -1;

    /** Advances the clock to the absolute time `now` (seconds) and fills the pose's timing fields. */
    public void advance(Pose p, float now) {
        float dt = last < 0 ? 0 : now - last;
        if (dt < 0 || dt > 0.25f) dt = 0.016f;
        last = now;
        float rate = General.calm() ? 0.45f : 1f;
        float k = 1 - (float) Math.exp(-6 * dt);
        fastBlend += ((p.fast ? 1 : 0) - fastBlend) * k;
        sneakBlend += ((p.sneaking ? 1 : 0) - sneakBlend) * Math.min(1, k * 2);
        dt *= rate;
        clock += dt;
        float period = 1.6f + (0.6f - 1.6f) * fastBlend;
        wingPhase = (wingPhase + dt / period) % 1f;
        p.time = clock;
        p.dt = dt;
        p.scene = this;
    }
}
