package dev.vesper.client.render;

import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.opengl.GL11;

/** Untextured 2D shapes (call between Gl.begin2D and Gl.end2D). Coordinates in GUI pixels. */
public final class Draw {
    private Draw() {
    }

    private static WorldRenderer begin(int mode) {
        WorldRenderer w = Tessellator.getInstance().getWorldRenderer();
        w.begin(mode, DefaultVertexFormats.POSITION_COLOR);
        return w;
    }

    private static void end() {
        Tessellator.getInstance().draw();
    }

    static void v(WorldRenderer w, double x, double y, int c) {
        w.pos(x, y, 0).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    public static void rect(double x1, double y1, double x2, double y2, int c) {
        quad(x1, y1, x2, y1, x2, y2, x1, y2, c, c, c, c);
    }

    public static void gradientV(double x1, double y1, double x2, double y2, int top, int bottom) {
        quad(x1, y1, x2, y1, x2, y2, x1, y2, top, top, bottom, bottom);
    }

    public static void gradientH(double x1, double y1, double x2, double y2, int left, int right) {
        quad(x1, y1, x2, y1, x2, y2, x1, y2, left, right, right, left);
    }

    /** Four corners clockwise from top-left, each with its own colour. */
    public static void quad(double ax, double ay, double bx, double by, double cx, double cy, double dx, double dy, int ca, int cb, int cc, int cd) {
        WorldRenderer w = begin(GL11.GL_QUADS);
        v(w, ax, ay, ca);
        v(w, dx, dy, cd);
        v(w, cx, cy, cc);
        v(w, bx, by, cb);
        end();
    }

    /** A soft round glow: `center` colour fading to transparent at radius r. */
    public static void glow(double cx, double cy, double r, int center) {
        WorldRenderer w = begin(GL11.GL_TRIANGLE_FAN);
        v(w, cx, cy, center);
        int edge = center & 0xFFFFFF;
        int n = 32;
        for (int i = 0; i <= n; i++) {
            double a = i * Math.PI * 2 / n;
            v(w, cx + Math.cos(a) * r, cy + Math.sin(a) * r, edge);
        }
        end();
    }

    public static void circle(double cx, double cy, double r, int c) {
        WorldRenderer w = begin(GL11.GL_TRIANGLE_FAN);
        v(w, cx, cy, c);
        int n = Math.max(12, (int) (r * 1.5));
        for (int i = 0; i <= n; i++) {
            double a = i * Math.PI * 2 / n;
            v(w, cx + Math.cos(a) * r, cy + Math.sin(a) * r, c);
        }
        end();
    }

    /** A ring between radii r1 (inner colour) and r2 (outer colour). */
    public static void ring(double cx, double cy, double r1, double r2, int inner, int outer) {
        WorldRenderer w = begin(GL11.GL_TRIANGLE_STRIP);
        int n = Math.max(24, (int) (r2 * 1.2));
        for (int i = 0; i <= n; i++) {
            double a = i * Math.PI * 2 / n;
            double c = Math.cos(a), s = Math.sin(a);
            v(w, cx + c * r1, cy + s * r1, inner);
            v(w, cx + c * r2, cy + s * r2, outer);
        }
        end();
    }

    public static void roundRect(double x, double y, double wd, double ht, double r, int c) {
        r = Math.min(r, Math.min(wd, ht) / 2);
        rect(x + r, y, x + wd - r, y + ht, c);
        rect(x, y + r, x + r, y + ht - r, c);
        rect(x + wd - r, y + r, x + wd, y + ht - r, c);
        corner(x + r, y + r, r, Math.PI, c);
        corner(x + wd - r, y + r, r, Math.PI * 1.5, c);
        corner(x + wd - r, y + ht - r, r, 0, c);
        corner(x + r, y + ht - r, r, Math.PI * 0.5, c);
    }

    private static void corner(double cx, double cy, double r, double start, int c) {
        WorldRenderer w = begin(GL11.GL_TRIANGLE_FAN);
        v(w, cx, cy, c);
        int n = 8;
        for (int i = 0; i <= n; i++) {
            double a = start + i * (Math.PI / 2) / n;
            v(w, cx + Math.cos(a) * r, cy + Math.sin(a) * r, c);
        }
        end();
    }

    /** A rounded outline of width t. */
    public static void roundOutline(double x, double y, double wd, double ht, double r, double t, int c) {
        r = Math.min(r, Math.min(wd, ht) / 2);
        WorldRenderer w = begin(GL11.GL_TRIANGLE_STRIP);
        double[][] centers = {{x + wd - r, y + ht - r, 0}, {x + r, y + ht - r, Math.PI * 0.5}, {x + r, y + r, Math.PI}, {x + wd - r, y + r, Math.PI * 1.5}};
        int n = 8;
        for (int k = 0; k <= 4; k++) {
            double[] ce = centers[k % 4];
            for (int i = 0; i <= n; i++) {
                if (k == 4 && i > 0) break;
                double a = ce[2] + i * (Math.PI / 2) / n;
                double co = Math.cos(a), si = Math.sin(a);
                v(w, ce[0] + co * r, ce[1] + si * r, c);
                v(w, ce[0] + co * (r - t), ce[1] + si * (r - t), c);
            }
        }
        end();
    }

    /** A line of width t made of a quad (crisper and wider than glLineWidth allows). */
    public static void line(double x1, double y1, double x2, double y2, double t, int c1, int c2) {
        double dx = x2 - x1, dy = y2 - y1, len = Math.sqrt(dx * dx + dy * dy);
        if (len < 1e-6) return;
        double nx = -dy / len * t / 2, ny = dx / len * t / 2;
        quad(x1 + nx, y1 + ny, x2 + nx, y2 + ny, x2 - nx, y2 - ny, x1 - nx, y1 - ny, c1, c2, c2, c1);
    }

    public static void line(double x1, double y1, double x2, double y2, double t, int c) {
        line(x1, y1, x2, y2, t, c, c);
    }

    /** A thick polyline through xs/ys with joined segments. */
    public static void polyline(double[] xs, double[] ys, int n, double t, int c) {
        if (n < 2) return;
        WorldRenderer w = begin(GL11.GL_TRIANGLE_STRIP);
        for (int i = 0; i < n; i++) {
            int a = Math.max(0, i - 1), b = Math.min(n - 1, i + 1);
            double dx = xs[b] - xs[a], dy = ys[b] - ys[a], len = Math.max(1e-6, Math.sqrt(dx * dx + dy * dy));
            double nx = -dy / len * t / 2, ny = dx / len * t / 2;
            v(w, xs[i] + nx, ys[i] + ny, c);
            v(w, xs[i] - nx, ys[i] - ny, c);
        }
        end();
    }

    /** A four-point twinkle star of radius r (thin arms). */
    public static void star4(double cx, double cy, double r, double rot, int c) {
        WorldRenderer w = begin(GL11.GL_TRIANGLE_FAN);
        v(w, cx, cy, c);
        int edge = Colors.alpha(c, 0.25f);
        for (int i = 0; i <= 8; i++) {
            double a = rot + i * Math.PI / 4;
            double rr = (i % 2 == 0) ? r : r * 0.22;
            v(w, cx + Math.cos(a) * rr, cy + Math.sin(a) * rr, i % 2 == 0 ? edge : c);
        }
        end();
    }
}
