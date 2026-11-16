package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.FxText;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.Textures;

/**
 * KA-POW! A comic-book burst: a spiky yellow star with a red rim slams
 * open, halftone dots fly out, the word punches in, and a cartoon smoke
 * cloud puffs where the body was.
 */
public final class KaPow extends KillEffect {
    private static final int SPIKES = 14;
    private final float[] spike = new float[SPIKES];
    private final double[] side = new double[2];

    @Override
    protected void setup() {
        for (int i = 0; i < SPIKES; i++) spike[i] = rng.range(0.75f, 1.15f);
    }

    private void star(double cx, double cy, double cz, double sx, double sz, double rIn, double rOut, int cIn, int cOut, float rot) {
        for (int i = 0; i < SPIKES; i++) {
            double a0 = rot + i * Math.PI * 2 / SPIKES, a1 = rot + (i + 0.5) * Math.PI * 2 / SPIKES, a2 = rot + (i + 1) * Math.PI * 2 / SPIKES;
            double ro = rOut * spike[i];
            // Triangle fan as quads: centre, inner corner, spike tip, next inner corner.
            FxDraw.quad(cx, cy, cz,
                cx + sx * Math.cos(a0) * rIn, cy + Math.sin(a0) * rIn, cz + sz * Math.cos(a0) * rIn,
                cx + sx * Math.cos(a1) * ro, cy + Math.sin(a1) * ro, cz + sz * Math.cos(a1) * ro,
                cx + sx * Math.cos(a2) * rIn, cy + Math.sin(a2) * rIn, cz + sz * Math.cos(a2) * rIn,
                cIn, cIn, cOut, cIn);
        }
    }

    @Override
    protected void draw(float ms) {
        sideAxis(ox, oz, side);
        double sx = side[0], sz = side[1];
        double cx = ox, cy = oy + height * 0.6, cz = oz;
        if (once(0, ms, 60)) {
            int n = count(30);
            for (int k = 0; k < n; k++) {
                float t = rng.f() * 6.283f, sp = rng.range(2.5f, 5f);
                int i = spawn(Particles.TEX_GLOW, 60, 0, height * 0.6, 0, rng.range(350, 600));
                Particles.vel(i, (float) (sx * Math.cos(t)) * sp, (float) Math.sin(t) * sp, (float) (sz * Math.cos(t)) * sp, 4f);
                Particles.size(i, rng.range(0.06f, 0.1f), 0.03f);
                Particles.color(i, c(0xFFFF3B3B), c(0x00FFB000));
            }
        }
        float open = Ease.outBack(Ease.span(ms, 40, 220));
        float a = 1 - Ease.span(ms, 950, 1250);
        if (a > 0 && open > 0) {
            Gl.solidBlend();
            FxDraw.geo();
            float rot = (float) Math.toRadians(-6 + ms * 0.004);
            star(cx, cy, cz, sx, sz, 0.55 * open, 1.25 * open, c(0xFFE8202A, a), c(0xFFE8202A, a), rot);
            star(cx, cy, cz, sx, sz, 0.42 * open, 0.98 * open, c(0xFFFFE14D, a), c(0xFFFFB000, a), rot + 0.11f);
            FxDraw.flush();
            Gl.additive();
            // Halftone dots around the burst.
            FxDraw.sprites(Textures.glow());
            for (int k = 0; k < 18; k++) {
                double ang = k * 0.349 + 0.2, rr = (1.35 + (k % 3) * 0.18) * open;
                FxDraw.sprite(cx + sx * Math.cos(ang) * rr, cy + Math.sin(ang) * rr, cz + sz * Math.cos(ang) * rr, 0.07, c(0xFFFF6B3B, a * 0.8f));
            }
            FxDraw.flush();
            float punch = 1.25f - 0.25f * Ease.outCubic(Ease.span(ms, 60, 260));
            FxText.text("KA-POW!", cx, cy + 0.02, cz, 0.5 * open * punch, -7, c(0xFFFFFFFF, a), c(0xFF26050A, a));
        }
        // Smoke puffs after the burst.
        if (ms > 500) {
            float s = Ease.outCubic(Ease.span(ms, 500, 1500)), sa = (1 - Ease.span(ms, 1000, 1500)) * Ease.span(ms, 500, 650);
            Gl.solidBlend();
            FxDraw.geo();
            for (int k = 0; k < 6; k++) {
                double ang = k * 1.047 + 0.5, rr = 0.35 + 0.25 * s;
                double px = ox + sx * Math.cos(ang) * rr, py = oy + 0.35 + Math.sin(ang) * rr * 0.5 + s * 0.5, pz = oz + sz * Math.cos(ang) * rr;
                FxDraw.disc(px, py, pz, sx, 0, sz, 0, 1, 0, 0.28 + 0.12 * s, c(0xFFE9E4DA, sa * 0.85f), c(0xFFBDB5A6, sa * 0.6f), 18);
            }
            FxDraw.flush();
            Gl.additive();
        }
    }
}
