package dev.vesper.client.fx;

import dev.vesper.client.render.GlyphFont;
import java.nio.FloatBuffer;
import net.minecraft.client.renderer.GlStateManager;
import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;

/**
 * Comic-style text standing in the world, facing the camera (kill effects,
 * world lyrics). Call inside a glow block after {@link View#capture()}; it
 * flushes the current batch and leaves additive blending, texture off.
 */
public final class FxText {
    private static final FloatBuffer M = BufferUtils.createFloatBuffer(16);
    /** Font size the glyphs are rasterised at (GUI units); scaled to world size. */
    private static final float PX = 18f;

    private FxText() {
    }

    /** Sets the matrix so +x is camera right and +y is camera down at (x,y,z), `height` blocks per line. */
    public static void push(double x, double y, double z, double height, float rotDeg) {
        FxDraw.flush();
        GlStateManager.pushMatrix();
        GlStateManager.translate(x, y, z);
        double fx = View.ry * View.uz - View.rz * View.uy, fy = View.rz * View.ux - View.rx * View.uz, fz = View.rx * View.uy - View.ry * View.ux;
        M.clear();
        M.put((float) View.rx).put((float) View.ry).put((float) View.rz).put(0);
        M.put((float) View.ux).put((float) View.uy).put((float) View.uz).put(0);
        M.put((float) fx).put((float) fy).put((float) fz).put(0);
        M.put(0).put(0).put(0).put(1);
        M.flip();
        GL11.glMultMatrix(M);
        if (rotDeg != 0) GlStateManager.rotate(rotDeg, 0, 0, 1);
        float k = (float) (height / PX);
        GlStateManager.scale(k, -k, k);
    }

    public static void pop() {
        GlStateManager.disableTexture2D();
        GlStateManager.disableAlpha();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        GlStateManager.popMatrix();
    }

    /** Width of `s` in the units {@link #push} sets up (multiply by height/18 for blocks). */
    public static float width(String s) {
        return GlyphFont.width(s, PX, true);
    }

    /** Draws centred text at the pushed origin with an outline (0 alpha = none). */
    public static void draw(String s, float dx, float dy, int fill, int outline) {
        float w = GlyphFont.width(s, PX, true);
        float x = dx - w / 2, y = dy - PX / 2;
        if ((outline >>> 24) != 0) {
            float o = 1.6f;
            for (int i = 0; i < 8; i++) {
                double a = i * Math.PI / 4;
                GlyphFont.draw(s, x + (float) Math.cos(a) * o, y + (float) Math.sin(a) * o, PX, true, outline);
            }
        }
        GlyphFont.draw(s, x, y, PX, true, fill);
    }

    /** One-call text: pushes, draws with normal blending, pops. */
    public static void text(String s, double x, double y, double z, double height, float rotDeg, int fill, int outline) {
        if (((fill | outline) >>> 24) == 0 || height <= 0) return;
        push(x, y, z, height, rotDeg);
        draw(s, 0, 0, fill, outline);
        pop();
    }
}
