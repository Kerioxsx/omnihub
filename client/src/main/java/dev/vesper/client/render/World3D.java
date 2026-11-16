package dev.vesper.client.render;

import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.ResourceLocation;
import org.lwjgl.opengl.GL11;

/**
 * World-space drawing helpers shared by effects, cosmetics and the
 * showroom. Positions are relative to the current matrix (callers translate
 * to world positions minus the camera first, see {@link #toWorld}).
 */
public final class World3D {
    private World3D() {
    }

    /** Translates the matrix so (x,y,z) in world coordinates is the origin. */
    public static void toWorld(double x, double y, double z) {
        RenderManager rm = Minecraft.getMinecraft().getRenderManager();
        GlStateManager.translate(x - rm.viewerPosX, y - rm.viewerPosY, z - rm.viewerPosZ);
    }

    /** Camera-facing rotation (call after translating to the sprite's position). */
    public static void faceCamera() {
        RenderManager rm = Minecraft.getMinecraft().getRenderManager();
        GlStateManager.rotate(-rm.playerViewY, 0, 1, 0);
        GlStateManager.rotate(rm.playerViewX, 1, 0, 0);
    }

    /** A camera-facing textured sprite of half-size s at (x,y,z) relative to the matrix. */
    public static void sprite(ResourceLocation tex, double x, double y, double z, double s, double rot, int argb) {
        Minecraft.getMinecraft().getTextureManager().bindTexture(tex);
        GlStateManager.enableTexture2D();
        GlStateManager.pushMatrix();
        GlStateManager.translate(x, y, z);
        faceCamera();
        if (rot != 0) GlStateManager.rotate((float) rot, 0, 0, 1);
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        int r = (argb >> 16) & 255, g = (argb >> 8) & 255, b = argb & 255, a = argb >>> 24;
        w.pos(-s, -s, 0).tex(0, 1).color(r, g, b, a).endVertex();
        w.pos(s, -s, 0).tex(1, 1).color(r, g, b, a).endVertex();
        w.pos(s, s, 0).tex(1, 0).color(r, g, b, a).endVertex();
        w.pos(-s, s, 0).tex(0, 0).color(r, g, b, a).endVertex();
        t.draw();
        GlStateManager.popMatrix();
    }

    /** Core + halo pass (the spec's bloom substitute). */
    public static void glowDot(double x, double y, double z, double s, int argb) {
        sprite(Textures.glow(), x, y, z, s * 2.8, 0, Colors.alpha(argb, 0.3f));
        sprite(Textures.glow(), x, y, z, s, 0, argb);
    }

    /** A flat horizontal ring (y up) between radii r1 and r2, untextured. */
    public static void flatRing(double y, double r1, double r2, int inner, int outer, int segments) {
        GlStateManager.disableTexture2D();
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
        for (int i = 0; i <= segments; i++) {
            double a = i * Math.PI * 2 / segments, c = Math.cos(a), s = Math.sin(a);
            col(w.pos(c * r1, y, s * r1), inner);
            col(w.pos(c * r2, y, s * r2), outer);
        }
        t.draw();
        GlStateManager.enableTexture2D();
    }

    /** A ribbon through points (x,y,z triples) of width `wd`, colour fading from c0 at the head to c1 at the tail; it faces up (y) by default. */
    public static void ribbon(double[] pts, int n, double wd, int c0, int c1) {
        if (n < 2) return;
        GlStateManager.disableTexture2D();
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
        for (int i = 0; i < n; i++) {
            int a = Math.max(0, i - 1) * 3, b = Math.min(n - 1, i + 1) * 3;
            double dx = pts[b] - pts[a], dz = pts[b + 2] - pts[a + 2];
            double len = Math.max(1e-6, Math.sqrt(dx * dx + dz * dz));
            double hw = wd / 2 * (1 - i / (double) n);
            double nx = -dz / len * hw, nz = dx / len * hw;
            int c = Colors.mix(c0, c1, i / (float) (n - 1));
            col(w.pos(pts[i * 3] + nx, pts[i * 3 + 1], pts[i * 3 + 2] + nz), c);
            col(w.pos(pts[i * 3] - nx, pts[i * 3 + 1], pts[i * 3 + 2] - nz), c);
        }
        t.draw();
        GlStateManager.enableTexture2D();
    }

    static void col(WorldRenderer w, int c) {
        w.color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    /** True while the local player is seen in first person. */
    public static boolean firstPerson() {
        return Minecraft.getMinecraft().gameSettings.thirdPersonView == 0;
    }
}
