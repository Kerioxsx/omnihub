package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.FxText;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.Textures;

/**
 * Bonk! A cartoon mallet swings down onto the head, "BONK!" pops out, a
 * puff of dust, and three dizzy stars circle where the head was.
 */
public final class Bonk extends KillEffect {
    private static final float HIT = 260;
    private final double[] side = new double[2];

    @Override
    protected void setup() {
    }

    @Override
    protected void draw(float ms) {
        sideAxis(ox, oz, side);
        double sx = side[0], sz = side[1];
        double headY = oy + height + 0.12;
        // Mallet angle in the camera-facing plane: wind-up, hit, a little bounce.
        double start = Math.toRadians(100), hit = Math.toRadians(191);
        double th;
        if (ms < HIT) th = start + (hit - start) * Ease.inQuad(ms / HIT);
        else if (ms < 520) th = hit - Math.toRadians(16) * Math.sin(Ease.span(ms, HIT, 520) * Math.PI);
        else th = hit;
        float fade = 1 - Ease.span(ms, 900, 1200);
        if (fade > 0) {
            double pu = 1.0, pv = height + 0.36, L = 1.02;
            double hu = pu + Math.cos(th) * L, hv = pv + Math.sin(th) * L;
            double px = ox + sx * pu, py = oy + pv, pz = oz + sz * pu;
            double hx = ox + sx * hu, hy = oy + hv, hz = oz + sz * hu;
            Gl.solidBlend();
            FxDraw.geo();
            FxDraw.line(px, py, pz, hx, hy, hz, 0.09, c(0xFF8A5A2B, fade));
            // Head: a block across the handle's end.
            double eu = Math.cos(th), ev = Math.sin(th), nu = -ev, nv = eu;
            double a = 0.34, b = 0.2;
            double[] u = {hu + nu * a - eu * b, hu - nu * a - eu * b, hu - nu * a + eu * b, hu + nu * a + eu * b};
            double[] v = {hv + nv * a - ev * b, hv - nv * a - ev * b, hv - nv * a + ev * b, hv + nv * a + ev * b};
            int dark = c(0xFFB3262E, fade), light = c(0xFFFF5A5F, fade);
            FxDraw.quad(ox + sx * u[0], oy + v[0], oz + sz * u[0], ox + sx * u[1], oy + v[1], oz + sz * u[1], ox + sx * u[2], oy + v[2], oz + sz * u[2], ox + sx * u[3], oy + v[3], oz + sz * u[3], dark, dark, light, light);
            // A pale band around the middle of the head.
            double m = 0.06;
            FxDraw.quad(ox + sx * (hu + nu * a - eu * m), oy + hv + nv * a - ev * m, oz + sz * (hu + nu * a - eu * m),
                ox + sx * (hu - nu * a - eu * m), oy + hv - nv * a - ev * m, oz + sz * (hu - nu * a - eu * m),
                ox + sx * (hu - nu * a + eu * m), oy + hv - nv * a + ev * m, oz + sz * (hu - nu * a + eu * m),
                ox + sx * (hu + nu * a + eu * m), oy + hv + nv * a + ev * m, oz + sz * (hu + nu * a + eu * m),
                c(0xFFFFE3C2, fade), c(0xFFFFE3C2, fade), c(0xFFFFE3C2, fade), c(0xFFFFE3C2, fade));
            FxDraw.flush();
            Gl.additive();
        }
        if (once(0, ms, HIT)) {
            int n = count(16);
            for (int k = 0; k < n; k++) {
                float t = rng.f() * 6.283f, sp = rng.range(0.8f, 1.8f);
                int i = spawn(Particles.TEX_GLOW, HIT, Math.cos(t) * 0.2, height + 0.05, Math.sin(t) * 0.2, rng.range(380, 620));
                Particles.vel(i, (float) Math.cos(t) * sp, rng.range(0.2f, 0.9f), (float) Math.sin(t) * sp, 3f);
                Particles.size(i, rng.range(0.16f, 0.26f), 0.36f);
                Particles.color(i, c(0x99FFF6E0), c(0x00C8B89A));
            }
        }
        // Impact lines at the moment of the hit.
        if (ms >= HIT && ms < HIT + 220) {
            float t = Ease.span(ms, HIT, HIT + 220);
            FxDraw.geo();
            for (int k = 0; k < 8; k++) {
                double a = k * Math.PI / 4 + 0.2;
                double r0 = 0.3 + 0.5 * t, r1 = 0.55 + 0.9 * t;
                FxDraw.line(ox + sx * Math.cos(a) * r0, headY + Math.sin(a) * r0, oz + sz * Math.cos(a) * r0, ox + sx * Math.cos(a) * r1, headY + Math.sin(a) * r1, oz + sz * Math.cos(a) * r1, 0.05, c(0xFFFFFFFF, (1 - t) * flash));
            }
            FxDraw.flush();
        }
        // Dizzy stars orbiting the head.
        if (ms > HIT + 80) {
            float a = (1 - Ease.span(ms, 1200, 1500)) * Ease.clamp01((ms - HIT - 80) / 120f);
            FxDraw.sprites(Textures.twinkle());
            for (int k = 0; k < 3; k++) {
                double ang = (ms - HIT) / 1000.0 * 6.5 + k * Math.PI * 2 / 3;
                double r = 0.42;
                double x = ox + Math.cos(ang) * r, z = oz + Math.sin(ang) * r, y = headY + 0.08 + Math.sin(ang * 2) * 0.05;
                FxDraw.sprite(x, y, z, 0.16, ang, c(0xFFFFE14D, a));
            }
            FxDraw.flush();
        }
        // BONK!
        if (ms >= HIT) {
            float pop = Ease.outBack(Ease.span(ms, HIT, HIT + 220));
            float a = 1 - Ease.span(ms, 1050, 1350);
            float wob = (float) Math.sin((ms - HIT) / 70.0) * 6 * (1 - Ease.span(ms, HIT, 700));
            FxText.text("BONK!", ox, headY + 0.75 + 0.2 * Ease.span(ms, HIT, 1350), oz, 0.55 * pop, -8 + wob, c(0xFFFFE14D, a), c(0xFF5A1300, a));
        }
    }
}
