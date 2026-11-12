package dev.vesper.client.fx;

import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.ResourceLocation;
import org.lwjgl.opengl.GL11;

/**
 * Batched 3D drawing for effects. Two kinds of batch, one open at a time:
 * {@link #geo()} for untextured quads (lines, rings, discs, rays) and
 * {@link #sprites} for camera-facing textured quads. Call between
 * Gl.beginGlow/endGlow after {@link View#capture()}; the texture-2D state is
 * left as beginGlow set it (off) after every flush.
 */
public final class FxDraw {
    private static final int NONE = 0, GEO = 1, SPRITE = 2;
    private static int mode = NONE;
    private static WorldRenderer w;

    private FxDraw() {
    }

    public static void geo() {
        flush();
        GlStateManager.disableTexture2D();
        w = Tessellator.getInstance().getWorldRenderer();
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_COLOR);
        mode = GEO;
    }

    public static void sprites(ResourceLocation tex) {
        flush();
        Minecraft.getMinecraft().getTextureManager().bindTexture(tex);
        GlStateManager.enableTexture2D();
        w = Tessellator.getInstance().getWorldRenderer();
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        mode = SPRITE;
    }

    public static void flush() {
        if (mode == NONE) return;
        Tessellator.getInstance().draw();
        if (mode == SPRITE) GlStateManager.disableTexture2D();
        mode = NONE;
    }

    private static void v(double x, double y, double z, int c) {
        w.pos(x, y, z).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    private static void vt(double x, double y, double z, double u, double tv, int c) {
        w.pos(x, y, z).tex(u, tv).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    // ---------------------------------------------------------------- geometry

    public static void quad(double ax, double ay, double az, double bx, double by, double bz, double cx, double cy, double cz, double dx, double dy, double dz, int ca, int cb, int cc, int cd) {
        if (mode != GEO) return;
        if ((ca | cb | cc | cd) >>> 24 == 0) return;
        v(ax, ay, az, ca);
        v(bx, by, bz, cb);
        v(cx, cy, cz, cc);
        v(dx, dy, dz, cd);
    }

    /** A camera-facing line of width wd (blocks) from p1 (colour c1) to p2 (colour c2). */
    public static void line(double x1, double y1, double z1, double x2, double y2, double z2, double wd, int c1, int c2) {
        if (mode != GEO || ((c1 | c2) >>> 24) == 0) return;
        double dx = x2 - x1, dy = y2 - y1, dz = z2 - z1;
        double vx = (x1 + x2) * 0.5 - View.cx, vy = (y1 + y2) * 0.5 - View.cy, vz = (z1 + z2) * 0.5 - View.cz;
        double sx = dy * vz - dz * vy, sy = dz * vx - dx * vz, sz = dx * vy - dy * vx;
        double l = Math.sqrt(sx * sx + sy * sy + sz * sz);
        if (l < 1e-9) return;
        double k = wd * 0.5 / l;
        sx *= k;
        sy *= k;
        sz *= k;
        v(x1 + sx, y1 + sy, z1 + sz, c1);
        v(x2 + sx, y2 + sy, z2 + sz, c2);
        v(x2 - sx, y2 - sy, z2 - sz, c2);
        v(x1 - sx, y1 - sy, z1 - sz, c1);
    }

    public static void line(double x1, double y1, double z1, double x2, double y2, double z2, double wd, int c) {
        line(x1, y1, z1, x2, y2, z2, wd, c, c);
    }

    /** A line drawn twice: a thin core and a wide faint halo (bloom substitute). */
    public static void glowLine(double x1, double y1, double z1, double x2, double y2, double z2, double wd, int c) {
        line(x1, y1, z1, x2, y2, z2, wd * 3.2, dev.vesper.client.render.Colors.alpha(c, 0.25f));
        line(x1, y1, z1, x2, y2, z2, wd, c);
    }

    /**
     * An annulus in the plane spanned by unit vectors u and v around (cx,cy,cz),
     * inner radius r1 coloured ci, outer r2 coloured co.
     */
    public static void ring(double cx, double cy, double cz, double ux, double uy, double uz, double vx, double vy, double vz, double r1, double r2, int ci, int co, int seg) {
        if (mode != GEO || ((ci | co) >>> 24) == 0 || r2 <= 0) return;
        double pc = 1, ps = 0;
        for (int i = 1; i <= seg; i++) {
            double a = i * Math.PI * 2 / seg, c = Math.cos(a), s = Math.sin(a);
            double ax = ux * pc + vx * ps, ay = uy * pc + vy * ps, az = uz * pc + vz * ps;
            double bx = ux * c + vx * s, by = uy * c + vy * s, bz = uz * c + vz * s;
            v(cx + ax * r1, cy + ay * r1, cz + az * r1, ci);
            v(cx + ax * r2, cy + ay * r2, cz + az * r2, co);
            v(cx + bx * r2, cy + by * r2, cz + bz * r2, co);
            v(cx + bx * r1, cy + by * r1, cz + bz * r1, ci);
            pc = c;
            ps = s;
        }
    }

    /** Horizontal annulus (lying on y). */
    public static void ringH(double cx, double cy, double cz, double r1, double r2, int ci, int co, int seg) {
        ring(cx, cy, cz, 1, 0, 0, 0, 0, 1, Math.max(0, r1), r2, ci, co, seg);
    }

    /** A ring line of radius r and width wd lying flat, with a soft halo either side. */
    public static void glowRingH(double cx, double cy, double cz, double r, double wd, int c, int seg) {
        int clear = c & 0xFFFFFF;
        int halo = dev.vesper.client.render.Colors.alpha(c, 0.3f);
        ringH(cx, cy, cz, r - wd * 2.5, r - wd * 0.5, clear, halo, seg);
        ringH(cx, cy, cz, r + wd * 0.5, r + wd * 2.5, halo, clear, seg);
        ringH(cx, cy, cz, r - wd * 0.5, r + wd * 0.5, c, c, seg);
    }

    /** A filled disc (centre colour fading to edge colour) in the plane of u,v. */
    public static void disc(double cx, double cy, double cz, double ux, double uy, double uz, double vx, double vy, double vz, double r, int cc, int ce, int seg) {
        ring(cx, cy, cz, ux, uy, uz, vx, vy, vz, 0, r, cc, ce, seg);
    }

    /** Closed polygon outline (camera-facing segments) of n corners at radius r in the plane of u,v, rotated by rot radians. */
    public static void polygon(double cx, double cy, double cz, double ux, double uy, double uz, double vx, double vy, double vz, double r, int n, double rot, double wd, int c) {
        if (mode != GEO || (c >>> 24) == 0 || r <= 0) return;
        double pa = rot, pc = Math.cos(pa), ps = Math.sin(pa);
        for (int i = 1; i <= n; i++) {
            double a = rot + i * Math.PI * 2 / n, cc = Math.cos(a), s = Math.sin(a);
            line(cx + (ux * pc + vx * ps) * r, cy + (uy * pc + vy * ps) * r, cz + (uz * pc + vz * ps) * r,
                cx + (ux * cc + vx * s) * r, cy + (uy * cc + vy * s) * r, cz + (uz * cc + vz * s) * r, wd, c);
            pc = cc;
            ps = s;
        }
    }

    /**
     * A camera-facing ribbon through n points (xyz triples). Width and colour
     * go from (w0,c0) at index 0 to (w1,c1) at index n-1.
     */
    public static void ribbon(double[] p, int n, double w0, double w1, int c0, int c1) {
        if (mode != GEO || n < 2) return;
        double psx = 0, psy = 0, psz = 0;
        int pcol = c0;
        for (int i = 0; i < n; i++) {
            int a = Math.max(0, i - 1) * 3, b = Math.min(n - 1, i + 1) * 3, o = i * 3;
            double dx = p[b] - p[a], dy = p[b + 1] - p[a + 1], dz = p[b + 2] - p[a + 2];
            double vx = p[o] - View.cx, vy = p[o + 1] - View.cy, vz = p[o + 2] - View.cz;
            double sx = dy * vz - dz * vy, sy = dz * vx - dx * vz, sz = dx * vy - dy * vx;
            double l = Math.sqrt(sx * sx + sy * sy + sz * sz);
            float t = i / (float) (n - 1);
            double hw = (w0 + (w1 - w0) * t) * 0.5;
            if (l > 1e-9) {
                sx *= hw / l;
                sy *= hw / l;
                sz *= hw / l;
            } else {
                sx = psx;
                sy = psy;
                sz = psz;
            }
            int col = dev.vesper.client.render.Colors.mix(c0, c1, t);
            if (i > 0) {
                int q = (i - 1) * 3;
                v(p[q] + psx, p[q + 1] + psy, p[q + 2] + psz, pcol);
                v(p[o] + sx, p[o + 1] + sy, p[o + 2] + sz, col);
                v(p[o] - sx, p[o + 1] - sy, p[o + 2] - sz, col);
                v(p[q] - psx, p[q + 1] - psy, p[q + 2] - psz, pcol);
            }
            psx = sx;
            psy = sy;
            psz = sz;
            pcol = col;
        }
    }

    // ---------------------------------------------------------------- sprites

    /** A camera-facing sprite of half-size s, rotated rot radians in the view plane. */
    public static void sprite(double x, double y, double z, double s, double rot, int c) {
        if (mode != SPRITE || (c >>> 24) == 0 || s <= 0) return;
        double cr = Math.cos(rot) * s, sr = Math.sin(rot) * s;
        double ax = View.rx * cr + View.ux * sr, ay = View.ry * cr + View.uy * sr, az = View.rz * cr + View.uz * sr;
        double bx = -View.rx * sr + View.ux * cr, by = -View.ry * sr + View.uy * cr, bz = -View.rz * sr + View.uz * cr;
        vt(x - ax - bx, y - ay - by, z - az - bz, 0, 1, c);
        vt(x + ax - bx, y + ay - by, z + az - bz, 1, 1, c);
        vt(x + ax + bx, y + ay + by, z + az + bz, 1, 0, c);
        vt(x - ax + bx, y - ay + by, z - az + bz, 0, 0, c);
    }

    public static void sprite(double x, double y, double z, double s, int c) {
        sprite(x, y, z, s, 0, c);
    }

    /** Core + halo (the spec's bloom substitute) with independent colours. */
    public static void glow(double x, double y, double z, double s, int core, int halo) {
        sprite(x, y, z, s * 2.8, 0, dev.vesper.client.render.Colors.alpha(halo, 0.32f));
        sprite(x, y, z, s, 0, core);
    }
}
