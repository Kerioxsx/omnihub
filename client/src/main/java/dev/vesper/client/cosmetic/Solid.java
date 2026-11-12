package dev.vesper.client.cosmetic;

import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.opengl.GL11;

/**
 * Untextured, unlit procedural geometry (wing ribs, crown links, the
 * showroom cart) batched as triangles, with fake lighting from per-face
 * vertex shades, plus untextured triangle fans/strips and GL lines. All
 * transforms are done on the CPU so one batch covers many oriented boxes.
 */
public final class Solid {
    private static WorldRenderer w;
    private static boolean open;
    private static int mode;

    private Solid() {
    }

    /** Starts a triangle batch (untextured). */
    public static void begin() {
        begin(GL11.GL_TRIANGLES);
    }

    public static void begin(int glMode) {
        if (open) end();
        GlStateManager.disableTexture2D();
        w = Tessellator.getInstance().getWorldRenderer();
        w.begin(glMode, DefaultVertexFormats.POSITION_COLOR);
        mode = glMode;
        open = true;
    }

    public static void end() {
        if (!open) return;
        Tessellator.getInstance().draw();
        open = false;
        if (mode == GL11.GL_LINES || mode == GL11.GL_LINE_STRIP || mode == GL11.GL_LINE_LOOP) {
            GL11.glLineWidth(1f);
            GL11.glDisable(GL11.GL_LINE_SMOOTH);
        }
    }

    /** Starts a line batch of the given width (pixels). */
    public static void beginLines(int glMode, float width) {
        if (open) end();
        GL11.glEnable(GL11.GL_LINE_SMOOTH);
        GL11.glHint(GL11.GL_LINE_SMOOTH_HINT, GL11.GL_NICEST);
        GL11.glLineWidth(width);
        begin(glMode);
    }

    public static void v(float x, float y, float z, int c) {
        w.pos(x, y, z).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    /** Colour scaled by a shade factor (alpha kept). */
    public static int shade(int c, float f) {
        int r = Math.min(255, Math.round(((c >> 16) & 255) * f));
        int g = Math.min(255, Math.round(((c >> 8) & 255) * f));
        int b = Math.min(255, Math.round((c & 255) * f));
        return (c & 0xFF000000) | (r << 16) | (g << 8) | b;
    }

    /** Face shade from a (not necessarily unit) normal: top 1, bottom 0.55, ±x 0.75, ±z 0.85. */
    static float faceShade(float nx, float ny, float nz) {
        float ax = Math.abs(nx), ay = Math.abs(ny), az = Math.abs(nz);
        if (ay >= ax && ay >= az) return ny > 0 ? 1f : 0.55f;
        return ax > az ? 0.75f : 0.85f;
    }

    private static void tri(float ax, float ay, float az, float bx, float by, float bz, float cx, float cy, float cz, int ca, int cb, int cc) {
        v(ax, ay, az, ca);
        v(bx, by, bz, cb);
        v(cx, cy, cz, cc);
    }

    private static void quadShaded(float[] p, int a, int b, int c, int d, int colA, int colB, int colC, int colD) {
        float e1x = p[b * 3] - p[a * 3], e1y = p[b * 3 + 1] - p[a * 3 + 1], e1z = p[b * 3 + 2] - p[a * 3 + 2];
        float e2x = p[d * 3] - p[a * 3], e2y = p[d * 3 + 1] - p[a * 3 + 1], e2z = p[d * 3 + 2] - p[a * 3 + 2];
        float nx = e1y * e2z - e1z * e2y, ny = e1z * e2x - e1x * e2z, nz = e1x * e2y - e1y * e2x;
        float s = faceShade(nx, ny, nz);
        int sa = shade(colA, s), sb = shade(colB, s), sc = shade(colC, s), sd = shade(colD, s);
        tri(p[a * 3], p[a * 3 + 1], p[a * 3 + 2], p[b * 3], p[b * 3 + 1], p[b * 3 + 2], p[c * 3], p[c * 3 + 1], p[c * 3 + 2], sa, sb, sc);
        tri(p[a * 3], p[a * 3 + 1], p[a * 3 + 2], p[c * 3], p[c * 3 + 1], p[c * 3 + 2], p[d * 3], p[d * 3 + 1], p[d * 3 + 2], sa, sc, sd);
    }

    private static final float[] P = new float[24];
    /** Corner i: bit 0 = +x, bit 1 = +y, bit 2 = +z. Faces wound counter-clockwise seen from outside. */
    private static final int[] FACES = {
        0, 2, 3, 1, // -z
        4, 5, 7, 6, // +z
        0, 4, 6, 2, // -x
        1, 3, 7, 5, // +x
        2, 6, 7, 3, // +y
        0, 1, 5, 4, // -y
    };

    /** Oriented box: centre plus three half-axis vectors (x, y, z axes of the box). */
    public static void box(float cx, float cy, float cz,
                           float axx, float axy, float axz,
                           float ayx, float ayy, float ayz,
                           float azx, float azy, float azz, int c) {
        for (int i = 0; i < 8; i++) {
            float sx = (i & 1) == 0 ? -1 : 1;
            float sy = (i & 2) == 0 ? -1 : 1;
            float sz = (i & 4) == 0 ? -1 : 1;
            P[i * 3] = cx + axx * sx + ayx * sy + azx * sz;
            P[i * 3 + 1] = cy + axy * sx + ayy * sy + azy * sz;
            P[i * 3 + 2] = cz + axz * sx + ayz * sy + azz * sz;
        }
        for (int f = 0; f < 6; f++) quadShaded(P, FACES[f * 4], FACES[f * 4 + 1], FACES[f * 4 + 2], FACES[f * 4 + 3], c, c, c, c);
    }

    /** Axis-aligned box between two corners. */
    public static void aabb(float x1, float y1, float z1, float x2, float y2, float z2, int c) {
        box((x1 + x2) / 2, (y1 + y2) / 2, (z1 + z2) / 2, (x2 - x1) / 2, 0, 0, 0, (y2 - y1) / 2, 0, 0, 0, (z2 - z1) / 2, c);
    }

    /** A tapered square beam from a to b (widths w0 at a, w1 at b), colours c0 → c1. */
    public static void beam(float ax, float ay, float az, float bx, float by, float bz, float w0, float w1, int c0, int c1) {
        float dx = bx - ax, dy = by - ay, dz = bz - az;
        float l = (float) Math.sqrt(dx * dx + dy * dy + dz * dz);
        if (l < 1e-5f) return;
        dx /= l;
        dy /= l;
        dz /= l;
        // any vector not parallel to d
        float tx = Math.abs(dy) < 0.9f ? 0 : 1, ty = Math.abs(dy) < 0.9f ? 1 : 0, tz = 0;
        float px = dy * tz - dz * ty, py = dz * tx - dx * tz, pz = dx * ty - dy * tx;
        float pl = (float) Math.sqrt(px * px + py * py + pz * pz);
        px /= pl;
        py /= pl;
        pz /= pl;
        float qx = dy * pz - dz * py, qy = dz * px - dx * pz, qz = dx * py - dy * px;
        float h0 = w0 / 2, h1 = w1 / 2;
        // ring at a: 0..3, ring at b: 4..7
        for (int i = 0; i < 4; i++) {
            float sp = (i == 0 || i == 3) ? -1 : 1, sq = i < 2 ? -1 : 1;
            P[i * 3] = ax + (px * sp + qx * sq) * h0;
            P[i * 3 + 1] = ay + (py * sp + qy * sq) * h0;
            P[i * 3 + 2] = az + (pz * sp + qz * sq) * h0;
            P[(i + 4) * 3] = bx + (px * sp + qx * sq) * h1;
            P[(i + 4) * 3 + 1] = by + (py * sp + qy * sq) * h1;
            P[(i + 4) * 3 + 2] = bz + (pz * sp + qz * sq) * h1;
        }
        for (int i = 0; i < 4; i++) {
            int j = (i + 1) % 4;
            quadShaded(P, i, j, j + 4, i + 4, c0, c0, c1, c1);
        }
        quadShaded(P, 0, 3, 2, 1, c0, c0, c0, c0);
        quadShaded(P, 4, 5, 6, 7, c1, c1, c1, c1);
    }

    /** A four-sided pyramid with its base centred at b, apex at b + h, base half-size r. */
    public static void spike(float bx, float by, float bz, float hx, float hy, float hz, float r, int base, int tip) {
        beam(bx, by, bz, bx + hx, by + hy, bz + hz, r * 2, 0.0005f, base, tip);
    }

    // ---- flat shapes (no shading) ----

    /** A disc fan in the plane spanned by the unit vectors (ax..) and (bx..), centre colour → rim colour. Needs begin(GL_TRIANGLES). */
    public static void disc(float cx, float cy, float cz, float ax, float ay, float az, float bx, float by, float bz,
                            float r, int center, int rim, int segs) {
        float px = cx + ax * r, py = cy + ay * r, pz = cz + az * r;
        for (int i = 1; i <= segs; i++) {
            double a = i * Math.PI * 2 / segs;
            float c = (float) Math.cos(a), s = (float) Math.sin(a);
            float qx = cx + (ax * c + bx * s) * r, qy = cy + (ay * c + by * s) * r, qz = cz + (az * c + bz * s) * r;
            tri(cx, cy, cz, px, py, pz, qx, qy, qz, center, rim, rim);
            px = qx;
            py = qy;
            pz = qz;
        }
    }

    /** An annulus (r1..r2) between angles a0..a1 (radians) in the plane (a, b). Needs begin(GL_TRIANGLES). */
    public static void annulus(float cx, float cy, float cz, float ax, float ay, float az, float bx, float by, float bz,
                               float r1, float r2, double a0, double a1, int inner, int outer, int segs) {
        double step = (a1 - a0) / segs;
        for (int i = 0; i < segs; i++) {
            double t0 = a0 + i * step, t1 = t0 + step;
            float c0 = (float) Math.cos(t0), s0 = (float) Math.sin(t0), c1 = (float) Math.cos(t1), s1 = (float) Math.sin(t1);
            float dx0 = ax * c0 + bx * s0, dy0 = ay * c0 + by * s0, dz0 = az * c0 + bz * s0;
            float dx1 = ax * c1 + bx * s1, dy1 = ay * c1 + by * s1, dz1 = az * c1 + bz * s1;
            tri(cx + dx0 * r1, cy + dy0 * r1, cz + dz0 * r1, cx + dx0 * r2, cy + dy0 * r2, cz + dz0 * r2, cx + dx1 * r2, cy + dy1 * r2, cz + dz1 * r2, inner, outer, outer);
            tri(cx + dx0 * r1, cy + dy0 * r1, cz + dz0 * r1, cx + dx1 * r2, cy + dy1 * r2, cz + dz1 * r2, cx + dx1 * r1, cy + dy1 * r1, cz + dz1 * r1, inner, outer, inner);
        }
    }

    /** One raw triangle with per-vertex colours. Needs begin(GL_TRIANGLES). */
    public static void triangle(float ax, float ay, float az, float bx, float by, float bz, float cx, float cy, float cz, int ca, int cb, int cc) {
        tri(ax, ay, az, bx, by, bz, cx, cy, cz, ca, cb, cc);
    }
}
