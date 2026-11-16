package dev.vesper.client.hud;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.GlyphFont;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.RenderHelper;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.item.ItemStack;
import net.minecraft.util.ResourceLocation;
import org.lwjgl.opengl.GL11;

/**
 * One draggable HUD widget. Each element is a {@link Module} in
 * {@link Category#HUD} (toggle, search and persistence come with that) and is
 * placed through {@link HudLayout} (fractions of the free screen space, plus
 * a scale).
 *
 * <p>Elements draw in their own unscaled units through the helpers below,
 * which apply the origin and scale by multiplying coordinates (not with a GL
 * scale), so smooth text is rasterised at its real pixel size and stays
 * sharp at every scale.
 */
public abstract class HudElement extends Module {
    /** Panel fill and outline shared by the boxed elements (VISUAL_SPEC §9 neutral UI). */
    public static final int PANEL = 0xC70E0F13;
    public static final int PANEL_EDGE = 0x10FFFFFF;
    public static final int TEXT = 0xFFE4E5EA;
    public static final int TEXT_MUTED = 0xFF9A9DA6;
    public static final int GOOD = 0xFF7CE38B;
    public static final int WARN = 0xFFFFD166;
    public static final int BAD = 0xFFFF6B6B;
    public static final int ACCENT = 0xFF38C6F4;

    protected static final Minecraft mc = Minecraft.getMinecraft();

    /** Unscaled size of the current content; set by {@link #refresh(boolean)}. */
    protected float w = 40, h = 12;

    private float ox, oy, s = 1, alpha = 1;

    protected HudElement(String id, String name, String description, boolean enabledByDefault, boolean publicSafe, float x, float y, float scale) {
        super(id, name, description, Category.HUD, enabledByDefault, publicSafe);
        HudLayout.define(id, x, y, scale, true);
    }

    /**
     * Recomputes what to show (and {@link #w}/{@link #h}). Called every frame
     * before drawing; elements throttle their own string building.
     *
     * @param sample true in the HUD editor when there is no real data: show sample content
     */
    public abstract void refresh(boolean sample);

    /** True when there is real data to show right now (after {@link #refresh(boolean)} with false). */
    public abstract boolean hasContent();

    /** Draws the content with its top-left at 0,0 in unscaled units. */
    protected abstract void paint(boolean sample);

    public final float width() {
        return w;
    }

    public final float height() {
        return h;
    }

    /** Sets the scale before {@link #refresh(boolean)} so text measurement matches the drawn size. */
    final void prepare(float scale) {
        s = scale;
    }

    /** Called inside Gl.begin2D/end2D. */
    public final void renderAt(float x, float y, float scale, boolean sample, float alphaMul) {
        ox = x;
        oy = y;
        s = scale;
        alpha = alphaMul;
        paint(sample);
    }

    // ---- drawing helpers (unscaled local coordinates) ----

    protected final float scale() {
        return s;
    }

    protected final int fade(int argb) {
        return alpha >= 1f ? argb : Colors.alpha(argb, alpha);
    }

    protected final void rect(float x, float y, float wd, float ht, int c) {
        Draw.rect(ox + x * s, oy + y * s, ox + (x + wd) * s, oy + (y + ht) * s, fade(c));
    }

    protected final void round(float x, float y, float wd, float ht, float r, int c) {
        Draw.roundRect(ox + x * s, oy + y * s, wd * s, ht * s, r * s, fade(c));
    }

    protected final void outline(float x, float y, float wd, float ht, float r, float t, int c) {
        Draw.roundOutline(ox + x * s, oy + y * s, wd * s, ht * s, r * s, Math.max(0.5f, t * s), fade(c));
    }

    protected final void line(float x1, float y1, float x2, float y2, float t, int c) {
        Draw.line(ox + x1 * s, oy + y1 * s, ox + x2 * s, oy + y2 * s, t * s, fade(c));
    }

    protected final void ring(float cx, float cy, float r, float t, int c) {
        int f = fade(c);
        Draw.ring(ox + cx * s, oy + cy * s, (r - t / 2) * s, (r + t / 2) * s, f, f);
    }

    protected final void circle(float cx, float cy, float r, int c) {
        Draw.circle(ox + cx * s, oy + cy * s, r * s, fade(c));
    }

    /** A filled triangle. */
    protected final void tri(float ax, float ay, float bx, float by, float cx, float cy, int c) {
        int f = fade(c);
        Draw.quad(ox + ax * s, oy + ay * s, ox + bx * s, oy + by * s, ox + cx * s, oy + cy * s, ox + cx * s, oy + cy * s, f, f, f, f);
    }

    /** Polyline through buffers already in local units (they are transformed in place into scratch arrays). */
    private final double[] px = new double[48], py = new double[48];

    protected final void polyline(double[] xs, double[] ys, int n, float t, int c) {
        n = Math.min(n, px.length);
        for (int i = 0; i < n; i++) {
            px[i] = ox + xs[i] * s;
            py[i] = oy + ys[i] * s;
        }
        Draw.polyline(px, py, n, t * s, fade(c));
    }

    /** Smooth text with its top-left at x,y. Returns the unscaled width. */
    protected final float text(String str, float x, float y, float size, boolean bold, int c) {
        float wd = GlyphFont.draw(str, ox + x * s, oy + y * s, size * s, bold, fade(c));
        restore2D();
        return wd / s;
    }

    /** Text vertically centred on cy. */
    protected final float textMid(String str, float x, float cy, float size, boolean bold, int c) {
        return text(str, x, cy - lineH(size) / 2f, size, bold, c);
    }

    /** Unscaled text width (measured at the drawn pixel size, so layout matches). */
    protected final float textW(String str, float size, boolean bold) {
        return GlyphFont.width(str, size * s, bold) / s;
    }

    protected final float lineH(float size) {
        return GlyphFont.lineHeight(size * s) / s;
    }

    /** A 16×16 item icon (and its damage bar) at x,y, scaled with the element. */
    protected final void item(ItemStack stack, float x, float y) {
        if (stack == null) return;
        GlStateManager.pushMatrix();
        GlStateManager.translate(ox + x * s, oy + y * s, 0);
        GlStateManager.scale(s, s, 1);
        GlStateManager.enableTexture2D();
        GlStateManager.enableRescaleNormal();
        GlStateManager.enableDepth();
        RenderHelper.enableGUIStandardItemLighting();
        try {
            mc.getRenderItem().renderItemAndEffectIntoGUI(stack, 0, 0);
        } catch (RuntimeException ignored) {
            // A broken item model must not take the HUD down.
        }
        RenderHelper.disableStandardItemLighting();
        GlStateManager.disableRescaleNormal();
        GlStateManager.disableDepth();
        GlStateManager.popMatrix();
        restore2D();
    }

    /** A textured quad from `tex` (UVs in 0..1), tinted by c. */
    protected final void sprite(ResourceLocation tex, float x, float y, float wd, float ht, float u0, float v0, float u1, float v1, int c) {
        GlStateManager.enableTexture2D();
        mc.getTextureManager().bindTexture(tex);
        GlStateManager.color(1, 1, 1, 1);
        int f = fade(c);
        int r = (f >> 16) & 255, g = (f >> 8) & 255, b = f & 255, a = f >>> 24;
        double x0 = ox + x * s, y0 = oy + y * s, x1 = ox + (x + wd) * s, y1 = oy + (y + ht) * s;
        Tessellator t = Tessellator.getInstance();
        WorldRenderer wr = t.getWorldRenderer();
        wr.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        wr.pos(x0, y0, 0).tex(u0, v0).color(r, g, b, a).endVertex();
        wr.pos(x0, y1, 0).tex(u0, v1).color(r, g, b, a).endVertex();
        wr.pos(x1, y1, 0).tex(u1, v1).color(r, g, b, a).endVertex();
        wr.pos(x1, y0, 0).tex(u1, v0).color(r, g, b, a).endVertex();
        t.draw();
        restore2D();
    }

    /** Back to the Gl.begin2D state after textured drawing. */
    protected static void restore2D() {
        GlStateManager.disableTexture2D();
        GlStateManager.disableLighting();
        GlStateManager.disableAlpha();
        GlStateManager.enableBlend();
        GlStateManager.tryBlendFuncSeparate(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA, 1, 0);
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        GlStateManager.color(1, 1, 1, 1);
    }

    /** The standard rounded panel behind boxed elements. */
    protected final void panel(float x, float y, float wd, float ht) {
        round(x, y, wd, ht, Math.min(3f, ht * 0.25f), PANEL);
        outline(x, y, wd, ht, Math.min(3f, ht * 0.25f), 0.5f, PANEL_EDGE);
    }

    /** Refresh throttle: true at most every `ms` milliseconds (or when sample mode flips). */
    private long nextRefresh;
    private boolean lastSample;

    protected final boolean due(long ms, boolean sample) {
        long now = System.currentTimeMillis();
        if (now < nextRefresh && sample == lastSample) return false;
        nextRefresh = now + ms;
        lastSample = sample;
        return true;
    }
}
