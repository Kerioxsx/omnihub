package dev.vesper.client.cosmetic;

import dev.vesper.client.render.Textures;
import java.nio.FloatBuffer;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.ResourceLocation;
import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;

/**
 * Batched camera-facing sprites that work under ANY current matrix (world
 * camera, GUI ortho with a y flip, the showroom's own perspective): the
 * camera's right/up/toward-viewer axes are read back from the modelview
 * matrix with {@link #camera()}, so the same cosmetic code billboards
 * correctly everywhere. Call {@link #camera()} after every matrix change
 * that the following sprites are drawn under.
 */
public final class Sprites {
    private static final FloatBuffer BUF = BufferUtils.createFloatBuffer(16);
    private static final float[] M = new float[16];
    /** Camera axes in the current local space (unit length). */
    public static float rx = 1, ry, rz, ux, uy = 1, uz, vx, vy, vz = 1;
    private static boolean open;
    private static WorldRenderer w;

    private Sprites() {
    }

    /** Reads the camera axes for the current modelview matrix. */
    public static void camera() {
        BUF.clear();
        GL11.glGetFloat(GL11.GL_MODELVIEW_MATRIX, BUF);
        BUF.get(M);
        float l = len(M[0], M[4], M[8]);
        rx = M[0] / l;
        ry = M[4] / l;
        rz = M[8] / l;
        l = len(M[1], M[5], M[9]);
        ux = M[1] / l;
        uy = M[5] / l;
        uz = M[9] / l;
        l = len(M[2], M[6], M[10]);
        vx = M[2] / l;
        vy = M[6] / l;
        vz = M[10] / l;
    }

    /** Eye-space depth of a local point (larger = nearer the viewer). Valid after {@link #camera()}. */
    public static float eyeZ(float x, float y, float z) {
        return M[2] * x + M[6] * y + M[10] * z + M[14];
    }

    /** Yaw (degrees) that turns local +z toward the viewer, for cylindrical billboards made of geometry. */
    public static float facingYaw() {
        return (float) Math.toDegrees(Math.atan2(vx, vz));
    }

    private static float len(float x, float y, float z) {
        float l = (float) Math.sqrt(x * x + y * y + z * z);
        return l < 1e-6f ? 1 : l;
    }

    public static void begin(ResourceLocation tex) {
        if (open) end();
        Minecraft.getMinecraft().getTextureManager().bindTexture(tex);
        GlStateManager.enableTexture2D();
        w = Tessellator.getInstance().getWorldRenderer();
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        open = true;
    }

    public static void beginGlow() {
        begin(Textures.glow());
    }

    public static void beginTwinkle() {
        begin(Textures.twinkle());
    }

    public static void end() {
        if (!open) return;
        Tessellator.getInstance().draw();
        open = false;
    }

    private static void v(double x, double y, double z, double u, double t, int c) {
        w.pos(x, y, z).tex(u, t).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    /** Spherical billboard of half-size hw × hh, rotated rot degrees in the screen plane. */
    public static void add(float x, float y, float z, float hw, float hh, float rot, int argb) {
        if ((argb >>> 24) == 0 || !open) return;
        float ax = rx, ay = ry, az = rz, bx = ux, by = uy, bz = uz;
        if (rot != 0) {
            float c = (float) Math.cos(Math.toRadians(rot)), s = (float) Math.sin(Math.toRadians(rot));
            ax = rx * c + ux * s;
            ay = ry * c + uy * s;
            az = rz * c + uz * s;
            bx = ux * c - rx * s;
            by = uy * c - ry * s;
            bz = uz * c - rz * s;
        }
        ax *= hw;
        ay *= hw;
        az *= hw;
        bx *= hh;
        by *= hh;
        bz *= hh;
        v(x - ax - bx, y - ay - by, z - az - bz, 0, 1, argb);
        v(x + ax - bx, y + ay - by, z + az - bz, 1, 1, argb);
        v(x + ax + bx, y + ay + by, z + az + bz, 1, 0, argb);
        v(x - ax + bx, y - ay + by, z - az + bz, 0, 0, argb);
    }

    public static void add(float x, float y, float z, float s, int argb) {
        add(x, y, z, s, s, 0, argb);
    }

    /** Cylindrical billboard: stays upright (local +y) and turns about it toward the viewer. */
    public static void addCyl(float x, float y, float z, float hw, float hh, int argb) {
        if ((argb >>> 24) == 0 || !open) return;
        float cx = vz, cz = -vx;
        float l = (float) Math.sqrt(cx * cx + cz * cz);
        if (l < 1e-4f) {
            cx = rx;
            cz = rz;
            l = Math.max(1e-4f, (float) Math.sqrt(cx * cx + cz * cz));
        }
        cx = cx / l * hw;
        cz = cz / l * hw;
        v(x - cx, y - hh, z - cz, 0, 1, argb);
        v(x + cx, y - hh, z + cz, 1, 1, argb);
        v(x + cx, y + hh, z + cz, 1, 0, argb);
        v(x - cx, y + hh, z - cz, 0, 0, argb);
    }

    /** A soft streak from a to b, `width` wide, facing the camera (glow texture stretched along it). */
    public static void line(float ax, float ay, float az, float bx, float by, float bz, float width, int argb) {
        if ((argb >>> 24) == 0 || !open) return;
        float dx = bx - ax, dy = by - ay, dz = bz - az;
        // perpendicular = dir × toViewer
        float px = dy * vz - dz * vy, py = dz * vx - dx * vz, pz = dx * vy - dy * vx;
        float l = (float) Math.sqrt(px * px + py * py + pz * pz);
        if (l < 1e-6f) {
            px = rx;
            py = ry;
            pz = rz;
            l = 1;
        }
        float h = width / 2 / l;
        px *= h;
        py *= h;
        pz *= h;
        // extend a little past the ends so the round falloff doesn't cut them short
        float ex = dx * 0.15f, ey = dy * 0.15f, ez = dz * 0.15f;
        v(ax - ex - px, ay - ey - py, az - ez - pz, 0, 1, argb);
        v(ax - ex + px, ay - ey + py, az - ez + pz, 1, 1, argb);
        v(bx + ex + px, by + ey + py, bz + ez + pz, 1, 0, argb);
        v(bx + ex - px, by + ey - py, bz + ez - pz, 0, 0, argb);
    }

    // ---- small deterministic helpers shared by the cosmetics ----

    /** Hash to 0..1. */
    public static float hash(int a, int b) {
        int h = a * 374761393 + b * 668265263;
        h = (h ^ (h >>> 13)) * 1274126177;
        h ^= h >>> 16;
        return (h & 0xFFFFFF) / (float) 0x1000000;
    }

    /** Smooth 1D value noise, -1..1. */
    public static float noise(float x) {
        int i = (int) Math.floor(x);
        float f = x - i;
        float u = f * f * (3 - 2 * f);
        return (hash(i, 17) * (1 - u) + hash(i + 1, 17) * u) * 2 - 1;
    }

    /** Segment count scaled by the quality preset (General.detail 0..3). */
    public static int seg(int base) {
        int d = dev.vesper.client.module.General.detail();
        float f = d == 0 ? 0.4f : d == 1 ? 0.65f : d == 2 ? 1f : 1.35f;
        return Math.max(4, Math.round(base * f));
    }
}
