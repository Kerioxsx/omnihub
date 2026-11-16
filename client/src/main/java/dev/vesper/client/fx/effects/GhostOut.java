package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.FxText;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;

/**
 * Ghost Out: a little cartoon ghost floats up out of the body, swaying,
 * waves one arm, says "boo!", and drifts off into the sky.
 */
public final class GhostOut extends KillEffect {
    private final double[] side = new double[2];

    @Override
    protected void setup() {
    }

    @Override
    protected void draw(float ms) {
        sideAxis(ox, oz, side);
        double sx = side[0], sz = side[1];
        float rise = Ease.inOutCubic(Ease.span(ms, 0, 1500));
        float a = Ease.clamp01(ms / 200f) * (1 - Ease.span(ms, 1150, 1500));
        double sway = Math.sin(ms / 1000.0 * 5.5) * 0.18;
        double cu = sway, cv = height * 0.55 + rise * 2.6; // ghost centre in (side, up)
        double r = 0.36;
        if (once(0, ms, 0)) {
            int n = count(14);
            for (int k = 0; k < n; k++) {
                float t = rng.f() * 6.283f;
                int i = spawn(Particles.TEX_GLOW, rng.range(0, 600), Math.cos(t) * 0.3, rng.range(0.2f, 1.4f), Math.sin(t) * 0.3, rng.range(500, 800));
                Particles.vel(i, 0, rng.range(0.6f, 1.4f), 0, 1f);
                Particles.size(i, 0.12f, 0.02f);
                Particles.color(i, c(0xCC9FE8FF), c(0x003F7FFF));
            }
        }
        if (a <= 0.01f) return;
        Gl.solidBlend();
        FxDraw.geo();
        int body = c(0xFFF4F7FF, a * 0.92f), edge = c(0xFFDCE6FF, a * 0.92f);
        // Head: a disc facing the camera (in the side/up plane).
        double hx = ox + sx * cu, hy = oy + cv, hz = oz + sz * cu;
        FxDraw.disc(hx, hy, hz, sx, 0, sz, 0, 1, 0, r, body, edge, 28);
        // Body: from the head's middle down to a wavy hem.
        int seg = 8;
        double hem = 0.5, wave = ms / 1000.0 * 9;
        for (int i = 0; i < seg; i++) {
            double u0 = -r + 2 * r * i / seg, u1 = -r + 2 * r * (i + 1) / seg;
            double w0 = Math.sin(i * 1.6 + wave) * 0.06, w1 = Math.sin((i + 1) * 1.6 + wave) * 0.06;
            double b0 = cv - hem + w0 - (i % 2 == 0 ? 0 : 0.07), b1 = cv - hem + w1 - ((i + 1) % 2 == 0 ? 0 : 0.07);
            FxDraw.quad(ox + sx * (cu + u0), oy + cv, oz + sz * (cu + u0), ox + sx * (cu + u1), oy + cv, oz + sz * (cu + u1),
                ox + sx * (cu + u1), oy + b1, oz + sz * (cu + u1), ox + sx * (cu + u0), oy + b0, oz + sz * (cu + u0), body, body, edge, edge);
        }
        // Waving arm on the right.
        double wv = Math.sin(ms / 1000.0 * 14) * 0.35;
        double ax0 = cu + r * 0.85, ay0 = cv - 0.12, ax1 = cu + r * 0.85 + 0.25, ay1 = cv + 0.05 + wv * 0.4;
        FxDraw.line(ox + sx * ax0, oy + ay0, oz + sz * ax0, ox + sx * ax1, oy + ay1, oz + sz * ax1, 0.1, body);
        // Eyes and mouth.
        int ink = c(0xFF1B1530, a);
        for (int e = -1; e <= 1; e += 2) {
            double eu = cu + e * 0.12, ev = cv + 0.04;
            FxDraw.disc(ox + sx * eu, oy + ev, oz + sz * eu, sx, 0, sz, 0, 1, 0, 0.055, ink, ink, 12);
        }
        double mo = 0.04 + 0.03 * Math.abs(Math.sin(ms / 1000.0 * 8));
        FxDraw.disc(ox + sx * cu, oy + cv - 0.14, oz + sz * cu, sx, 0, sz, 0, 1, 0, mo, ink, ink, 12);
        FxDraw.flush();
        Gl.additive();
        if (ms > 350 && ms < 1250) {
            float ta = Ease.span(ms, 350, 480) * (1 - Ease.span(ms, 1050, 1250));
            FxText.text("boo!", ox + sx * (cu - 0.55), oy + cv + 0.45, oz + sz * (cu - 0.55), 0.3 * Ease.outBack(Ease.span(ms, 350, 520)), 10, c(0xFFFFFFFF, ta * a), c(0xFF2C2350, ta * a));
        }
    }
}
