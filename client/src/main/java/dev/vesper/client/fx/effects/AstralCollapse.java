package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Textures;

/**
 * Astral Collapse (VISUAL_SPEC 7.2, blue/teal): 24 star points spiral in
 * toward the chest while a ground ring contracts; a core glow swells,
 * flashes and bursts into 40 drifting micro-stars that twinkle out.
 */
public final class AstralCollapse extends KillEffect {
    private static final int POINTS = 24;
    private final float[] dirX = new float[POINTS], dirY = new float[POINTS], dirZ = new float[POINTS];
    private float spinDir;

    @Override
    protected void setup() {
        // Fibonacci sphere with a seeded jitter and rotation.
        float off = rng.f() * 6.283f;
        spinDir = rng.chance(0.5f) ? 1 : -1;
        for (int i = 0; i < POINTS; i++) {
            float yy = 1 - (i + 0.5f) / POINTS * 2;
            float r = (float) Math.sqrt(Math.max(0, 1 - yy * yy));
            float th = i * 2.39996f + off + rng.signed() * 0.2f;
            dirX[i] = (float) Math.cos(th) * r;
            dirY[i] = yy * 0.8f;
            dirZ[i] = (float) Math.sin(th) * r;
        }
    }

    private double cy() {
        return oy + Math.min(1.0, height * 0.56);
    }

    @Override
    protected void draw(float ms) {
        double ccx = ox, ccy = cy(), ccz = oz;

        // Burst particles at 700 ms (scheduled on the exact instant even if this frame is late).
        if (once(0, ms, 700)) {
            int n = count(40);
            for (int k = 0; k < n; k++) {
                float u = rng.signed(), th = rng.f() * 6.283f, r = (float) Math.sqrt(1 - u * u);
                float sp = rng.range(3f, 5f);
                int i = spawn(Particles.TEX_TWINKLE, 700, ccx - ox, ccy - oy, ccz - oz, 800 + rng.range(-60, 0));
                Particles.vel(i, (float) Math.cos(th) * r * sp, u * sp * 0.8f, (float) Math.sin(th) * r * sp, 2.1f / dur);
                Particles.gravity(i, 0, -0.2f);
                Particles.size(i, rng.range(0.05f, 0.08f), 0.03f);
                Particles.color(i, c(0xFFDFF8FF), c(0xFF38C6F4));
                Particles.flicker(i, 12, rng.f() * 6.283f);
                Particles.spin(i, rng.f() * 3f, rng.signed() * 2f);
            }
        }

        FxDraw.geo();
        // Ground ring: appears at r 1.4, contracts to 0.3 (350-700), fades by 820.
        if (ms < 820) {
            float r = ms < 350 ? 1.4f : 1.4f - 1.1f * Ease.inOutCubic(Ease.span(ms, 350, 700));
            float a = Ease.span(ms, 0, 120) * (1 - Ease.span(ms, 700, 820));
            FxDraw.glowRingH(ox, oy + 0.03, oz, r, 0.06, c(0xFF7FE7FF, a), 48);
        }
        // Spiral trails: each point's three previous positions as a short fading line.
        if (ms < 720) {
            for (int i = 0; i < POINTS; i++) {
                double px0 = 0, py0 = 0, pz0 = 0;
                for (int k = 0; k < 4; k++) {
                    float tm = Math.max(0, ms - k * 28);
                    double r = pos(i, tm), th = theta(tm);
                    double cs = Math.cos(th), sn = Math.sin(th);
                    double px = ccx + (dirX[i] * cs - dirZ[i] * sn) * r, py = ccy + dirY[i] * r, pz = ccz + (dirX[i] * sn + dirZ[i] * cs) * r;
                    if (k > 0) {
                        float a = (1 - k / 4f) * 0.55f * (1 - Ease.span(ms, 650, 720)) * Ease.span(ms, 0, 80);
                        FxDraw.line(px0, py0, pz0, px, py, pz, 0.035 * (1 - k / 5.0), c(0xFF7FE7FF, a), c(0xFF38C6F4, a * 0.4f));
                    }
                    px0 = px;
                    py0 = py;
                    pz0 = pz;
                }
            }
        }
        FxDraw.flush();

        FxDraw.sprites(Textures.twinkle());
        if (ms < 720) {
            float a = Ease.span(ms, 0, 80) * (1 - Ease.span(ms, 660, 720));
            double r = pos(0, ms), th = theta(ms);
            double cs = Math.cos(th), sn = Math.sin(th);
            for (int i = 0; i < POINTS; i++) {
                r = pos(i, ms);
                double px = ccx + (dirX[i] * cs - dirZ[i] * sn) * r, py = ccy + dirY[i] * r, pz = ccz + (dirX[i] * sn + dirZ[i] * cs) * r;
                FxDraw.sprite(px, py, pz, 0.10, ms * 0.004 + i, c(0xFFE8FCFF, a));
            }
        }
        FxDraw.flush();

        FxDraw.sprites(Textures.glow());
        if (ms < 720) {
            float a = Ease.span(ms, 0, 80) * (1 - Ease.span(ms, 660, 720)) * 0.35f;
            double th = theta(ms), cs = Math.cos(th), sn = Math.sin(th);
            for (int i = 0; i < POINTS; i++) {
                double r = pos(i, ms);
                FxDraw.sprite(ccx + (dirX[i] * cs - dirZ[i] * sn) * r, ccy + dirY[i] * r, ccz + (dirX[i] * sn + dirZ[i] * cs) * r, 0.28, c(0xFF38C6F4, a));
            }
        }
        // Core glow: grows 350-700 to 0.8, flashes to 1.2 then collapses by 850.
        float core = 0;
        if (ms >= 350 && ms < 700) core = 0.8f * Ease.outCubic(Ease.span(ms, 350, 700));
        else if (ms >= 700 && ms < 760) core = 0.8f + 0.4f * flash * Ease.outCubic(Ease.span(ms, 700, 760));
        else if (ms >= 760 && ms < 850) core = (0.8f + 0.4f * flash) * (1 - Ease.inQuad(Ease.span(ms, 760, 850)));
        if (core > 0) {
            float a = ms >= 700 ? flash : 1;
            FxDraw.glow(ccx, ccy, ccz, core * 0.5, c(0xFFFFFFFF, a), c(0xFF38C6F4, a));
        }
        // A faint afterglow where the core was, fading with the micro-stars.
        if (ms >= 800) {
            float a = 0.25f * (1 - Ease.inQuad(Ease.span(ms, 800, 1500)));
            FxDraw.sprite(ccx, ccy, ccz, 0.9, c(0xFF38C6F4, a));
        }
        FxDraw.flush();
    }

    /** Radius of point i at time ms: 1.6 → ~0.15 over 0-700 (easeInQuad), with a tiny per-point offset. */
    private double pos(int i, float ms) {
        float t = Ease.span(ms, 0, 700);
        return 0.15 + (1.45 + (i % 3) * 0.05) * (1 - Ease.inQuad(t));
    }

    /** Spiral angle: +540 degrees across the gather/compress phases. */
    private double theta(float ms) {
        float t = Ease.span(ms, 0, 700);
        return spinDir * Math.PI * 3 * (t * 0.4 + Ease.inQuad(t) * 0.6);
    }
}
