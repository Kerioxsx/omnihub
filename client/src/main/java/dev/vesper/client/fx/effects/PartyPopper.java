package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.FxText;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;

/**
 * Party Popper: a pop, a fountain of confetti in six colours tumbling down,
 * curly streamers, and a rainbow "GG!" bouncing above the spot.
 */
public final class PartyPopper extends KillEffect {
    private static final int[] PARTY = {0xFFFF4F9A, 0xFFFFD23F, 0xFF3FE0FF, 0xFF7CFF5B, 0xFFFF8A2B, 0xFFB57CFF};
    private static final int STREAMERS = 5;
    private final float[] stAng = new float[STREAMERS], stCurl = new float[STREAMERS];
    private final double[] pts = new double[24 * 3];

    @Override
    protected void setup() {
        for (int i = 0; i < STREAMERS; i++) {
            stAng[i] = rng.f() * 6.283f;
            stCurl[i] = rng.range(5f, 9f) * (rng.chance(0.5f) ? 1 : -1);
        }
    }

    @Override
    protected void draw(float ms) {
        if (once(0, ms, 100)) {
            int n = count(120);
            for (int k = 0; k < n; k++) {
                float th = rng.f() * 6.283f, out = rng.range(0.6f, 2.2f), up = rng.range(4.5f, 7.5f);
                int i = spawn(Particles.TEX_SQUARE, 100, 0, 0.3, 0, rng.range(1100, 1400));
                Particles.vel(i, (float) Math.cos(th) * out, up, (float) Math.sin(th) * out, 1.6f / dur);
                Particles.gravity(i, 9.5f, 0);
                Particles.size(i, rng.range(0.05f, 0.08f), 0.05f);
                int col = c(PARTY[rng.i(PARTY.length)]);
                Particles.color(i, col, Colors.alpha(col, 0.7f));
                Particles.spin(i, rng.f() * 6, rng.signed() * 14);
                Particles.flicker(i, rng.range(5, 9), rng.f() * 6);
            }
        }
        // The pop: a quick ring and flash at the feet.
        if (ms >= 80 && ms < 380) {
            float t = Ease.outCubic(Ease.span(ms, 80, 380));
            FxDraw.geo();
            FxDraw.glowRingH(ox, oy + 0.05, oz, 0.2 + 1.3 * t, 0.05, c(0xFFFFFFFF, (1 - t) * flash), 40);
            FxDraw.flush();
        }
        // Streamers: curly ribbons rising and drooping.
        if (ms >= 120 && ms < 1400) {
            float grow = Ease.outCubic(Ease.span(ms, 120, 620)), a = 1 - Ease.span(ms, 1000, 1400);
            FxDraw.geo();
            for (int s = 0; s < STREAMERS; s++) {
                int n = 24;
                for (int k = 0; k < n; k++) {
                    float f = k / (float) (n - 1) * grow;
                    double rise = Math.sin(f * Math.PI * 0.85) * 2.2 - Ease.span(ms, 600, 1400) * f * 1.2;
                    double out = f * 1.4, curl = f * stCurl[s];
                    double ang = stAng[s] + Math.sin(curl) * 0.35;
                    pts[k * 3] = ox + Math.cos(ang) * out + Math.cos(curl) * 0.12;
                    pts[k * 3 + 1] = oy + 0.3 + rise;
                    pts[k * 3 + 2] = oz + Math.sin(ang) * out + Math.sin(curl) * 0.12;
                }
                int col = PARTY[(s * 2) % PARTY.length];
                FxDraw.ribbon(pts, n, 0.07, 0.03, c(col, a), c(col, a * 0.6f));
            }
            FxDraw.flush();
        }
        // GG! cycling through the colours, bouncing.
        if (ms >= 150) {
            float pop = Ease.outBack(Ease.span(ms, 150, 380));
            float a = 1 - Ease.span(ms, 1150, 1450);
            int col = Colors.mix(PARTY[(int) (ms / 120) % PARTY.length], PARTY[((int) (ms / 120) + 1) % PARTY.length], (ms % 120) / 120f);
            double bounce = Math.abs(Math.sin((ms - 150) / 1000.0 * Math.PI * 2.5)) * 0.18 * (1 - Ease.span(ms, 150, 1200));
            FxText.text("GG!", ox, oy + height + 0.8 + bounce, oz, 0.62 * pop, 0, c(col, a), c(0xFF1A0B2E, a));
        }
    }
}
