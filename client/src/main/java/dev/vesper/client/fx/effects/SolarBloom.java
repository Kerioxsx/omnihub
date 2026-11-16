package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Textures;

/**
 * Solar Bloom (VISUAL_SPEC 7.4, red/amber): a warm core gathers at the
 * chest, blooms into eight petal rays and a radial ground wave, then
 * square embers scatter, rise and cool from gold to deep red.
 */
public final class SolarBloom extends KillEffect {
    private static final int RAYS = 8;
    private final float[] rayAng = new float[RAYS], rayLen = new float[RAYS];
    private final double[] side = new double[2];

    @Override
    protected void setup() {
        float off = rng.f() * 6.283f;
        for (int i = 0; i < RAYS; i++) {
            rayAng[i] = off + i * 6.283f / RAYS + rng.signed() * 0.12f;
            rayLen[i] = rng.range(0.9f, 1.4f);
        }
    }

    @Override
    protected void draw(float ms) {
        double cx = ox, cy = oy + Math.min(1.0, height * 0.55), cz = oz;
        if (once(0, ms, 420)) {
            int n = count(46);
            for (int k = 0; k < n; k++) {
                float th = rng.f() * 6.283f, up = rng.range(0.4f, 1.6f), sp = rng.range(1.6f, 3.4f);
                int i = spawn(Particles.TEX_SQUARE, 420, 0, cy - oy, 0, rng.range(700, 1050));
                Particles.vel(i, (float) Math.cos(th) * sp, up * 1.4f, (float) Math.sin(th) * sp, 2.4f / dur);
                Particles.gravity(i, -0.6f, 0.35f);
                Particles.size(i, rng.range(0.05f, 0.09f), 0.02f);
                Particles.color3(i, c(0xFFFFF2B0), c(0xFFFFA62B), c(0xFFB3261E));
                Particles.spin(i, rng.f() * 3, rng.signed() * 5);
                Particles.halo(i, 2.2f);
            }
        }
        FxDraw.geo();
        // Radial ground wave 420-1000 ms.
        if (ms >= 420 && ms < 1000) {
            float t = Ease.outCubic(Ease.span(ms, 420, 1000));
            float a = 1 - Ease.span(ms, 600, 1000);
            FxDraw.ringH(ox, oy + 0.03, oz, 0.2 + 2.2 * t, 0.35 + 2.4 * t, c(0x00FF8A3D, 0), c(0xFFFFB347, a * 0.7f), 56);
            FxDraw.glowRingH(ox, oy + 0.035, oz, 0.3 + 2.3 * t, 0.05, c(0xFFFFE08A, a), 56);
        }
        // Petal rays 380-900 ms: camera-facing tapered beams from the core.
        if (ms >= 380 && ms < 900) {
            float grow = Ease.outBack(Ease.span(ms, 380, 560)), a = 1 - Ease.span(ms, 700, 900);
            sideAxis(cx, cz, side);
            for (int i = 0; i < RAYS; i++) {
                double dx = Math.cos(rayAng[i]), dy = Math.sin(rayAng[i]);
                double len = rayLen[i] * grow;
                double ex = cx + side[0] * dx * len, ey = cy + dy * len, ez = cz + side[1] * dx * len;
                FxDraw.line(cx, cy, cz, ex, ey, ez, 0.3 * (1 - Ease.span(ms, 560, 900) * 0.6), c(0xFFFFF4C2, a * flash), c(0x55FF7A1A, 0.35f * a));
                FxDraw.line(cx, cy, cz, ex, ey, ez, 0.08, c(0xFFFFFFFF, a * flash), c(0x00FFB347, 0));
            }
        }
        FxDraw.flush();
        FxDraw.sprites(Textures.glow());
        // Core: gathers 0-380, blooms 380-480, fades by 1000.
        float core = ms < 380 ? 0.5f * Ease.inQuad(Ease.span(ms, 0, 380)) : ms < 480 ? 0.5f + 0.8f * flash * Ease.outCubic(Ease.span(ms, 380, 480)) : (0.5f + 0.8f * flash) * (1 - Ease.inQuad(Ease.span(ms, 480, 1100)));
        if (core > 0.01f) FxDraw.glow(cx, cy, cz, core, c(0xFFFFF6D8), c(0xFFFF8A3D));
        // Heat haze sprites drifting up while the embers fly.
        if (ms > 450) {
            float a = 0.18f * (1 - Ease.span(ms, 450, 1500));
            for (int k = 0; k < 3; k++) FxDraw.sprite(cx, cy + (ms - 450) / 1000.0 * (0.6 + k * 0.3), cz, 0.6 + k * 0.2, c(0xFFFF6A2B, a));
        }
        FxDraw.flush();
    }
}
