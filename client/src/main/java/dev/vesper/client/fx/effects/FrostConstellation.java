package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.fx.View;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Textures;

/**
 * Frost Constellation (VISUAL_SPEC 7.5, icy blue): six crystal stars rise
 * into a ring around the body and link up as a constellation, a cold
 * twinkle runs round the ring, then the crystals shatter into snow and
 * everything fades clean.
 */
public final class FrostConstellation extends KillEffect {
    private static final int STARS = 6;
    private final float[] ang = new float[STARS], hgt = new float[STARS];

    @Override
    protected void setup() {
        float off = rng.f() * 6.283f;
        for (int i = 0; i < STARS; i++) {
            ang[i] = off + i * 6.283f / STARS + rng.signed() * 0.15f;
            hgt[i] = rng.range(0.6f, 1.5f);
        }
    }

    private double sx(int i, float r) {
        return ox + Math.cos(ang[i]) * r;
    }

    private double sz(int i, float r) {
        return oz + Math.sin(ang[i]) * r;
    }

    @Override
    protected void draw(float ms) {
        float rise = Ease.outCubic(Ease.span(ms, 0, 450));
        float r = 0.95f + 0.1f * Ease.span(ms, 450, 1100);
        float fade = 1 - Ease.span(ms, 1050, 1350);
        if (once(0, ms, 1050)) {
            for (int s = 0; s < STARS; s++) {
                int n = count(9);
                for (int k = 0; k < n; k++) {
                    int i = spawn(Particles.TEX_TWINKLE, 1050, sx(s, r) - x, oy - y + hgt[s] * height / 1.8f, sz(s, r) - z, rng.range(450, 650));
                    Particles.vel(i, rng.signed() * 1.2f, rng.range(-0.2f, 0.8f), rng.signed() * 1.2f, 3f / dur);
                    Particles.gravity(i, 0.8f, 0);
                    Particles.size(i, rng.range(0.04f, 0.07f), 0.02f);
                    Particles.color(i, c(0xFFFFFFFF), c(0xFF9BE7FF));
                    Particles.spin(i, rng.f() * 3, rng.signed() * 3);
                }
            }
        }
        FxDraw.geo();
        // Constellation links 350-1100 ms, drawn on progressively.
        float link = Ease.span(ms, 350, 650);
        if (link > 0 && fade > 0) {
            for (int i = 0; i < STARS; i++) {
                int j = (i + 1) % STARS;
                double y1 = oy + hgt[i] * rise * height / 1.8f, y2 = oy + hgt[j] * rise * height / 1.8f;
                double x1 = sx(i, r), z1 = sz(i, r), x2 = sx(j, r), z2 = sz(j, r);
                double f = Math.min(1, link * STARS - i);
                if (f <= 0) continue;
                FxDraw.line(x1, y1, z1, x1 + (x2 - x1) * f, y1 + (y2 - y1) * f, z1 + (z2 - z1) * f, 0.025, c(0xFFCFF4FF, 0.75f * fade), c(0xFF7FD3FF, 0.6f * fade));
            }
        }
        // Frost ring on the ground.
        if (fade > 0) FxDraw.glowRingH(ox, oy + 0.03, oz, r * Ease.outCubic(Ease.span(ms, 100, 500)), 0.045, c(0xFFBFEFFF, 0.8f * fade), 48);
        // Crystal shapes: hexagons at each star.
        for (int i = 0; i < STARS && fade > 0; i++) {
            double yy = oy + hgt[i] * rise * height / 1.8f;
            FxDraw.polygon(sx(i, r), yy, sz(i, r), View.rx, View.ry, View.rz, View.ux, View.uy, View.uz, 0.09 * rise, 6, ms * 0.002 + i, 0.015, c(0xFFE6FAFF, fade));
        }
        FxDraw.flush();
        FxDraw.sprites(Textures.twinkle());
        // Twinkle chase around the ring 650-1050 ms.
        for (int i = 0; i < STARS && fade > 0; i++) {
            double yy = oy + hgt[i] * rise * height / 1.8f;
            float chase = Ease.bell(Ease.span(ms, 650 + i * 60, 820 + i * 60));
            FxDraw.sprite(sx(i, r), yy, sz(i, r), 0.12 + 0.22 * chase * flash, ms * 0.003, c(0xFFFFFFFF, Math.min(1, rise * 1.5f) * fade));
        }
        FxDraw.flush();
        FxDraw.sprites(Textures.glow());
        if (fade > 0) FxDraw.sprite(ox, oy + 0.9 * height / 1.8, oz, 1.1, c(0xFF6EC8FF, 0.16f * rise * fade));
        FxDraw.flush();
    }
}
