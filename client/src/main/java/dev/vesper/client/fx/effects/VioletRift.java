package dev.vesper.client.fx.effects;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.KillEffect;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;

/**
 * Violet Rift (VISUAL_SPEC 7.3, violet/magenta): an upright 16-sided rift
 * ring opens facing the camera with a rotating inner square, its interior
 * darkens, right-angled neon bolts crawl across the ground, thin shards rise
 * from the rim, then the ring closes into a purple haze.
 */
public final class VioletRift extends KillEffect {
    private static final int BOLTS = 6, MAX_SEG = 6, SHARDS = 18;
    /** Bolt points (x,z offsets from the origin), (MAX_SEG+1) points per bolt. */
    private final float[] boltX = new float[BOLTS * (MAX_SEG + 1)], boltZ = new float[BOLTS * (MAX_SEG + 1)];
    private final int[] boltSegs = new int[BOLTS];
    private final float[] boltLen = new float[BOLTS];
    private final float[] shardAng = new float[SHARDS], shardSpeed = new float[SHARDS], shardT = new float[SHARDS], shardTilt = new float[SHARDS];
    private int shardCount;
    private final double[] side = new double[2];

    @Override
    protected void setup() {
        float base = rng.f() * 6.283f;
        for (int b = 0; b < BOLTS; b++) {
            float ang = base + b * 6.283f / BOLTS + rng.signed() * 0.25f;
            float dx = (float) Math.cos(ang), dz = (float) Math.sin(ang);
            int segs = 3 + rng.i(3);
            float total = rng.range(1.0f, 2.5f);
            boltSegs[b] = segs;
            int o = b * (MAX_SEG + 1);
            float x = dx * 0.35f, z = dz * 0.35f;
            boltX[o] = x;
            boltZ[o] = z;
            float len = 0;
            boolean alongX = Math.abs(dx) > Math.abs(dz);
            for (int s = 1; s <= segs; s++) {
                float l = total / segs * rng.range(0.6f, 1.4f);
                if (alongX) x += Math.signum(dx) * l;
                else z += Math.signum(dz) * l;
                alongX = !alongX;
                boltX[o + s] = x;
                boltZ[o + s] = z;
                len += l;
            }
            boltLen[b] = len;
        }
        shardCount = Math.min(SHARDS, count(SHARDS));
        for (int i = 0; i < shardCount; i++) {
            shardAng[i] = rng.f() * 6.283f;
            shardSpeed[i] = rng.range(1.5f, 3f);
            shardT[i] = 450 + rng.f() * 150;
            shardTilt[i] = rng.signed();
        }
        spawned += shardCount;
    }

    private float radius(float ms) {
        if (ms < 300) return 0.9f * Ease.outBack(ms / 300f);
        if (ms < 1000) return 0.9f;
        return 0.9f * (1 - Ease.inQuad(Ease.span(ms, 1000, 1500)));
    }

    @Override
    protected void draw(float ms) {
        double cx = ox, cy = oy + Math.min(1.0, height * 0.56), cz = oz;
        sideAxis(cx, cz, side);
        double ux = side[0], uz = side[1];
        float r = Math.max(0, radius(ms));

        if (once(0, ms, 1000)) {
            int n = count(10);
            for (int k = 0; k < n; k++) {
                int i = spawn(Particles.TEX_GLOW, 1000, rng.signed() * 0.6, cy - oy + rng.signed() * 0.5, rng.signed() * 0.6, 500);
                float s = rng.range(0.6f, 1.0f);
                Particles.size(i, s, s * 1.4f);
                Particles.color(i, c(0x337B3FC4), c(0x004A1D80));
                Particles.vel(i, rng.signed() * 0.3f, 0.25f, rng.signed() * 0.3f, 1.5f);
            }
        }

        // Dark interior (normal blending so it actually darkens).
        float fill = Ease.span(ms, 300, 650);
        if (fill > 0 && r > 0.02f) {
            Gl.solidBlend();
            FxDraw.geo();
            FxDraw.disc(cx, cy, cz, ux, 0, uz, 0, 1, 0, r * 0.97, c(0xFF14051F, 0.85f * fill), c(0xFF7B3FC4, 0.6f * fill), 32);
            FxDraw.flush();
            Gl.additive();
        }

        FxDraw.geo();
        if (r > 0.01f) {
            // Rim: 16-gon with a halo, plus the rotating inner square.
            FxDraw.polygon(cx, cy, cz, ux, 0, uz, 0, 1, 0, r, 16, 0, 0.11, c(0xFFC77DFF, 0.25f));
            FxDraw.polygon(cx, cy, cz, ux, 0, uz, 0, 1, 0, r, 16, 0, 0.035, c(0xFFC77DFF));
            float rot = (float) Math.toRadians(ms * 0.09);
            FxDraw.polygon(cx, cy, cz, ux, 0, uz, 0, 1, 0, r * 0.6, 4, rot, 0.025, c(0xFFF0B8FF, 0.9f));
            if (fill > 0) FxDraw.ring(cx, cy, cz, ux, 0, uz, 0, 1, 0, r * 0.75, r, 0x00000000, c(0xFFB45CFF, 0.35f * fill), 32);
        }
        // Neon bolts crawl outward along the ground (grow 300-600, hold, fade 1000-1300).
        if (ms >= 300 && ms < 1300) {
            float grow = Ease.outCubic(Ease.span(ms, 300, 600));
            float a = 1 - Ease.span(ms, 1000, 1300);
            int core = c(0xFFC77DFF, a), halo = c(0xFFC77DFF, a * 0.25f);
            double gy = oy + 0.03;
            for (int b = 0; b < BOLTS; b++) {
                int o = b * (MAX_SEG + 1);
                float remain = grow * boltLen[b];
                for (int s = 1; s <= boltSegs[b] && remain > 0; s++) {
                    float x0 = boltX[o + s - 1], z0 = boltZ[o + s - 1], x1 = boltX[o + s], z1 = boltZ[o + s];
                    float l = Math.abs(x1 - x0) + Math.abs(z1 - z0);
                    float k = Math.min(1, remain / Math.max(1e-4f, l));
                    double ex = x0 + (x1 - x0) * k, ez = z0 + (z1 - z0) * k;
                    FxDraw.line(ox + x0, gy, oz + z0, ox + ex, gy, oz + ez, 0.1, halo);
                    FxDraw.line(ox + x0, gy, oz + z0, ox + ex, gy, oz + ez, 0.03, core);
                    remain -= l;
                }
            }
        }
        // Shards rising from the rim, 500 ms each.
        for (int i = 0; i < shardCount; i++) {
            float age = ms - shardT[i];
            if (age < 0 || age > 500) continue;
            float u = age / 500f;
            double ca = Math.cos(shardAng[i]) * 0.9, sa = Math.sin(shardAng[i]) * 0.9;
            double bx = cx + ux * ca, by = cy + sa, bz = cz + uz * ca;
            by += shardSpeed[i] * age / 1000.0;
            double tilt = shardTilt[i] * 0.12 * Math.sin(age / 90.0);
            FxDraw.line(bx, by, bz, bx + ux * tilt, by + 0.35, bz + uz * tilt, 0.04, c(0xFFE6CCFF, 1 - u), c(0xFFE6CCFF, (1 - u) * 0.2f));
        }
        FxDraw.flush();

        FxDraw.sprites(dev.vesper.client.render.Textures.glow());
        if (r > 0.01f) {
            // Rim glow (bloom substitute) and a soft violet haze while open.
            FxDraw.sprite(cx, cy, cz, r * 1.9, Colors.alpha(c(0xFF7B3FC4), 0.22f * Math.min(1, r / 0.9f)));
        }
        if (ms < 250) {
            float a = flash * (1 - Ease.span(ms, 0, 250));
            FxDraw.sprite(cx, cy, cz, 0.6, c(0xFFF0B8FF, 0.6f * a));
        }
        FxDraw.flush();
    }
}
