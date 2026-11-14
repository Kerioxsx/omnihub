package dev.vesper.client.visual;

import java.nio.ByteBuffer;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL12;

/**
 * A copy of what is on screen so far this frame, in a texture, drawn back
 * over the screen (motion blur, menu blur). One GPU copy per use; nothing
 * leaves the graphics card.
 */
final class FrameGrab {
    private int tex = -1, w, h;
    private boolean has;

    /** Copies the frame drawn so far (the game's framebuffer is bound while it draws). */
    void copy() {
        Minecraft mc = Minecraft.getMinecraft();
        int dw = mc.displayWidth, dh = mc.displayHeight;
        if (dw <= 0 || dh <= 0) return;
        if (tex < 0) tex = GL11.glGenTextures();
        GlStateManager.bindTexture(tex);
        if (dw != w || dh != h) {
            w = dw;
            h = dh;
            GL11.glTexImage2D(GL11.GL_TEXTURE_2D, 0, GL11.GL_RGB8, w, h, 0, GL11.GL_RGB, GL11.GL_UNSIGNED_BYTE, (ByteBuffer) null);
            GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MIN_FILTER, GL11.GL_LINEAR);
            GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MAG_FILTER, GL11.GL_LINEAR);
            GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_WRAP_S, GL12.GL_CLAMP_TO_EDGE);
            GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_WRAP_T, GL12.GL_CLAMP_TO_EDGE);
            has = false;
        }
        GL11.glCopyTexSubImage2D(GL11.GL_TEXTURE_2D, 0, 0, 0, 0, 0, w, h);
        has = true;
    }

    /** A copy of this window's size exists. */
    boolean ready() {
        Minecraft mc = Minecraft.getMinecraft();
        return has && tex >= 0 && w == mc.displayWidth && h == mc.displayHeight;
    }

    void forget() {
        has = false;
    }

    /** Begins drawing copies (GUI coordinates, normal blending). */
    void begin() {
        GlStateManager.bindTexture(tex);
        GlStateManager.enableTexture2D();
        GlStateManager.enableBlend();
        GlStateManager.tryBlendFuncSeparate(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA, 1, 0);
        GlStateManager.disableAlpha();
        GlStateManager.disableDepth();
        GlStateManager.depthMask(false);
        GlStateManager.disableLighting();
        GlStateManager.disableCull();
    }

    /** The copy over the whole screen, shifted by (dx, dy) GUI units. */
    void draw(float alpha, double dx, double dy) {
        ScaledResolution sr = new ScaledResolution(Minecraft.getMinecraft());
        double sw = sr.getScaledWidth_double(), sh = sr.getScaledHeight_double();
        GlStateManager.color(1, 1, 1, alpha);
        Tessellator t = Tessellator.getInstance();
        WorldRenderer wr = t.getWorldRenderer();
        wr.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX);
        wr.pos(dx, sh + dy, 0).tex(0, 0).endVertex();
        wr.pos(sw + dx, sh + dy, 0).tex(1, 0).endVertex();
        wr.pos(sw + dx, dy, 0).tex(1, 1).endVertex();
        wr.pos(dx, dy, 0).tex(0, 1).endVertex();
        t.draw();
    }

    /** The copy scaled about the screen centre (zoom blur). */
    void drawScaled(float alpha, double scale) {
        ScaledResolution sr = new ScaledResolution(Minecraft.getMinecraft());
        double sw = sr.getScaledWidth_double(), sh = sr.getScaledHeight_double();
        double dx = sw * (scale - 1) / 2, dy = sh * (scale - 1) / 2;
        GlStateManager.color(1, 1, 1, alpha);
        Tessellator t = Tessellator.getInstance();
        WorldRenderer wr = t.getWorldRenderer();
        wr.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX);
        wr.pos(-dx, sh + dy, 0).tex(0, 0).endVertex();
        wr.pos(sw + dx, sh + dy, 0).tex(1, 0).endVertex();
        wr.pos(sw + dx, -dy, 0).tex(1, 1).endVertex();
        wr.pos(-dx, -dy, 0).tex(0, 1).endVertex();
        t.draw();
    }

    void end() {
        GlStateManager.color(1, 1, 1, 1);
        GlStateManager.enableDepth();
        GlStateManager.depthMask(true);
        GlStateManager.enableAlpha();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
    }
}
