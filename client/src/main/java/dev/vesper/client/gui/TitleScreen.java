package dev.vesper.client.gui;

import dev.vesper.client.VesperClient;
import dev.vesper.client.module.General;
import dev.vesper.client.preview.PlayerPreview;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import dev.vesper.client.render.Textures;
import net.minecraft.client.gui.GuiMultiplayer;
import net.minecraft.client.gui.GuiOptions;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.gui.GuiSelectWorld;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.input.Keyboard;
import org.lwjgl.opengl.GL11;

/**
 * Vesper's title: a "Press to enter" splash (once per launch), then the menu
 * (VISUAL_SPEC §1–2). One screen for both, so the logo moves continuously
 * from the splash into the menu header. Every item opens the real
 * Minecraft screen, which returns here with Done/Back.
 */
public class TitleScreen extends GuiScreen {
    private static boolean splashDone;

    private static final String[] ITEMS = {"Singleplayer", "Multiplayer", "Minecraft Settings", "Client", "Quit Game"};

    private boolean splash;
    /** Seconds when the splash opened / the exit started / this screen opened / we started leaving. */
    private float splashAt, exitAt = -1, openedAt, leaveAt = -1;
    private int leaveTo = -1;
    private int focus = -1;
    private final float[] hover = new float[ITEMS.length];
    private float lastFrame;
    private float previewYaw;
    private int lastMx = -1, lastMy = -1;

    public static boolean enabled() {
        return General.INSTANCE == null || General.INSTANCE.customTitle.on();
    }

    @Override
    public void initGui() {
        float now = Ease.seconds();
        openedAt = now;
        lastFrame = now;
        leaveAt = -1;
        leaveTo = -1;
        boolean wantSplash = General.INSTANCE == null || General.INSTANCE.splash.on();
        if (!splashDone && wantSplash) {
            splash = true;
            splashAt = now;
        } else {
            splash = false;
            splashDone = true;
            exitAt = -1;
        }
        Keyboard.enableRepeatEvents(true);
    }

    @Override
    public void onGuiClosed() {
        Keyboard.enableRepeatEvents(false);
    }

    @Override
    public boolean doesGuiPauseGame() {
        return false;
    }

    // ---------- layout (GUI units; everything scales with the height) ----------

    private float s() {
        return General.INSTANCE == null ? 1f : General.INSTANCE.uiScale.f();
    }

    private float colX() {
        return 0.085f * width;
    }

    private float itemY(int i) {
        float y = 0.40f * height + i * 0.056f * height * s();
        if (i == ITEMS.length - 1) y += 0.02f * height;
        return y;
    }

    private float itemSize() {
        return Math.max(7f, 0.019f * height / 0.7f * s());
    }

    private boolean over(int i, int mx, int my) {
        float y = itemY(i), sz = itemSize();
        float w = GlyphFont.width(ITEMS[i], sz, false) + 0.04f * height;
        return mx >= colX() - 0.02f * height && mx <= colX() + w && my >= y - 0.012f * height && my <= y + sz * 1.05f + 0.012f * height;
    }

    // ---------- input ----------

    private boolean inMenu() {
        return !splash && exitAt < 0 && leaveAt < 0;
    }

    private void enter() {
        if (!splash) return;
        splash = false;
        splashDone = true;
        exitAt = Ease.seconds();
        UiSound.press();
    }

    @Override
    protected void keyTyped(char c, int key) {
        if (splash) {
            enter();
            return;
        }
        if (!inMenu()) return;
        if (key == Keyboard.KEY_DOWN || key == Keyboard.KEY_TAB && !isShiftKeyDown()) setFocus(focus < 0 ? 0 : (focus + 1) % ITEMS.length);
        else if (key == Keyboard.KEY_UP || key == Keyboard.KEY_TAB) setFocus(focus <= 0 ? ITEMS.length - 1 : focus - 1);
        else if ((key == Keyboard.KEY_RETURN || key == Keyboard.KEY_NUMPADENTER || key == Keyboard.KEY_SPACE) && focus >= 0) activate(focus);
        // Esc on the title does nothing (like vanilla): there's nothing to go back to.
    }

    private void setFocus(int f) {
        if (f != focus && f >= 0) UiSound.hover();
        focus = f;
    }

    @Override
    protected void mouseClicked(int mx, int my, int button) {
        if (splash) {
            enter();
            return;
        }
        if (!inMenu() || button != 0) return;
        for (int i = 0; i < ITEMS.length; i++) if (over(i, mx, my)) activate(i);
    }

    private void activate(int i) {
        UiSound.press();
        leaveTo = i;
        leaveAt = General.calm() ? Ease.seconds() - 1 : Ease.seconds();
    }

    private void open(int i) {
        switch (i) {
            case 0:
                mc.displayGuiScreen(new GuiSelectWorld(this));
                break;
            case 1:
                mc.displayGuiScreen(new GuiMultiplayer(this));
                break;
            case 2:
                mc.displayGuiScreen(new GuiOptions(this, mc.gameSettings));
                break;
            case 3:
                mc.displayGuiScreen(new ClientScreen(this));
                break;
            default:
                VesperClient.config().save();
                mc.shutdown();
        }
    }

    // ---------- drawing ----------

    @Override
    public void drawScreen(int mx, int my, float pt) {
        float now = Ease.seconds();
        float dt = Math.min(0.1f, now - lastFrame);
        lastFrame = now;
        boolean calm = General.calm();

        // The mouse takes focus only when it moves (so keyboard focus isn't stolen by a resting cursor).
        boolean moved = mx != lastMx || my != lastMy;
        lastMx = mx;
        lastMy = my;
        if (inMenu() && moved) {
            int f = -1;
            for (int i = 0; i < ITEMS.length; i++) if (over(i, mx, my)) f = i;
            if (f >= 0) setFocus(f);
            else if (focus >= 0 && !Keyboard.isKeyDown(Keyboard.KEY_TAB)) focus = -1;
        }
        for (int i = 0; i < ITEMS.length; i++) hover[i] = Ease.approach(hover[i], inMenu() && focus == i ? 1 : 0, calm ? 40 : 16, dt);

        float leave = leaveAt < 0 ? 0 : Ease.span(now, leaveAt, leaveAt + 0.22f);
        if (leaveAt >= 0 && leave >= 1) {
            int to = leaveTo;
            leaveAt = -1;
            leaveTo = -1;
            open(to);
            return;
        }

        Gl.begin2D();
        float bgA = splash ? Ease.span(now, splashAt, splashAt + 0.4f) : 1f;
        Draw.rect(0, 0, width, height, 0xFF000000);
        MenuBackdrop.base(width, height, bgA);

        float ex = splash ? 0 : exitAt < 0 ? 1 : Ease.span(now, exitAt, exitAt + (calm ? 0.25f : 0.8f));
        boolean exiting = exitAt >= 0;
        float menuIn = splash ? 0 : exiting ? Ease.span(ex, 0.25f, 1f) : Ease.span(now, openedAt, openedAt + 0.6f);
        float fade = 1 - leave;

        if (splash || exiting) {
            MenuBackdrop.sheen(width, height, now, (1 - ex) * bgA);
            MenuBackdrop.motes(width, height, now, (1 - ex) * bgA);
        }
        MenuBackdrop.curtains(width, height, now, menuIn * fade);
        Gl.end2D();

        if (menuIn > 0.01f && (General.INSTANCE == null || General.INSTANCE.playerPreview.on())) {
            float target = Math.max(-25, Math.min(25, (mx - 0.72f * width) / (0.3f * width) * 25));
            previewYaw = Ease.approach(previewYaw, target, 4f, dt);
            float p0 = exiting ? exitAt + 0.5f : openedAt + 0.3f;
            float pin = Ease.outCubic(Ease.span(now, p0, p0 + 0.5f));
            Gl.begin2D();
            Draw.glow(0.72f * width, 0.88f * height, 0.09f * height, Colors.withAlpha(0x000000, 0.35f * pin * fade));
            Gl.end2D();
            try {
                PlayerPreview.draw(0.72f * width, 0.88f * height + (1 - pin) * 0.03f * height, 0.52f * height, previewYaw, pin * fade, now);
            } catch (Throwable err) {
                VesperClient.LOG.warn("Player preview failed, turning it off: {}", err.toString());
                if (General.INSTANCE != null) General.INSTANCE.playerPreview.set(false);
            }
        }

        Gl.begin2D();
        MenuBackdrop.stars(width, height, now, menuIn * fade);

        // Logo: one transform from the splash centre to the menu header.
        float big = 0.16f * height, small = 0.045f * height * s();
        float lx0 = width / 2f - big / 2, ly0 = 0.40f * height - big / 2;
        float lx1 = colX(), ly1 = 0.30f * height - small / 2;
        float mv = Ease.inOutCubic(Ease.span(ex, 0, 0.875f));
        float ls = Ease.lerp(big, small, mv), lx = Ease.lerp(lx0, lx1, mv), ly = Ease.lerp(ly0, ly1, mv);
        float logoIn = splash ? Ease.span(now, splashAt + 0.15f, splashAt + 0.75f) : 1f;
        float pop = splash && !calm ? Ease.lerp(0.86f, 1f, Ease.outBack(logoIn)) : 1f;
        float drawn = calm || !splash ? 1f : Ease.span(now, splashAt + 0.15f, splashAt + 0.15f + 0.04f * Sigil.STROKES + 0.2f);
        float lsz = ls * pop;
        Sigil.draw(lx + (ls - lsz) / 2, ly + (ls - lsz) / 2, lsz, Colors.withAlpha(0xF2F3F5, Math.min(1, logoIn * 1.2f) * fade), drawn);

        if (splash && !calm) {
            float sp = Ease.span(now, splashAt + 0.65f, splashAt + 0.9f);
            if (sp > 0 && sp < 1) sparkle(Sigil.tipX(lx, ls), Sigil.tipY(ly, ls), 0.09f * height * Ease.bell(sp), sp * 45f);
        }

        if (splash || exiting) {
            float nameA = splash ? Ease.outCubic(Ease.span(now, splashAt + 0.5f, splashAt + 0.95f)) : 1 - Ease.span(ex, 0, 0.44f);
            float ns = 0.032f * height / 0.7f;
            float nameY = 0.55f * height - ns * 0.8f + (splash ? (1 - nameA) * 0.012f * height : 0);
            GlyphFont.drawCentered("VESPER", width / 2f, nameY, ns, true, Colors.withAlpha(0xF2F3F5, nameA));
            float pin = splash ? Ease.span(now, splashAt + 0.9f, splashAt + 1.3f) : 1 - Ease.span(ex, 0, 0.31f);
            float pulse = splash && pin >= 1 && !calm ? 0.45f + 0.5f * (0.5f + 0.5f * (float) Math.sin((now - splashAt) * 6.283f / 1.8f)) : 0.95f;
            GlyphFont.drawCentered("Press to enter", width / 2f, 0.615f * height, Math.max(6f, 0.018f * height / 0.7f), false, Colors.withAlpha(0xC9CCD3, pin * pulse));
        }

        if (menuIn > 0) {
            float nameIn = Ease.outCubic(menuIn) * fade;
            float hs = 0.024f * height / 0.7f * s();
            GlyphFont.draw("Vesper Client", lx1 + small + 0.012f * height, 0.30f * height - hs * 0.62f, hs, true, Colors.withAlpha(0xF2F3F5, nameIn));
            for (int i = 0; i < ITEMS.length; i++) {
                float start = exiting ? 0.4375f + i * 0.0625f : i * 0.08f;
                float a = calm ? menuIn : exiting ? Ease.outCubic(Ease.span(ex, start, start + 0.375f)) : Ease.outCubic(Ease.span(now, openedAt + start, openedAt + start + 0.3f));
                a *= fade;
                float h = Ease.outCubic(hover[i]);
                float x = colX() - (1 - a) * 0.02f * height + h * 0.008f * height;
                float y = itemY(i), sz = itemSize();
                if (h > 0.01f) {
                    float tw = GlyphFont.width(ITEMS[i], sz, false);
                    Draw.roundRect(x - 0.012f * height, y - 0.006f * height, tw + 0.024f * height, sz * 1.05f + 0.012f * height, 0.01f * height, Colors.withAlpha(accent(), 0.12f * h * a));
                    float bh = 0.022f * height * h, by = y + sz * 0.52f - bh / 2;
                    Draw.rect(colX() - 0.012f * height - 2, by, colX() - 0.012f * height, by + bh, Colors.withAlpha(accent(), a));
                }
                GlyphFont.draw(ITEMS[i], x, y, sz, false, Colors.withAlpha(Colors.mix(0xFFC9CCD3, 0xFFFFFFFF, h), a));
            }
            float fs = Math.max(6f, 0.014f * height / 0.7f);
            GlyphFont.draw("Vesper " + VesperClient.VERSION + " · Minecraft 1.8.9 · Not an official Minecraft product", colX(), height - 0.05f * height, fs, false, Colors.withAlpha(0x6C707A, nameIn));
        }
        if (exiting && ex >= 1) {
            exitAt = -1;
            openedAt = now - 2;
        }
        if (leave > 0) Draw.rect(0, 0, width, height, Colors.withAlpha(0x000000, leave * 0.6f));
        Gl.end2D();
    }

    /** Focus accent (Tidal blue). */
    static int accent() {
        return 0x6FB7FF;
    }

    private void sparkle(float x, float y, float size, float rotDeg) {
        if (size < 0.5f) return;
        GlStateManager.enableTexture2D();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        mc.getTextureManager().bindTexture(Textures.twinkle());
        GlStateManager.pushMatrix();
        GlStateManager.translate(x, y, 0);
        GlStateManager.rotate(rotDeg, 0, 0, 1);
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        w.pos(-size, -size, 0).tex(0, 0).color(255, 255, 255, 255).endVertex();
        w.pos(-size, size, 0).tex(0, 1).color(255, 255, 255, 255).endVertex();
        w.pos(size, size, 0).tex(1, 1).color(255, 255, 255, 255).endVertex();
        w.pos(size, -size, 0).tex(1, 0).color(255, 255, 255, 255).endVertex();
        t.draw();
        GlStateManager.popMatrix();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.disableTexture2D();
    }
}
