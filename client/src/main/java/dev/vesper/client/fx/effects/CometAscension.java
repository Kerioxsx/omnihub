package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Textures;

/**
 * Comet Ascension (VISUAL_SPEC 7.6, blue-white): three comets arc in past
 * the body from different sides, cross at the chest, then rise together
 * into the sky leaving long fading trails and a shower of sparks.
 */
public final class CometAscension extends KillEffect {
    private static final int COMETS = 3, TRAIL = 14;
    private final float[] start = new float[COMETS];
    private final double[] pts = new double[TRAIL * 3];

    @Override
    protected void setup() {
        float off = rng.f() * 6.283f;
        for (int i = 0; i < COMETS; i++) start[i] = off + i * 6.283f / COMETS;
    }

    /** Position of comet i at timeline ms into out[0..2] (base space). */
    private void pos(int i, float ms, double[] out, int o) {
        double chest = oy + height * 0.55;
        if (ms < 450) {
            // Arc in from 2.2 blocks out and 1.5 up to the chest, spiralling a quarter turn.
            float t = Ease.inOutCubic(Ease.span(ms, 0, 450));
            double a = start[i] + t * 1.6, rad = 2.2 * (1 - t);
            out[o] = ox + Math.cos(a) * rad;
            out[o + 1] = chest + 1.5 * (1 - t) * (1 - t);
            out[o + 2] = oz + Math.sin(a) * rad;
        } else {
            // Rise in a tight helix to 7 blocks.
            float t = Ease.inQuad(Ease.span(ms, 450, 1300));
            double a = start[i] + 1.6 + t * 6, rad = 0.25 + 0.35 * t;
            out[o] = ox + Math.cos(a) * rad;
            out[o + 1] = chest + t * 7;
            out[o + 2] = oz + Math.sin(a) * rad;
        }
    }

    @Override
    protected void draw(float ms) {
        if (once(0, ms, 450)) {
            int n = count(30);
            for (int k = 0; k < n; k++) {
                float th = rng.f() * 6.283f, sp = rng.range(1.5f, 3.2f);
                int i = spawn(Particles.TEX_TWINKLE, 450, 0, height * 0.55, 0, rng.range(500, 800));
                Particles.vel(i, (float) Math.cos(th) * sp, rng.range(-0.5f, 2f), (float) Math.sin(th) * sp, 2.6f / dur);
                Particles.gravity(i, 1.5f, 0);
                Particles.size(i, rng.range(0.04f, 0.07f), 0.01f);
                Particles.color(i, c(0xFFFFFFFF), c(0xFF5AB8FF));
            }
        }
        float fade = 1 - Ease.span(ms, 1150, 1450);
        FxDraw.geo();
        for (int i = 0; i < COMETS && fade > 0; i++) {
            for (int k = 0; k < TRAIL; k++) pos(i, Math.max(0, ms - k * 22), pts, k * 3);
            FxDraw.ribbon(pts, TRAIL, 0.16, 0.01, c(0xFFEAF6FF, fade), c(0x002A7BFF, 0));
        }
        // A flash ring where they cross.
        if (ms >= 420 && ms < 750) {
            float t = Ease.outCubic(Ease.span(ms, 420, 750));
            FxDraw.glowRingH(ox, oy + height * 0.55, oz, 0.2 + 1.3 * t, 0.05, c(0xFFBFE4FF, (1 - t) * flash), 48);
        }
        FxDraw.flush();
        FxDraw.sprites(Textures.glow());
        double[] p = new double[3];
        for (int i = 0; i < COMETS && fade > 0; i++) {
            pos(i, ms, p, 0);
            FxDraw.glow(p[0], p[1], p[2], 0.22, c(0xFFFFFFFF, fade), c(0xFF4FA8FF, fade));
        }
        if (ms >= 430 && ms < 620) FxDraw.glow(ox, oy + height * 0.55, oz, 0.7 * Ease.bell(Ease.span(ms, 430, 620)) * flash, c(0xFFFFFFFF), c(0xFF4FA8FF));
        FxDraw.flush();
    }
}
