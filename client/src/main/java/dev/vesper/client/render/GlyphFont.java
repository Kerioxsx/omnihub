package dev.vesper.client.render;

import dev.vesper.client.VesperClient;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.GraphicsEnvironment;
import java.awt.RenderingHints;
import java.awt.image.BufferedImage;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.ResourceLocation;
import org.lwjgl.opengl.GL11;

/**
 * Smooth, anti-aliased text from a system sans-serif font (nothing is
 * bundled). Each size is rasterised at the real screen pixel size, so text
 * stays sharp at any GUI scale and resolution. Falls back to Minecraft's
 * font if AWT can't render.
 */
public final class GlyphFont {
    private static final String CHARS;

    static {
        StringBuilder sb = new StringBuilder();
        for (char c = 32; c < 127; c++) sb.append(c);
        sb.append("·•–—…←→↑↓✓★☆✦✧◆◇○●×°§²³µ¼½¾ÀÁÂÄÇÈÉÊËÍÎÏÑÓÔÖÚÜßàáâäçèéêëíîïñóôöùúûüÿ☠⚔♥");
        CHARS = sb.toString();
    }

    private static final Map<String, GlyphFont> CACHE = new HashMap<String, GlyphFont>();
    private static String family;
    private static boolean broken;

    private final int px;
    private final int[] u = new int[65536], v = new int[65536], w = new int[65536];
    private final boolean[] has = new boolean[65536];
    private int lineH, ascent, texW, texH;
    private ResourceLocation tex;

    private GlyphFont(int px, boolean bold) {
        this.px = px;
        Font f = new Font(family(), bold ? Font.BOLD : Font.PLAIN, px);
        BufferedImage probe = new BufferedImage(1, 1, BufferedImage.TYPE_INT_ARGB);
        Graphics2D pg = probe.createGraphics();
        pg.setFont(f);
        FontMetrics fm = pg.getFontMetrics();
        lineH = fm.getHeight();
        ascent = fm.getAscent();
        pg.dispose();
        texW = 1024;
        int x = 1, y = 1;
        int pad = 2;
        for (int i = 0; i < CHARS.length(); i++) {
            int cw = Math.max(1, fm.charWidth(CHARS.charAt(i)));
            if (x + cw + pad * 2 >= texW) {
                x = 1;
                y += lineH + pad;
            }
            x += cw + pad * 2;
        }
        texH = Integer.highestOneBit(y + lineH + pad) * 2;
        BufferedImage img = new BufferedImage(texW, texH, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g = img.createGraphics();
        g.setFont(f);
        g.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
        g.setRenderingHint(RenderingHints.KEY_FRACTIONALMETRICS, RenderingHints.VALUE_FRACTIONALMETRICS_ON);
        g.setRenderingHint(RenderingHints.KEY_RENDERING, RenderingHints.VALUE_RENDER_QUALITY);
        g.setColor(java.awt.Color.WHITE);
        x = 1;
        y = 1;
        for (int i = 0; i < CHARS.length(); i++) {
            char c = CHARS.charAt(i);
            int cw = Math.max(1, fm.charWidth(c));
            if (x + cw + pad * 2 >= texW) {
                x = 1;
                y += lineH + pad;
            }
            if (f.canDisplay(c)) {
                g.drawString(String.valueOf(c), x + pad, y + ascent);
                u[c] = x;
                v[c] = y;
                w[c] = cw;
                has[c] = true;
            }
            x += cw + pad * 2;
        }
        g.dispose();
        DynamicTexture dt = new DynamicTexture(img);
        tex = Minecraft.getMinecraft().getTextureManager().getDynamicTextureLocation("vesper_font_" + px + (bold ? "b" : ""), dt);
        // Linear filtering: smooth edges when not drawn 1:1.
        GlStateManager.bindTexture(dt.getGlTextureId());
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MIN_FILTER, GL11.GL_LINEAR);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MAG_FILTER, GL11.GL_LINEAR);
    }

    private static String family() {
        if (family != null) return family;
        String[] wanted = {"Segoe UI", "Inter", "SF Pro Text", "Helvetica Neue", "Roboto", "Noto Sans", "DejaVu Sans", "Arial"};
        family = "SansSerif";
        try {
            String[] have = GraphicsEnvironment.getLocalGraphicsEnvironment().getAvailableFontFamilyNames();
            Arrays.sort(have);
            for (String n : wanted) {
                if (Arrays.binarySearch(have, n) >= 0) {
                    family = n;
                    break;
                }
            }
        } catch (Throwable t) {
            VesperClient.LOG.warn("Font list unavailable; using SansSerif");
        }
        return family;
    }

    private static int scale() {
        return new ScaledResolution(Minecraft.getMinecraft()).getScaleFactor();
    }

    /** The font for text `size` GUI units tall (cached per real pixel size). */
    public static GlyphFont of(float size, boolean bold) {
        if (broken) return null;
        int px = Math.max(6, Math.round(size * scale()));
        String key = px + (bold ? "b" : "");
        GlyphFont f = CACHE.get(key);
        if (f == null) {
            try {
                f = new GlyphFont(px, bold);
                CACHE.put(key, f);
            } catch (Throwable t) {
                broken = true;
                VesperClient.LOG.warn("Smooth font unavailable ({}); using Minecraft's font", t.toString());
                return null;
            }
        }
        return f;
    }

    /** Width in GUI units. */
    public static float width(String s, float size, boolean bold) {
        GlyphFont f = of(size, bold);
        if (f == null) return Minecraft.getMinecraft().fontRendererObj.getStringWidth(s) * size / 9f;
        int tw = 0;
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            tw += f.has[c] ? f.w[c] : f.w['?'];
        }
        return tw / (float) scale();
    }

    /** Draws text with its top-left at x,y (GUI units). Returns the width. */
    public static float draw(String s, float x, float y, float size, boolean bold, int argb) {
        GlyphFont f = of(size, bold);
        if (f == null) {
            GlStateManager.pushMatrix();
            GlStateManager.translate(x, y, 0);
            GlStateManager.scale(size / 9f, size / 9f, 1);
            Minecraft.getMinecraft().fontRendererObj.drawString(s, 0, 0, argb);
            GlStateManager.popMatrix();
            return width(s, size, bold);
        }
        return f.render(s, x, y, argb);
    }

    public static float drawCentered(String s, float cx, float y, float size, boolean bold, int argb) {
        return draw(s, cx - width(s, size, bold) / 2, y, size, bold, argb);
    }

    private float render(String s, float x, float y, int argb) {
        float k = 1f / scale();
        GlStateManager.enableTexture2D();
        GlStateManager.enableBlend();
        GlStateManager.tryBlendFuncSeparate(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA, 1, 0);
        GlStateManager.disableAlpha();
        Minecraft.getMinecraft().getTextureManager().bindTexture(tex);
        GlStateManager.color(1, 1, 1, 1);
        Tessellator t = Tessellator.getInstance();
        WorldRenderer wr = t.getWorldRenderer();
        wr.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        int r = (argb >> 16) & 255, g = (argb >> 8) & 255, b = argb & 255, a = argb >>> 24;
        // Snap to whole screen pixels so 1:1 glyphs stay crisp.
        float cx = Math.round(x / k) * k, top = Math.round(y / k) * k;
        int pad = 2;
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (!has[c]) c = '?';
            float gw = (w[c] + pad * 2) * k, gh = lineH * k;
            float x0 = cx - pad * k;
            float u0 = u[c] / (float) texW, v0 = v[c] / (float) texH, u1 = (u[c] + w[c] + pad * 2) / (float) texW, v1 = (v[c] + lineH) / (float) texH;
            wr.pos(x0, top, 0).tex(u0, v0).color(r, g, b, a).endVertex();
            wr.pos(x0, top + gh, 0).tex(u0, v1).color(r, g, b, a).endVertex();
            wr.pos(x0 + gw, top + gh, 0).tex(u1, v1).color(r, g, b, a).endVertex();
            wr.pos(x0 + gw, top, 0).tex(u1, v0).color(r, g, b, a).endVertex();
            cx += w[c] * k;
        }
        t.draw();
        GlStateManager.enableAlpha();
        return cx - x;
    }

    /** Height of a line in GUI units. */
    public static float lineHeight(float size) {
        GlyphFont f = of(size, false);
        return f == null ? size : f.lineH / (float) scale();
    }

    /** Drop the cache (after a resolution / GUI-scale change frees textures lazily). */
    public static void clear() {
        for (GlyphFont f : CACHE.values()) Minecraft.getMinecraft().getTextureManager().deleteTexture(f.tex);
        CACHE.clear();
    }
}
