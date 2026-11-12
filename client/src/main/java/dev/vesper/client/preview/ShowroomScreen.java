package dev.vesper.client.preview;

import dev.vesper.client.VesperClient;
import dev.vesper.client.cosmetic.CosmeticStyle;
import dev.vesper.client.cosmetic.Cosmetics;
import dev.vesper.client.cosmetic.Palette;
import dev.vesper.client.cosmetic.Scene;
import dev.vesper.client.cosmetic.Sprites;
import dev.vesper.client.fx.KillEffects;
import dev.vesper.client.fx.sky.Skies;
import dev.vesper.client.gui.UiSound;
import dev.vesper.client.module.Module;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import dev.vesper.client.render.ObjModel;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.RenderHelper;
import org.lwjgl.input.Mouse;
import org.lwjgl.opengl.GL11;
import org.lwjgl.util.glu.Project;

/**
 * The local showroom: a small 3D stage with a sky, your avatar on a
 * turntable wearing your cosmetics, a test figure for kill effects and the
 * optional "Skiff" hover-cart prop. Nothing here is a world or an entity.
 */
public class ShowroomScreen extends GuiScreen {
    private final GuiScreen parent;
    private float yaw = -20, pitch = 12, dist = 4.6f, targetDist = 4.6f;
    private float spin;
    private boolean dragging, autoSpin = true;
    private int lastX;
    private static final float NPC_X = 2.4f, NPC_Z = -0.6f;
    private static String sky = "nocturne";
    private static boolean skiff;
    private float lastFrame;

    private static final class Hit {
        final float x, y, w, h;
        final Runnable r;

        Hit(float x, float y, float w, float h, Runnable r) {
            this.x = x;
            this.y = y;
            this.w = w;
            this.h = h;
            this.r = r;
        }
    }

    private final List<Hit> hits = new ArrayList<Hit>();

    public ShowroomScreen(GuiScreen parent) {
        this.parent = parent;
    }

    @Override
    public void initGui() {
        lastFrame = Ease.seconds();
    }

    @Override
    public void onGuiClosed() {
        KillEffects.clearStage();
        VesperClient.config().save();
    }

    @Override
    protected void keyTyped(char c, int key) {
        if (key == 1) mc.displayGuiScreen(parent);
    }

    @Override
    protected void mouseClicked(int mx, int my, int button) {
        for (int i = hits.size() - 1; i >= 0; i--) {
            Hit h = hits.get(i);
            if (mx >= h.x && mx < h.x + h.w && my >= h.y && my < h.y + h.h) {
                UiSound.press();
                h.r.run();
                return;
            }
        }
        if (button == 0) {
            dragging = true;
            autoSpin = false;
            lastX = mx;
        }
    }

    @Override
    protected void mouseClickMove(int mx, int my, int button, long held) {
        if (dragging) {
            yaw += (mx - lastX) * 0.8f;
            lastX = mx;
        }
    }

    @Override
    protected void mouseReleased(int mx, int my, int state) {
        dragging = false;
    }

    @Override
    public void handleMouseInput() throws java.io.IOException {
        super.handleMouseInput();
        int d = Mouse.getEventDWheel();
        if (d != 0) targetDist = Math.max(2.8f, Math.min(9f, targetDist - Math.signum(d) * 0.5f));
    }

    @Override
    public void drawScreen(int mx, int my, float pt) {
        float now = Ease.seconds();
        float dt = Math.min(0.1f, now - lastFrame);
        lastFrame = now;
        if (autoSpin) spin = 14 * (float) Math.sin(now * 0.25f);
        dist = Ease.approach(dist, targetDist, 10, dt);
        hits.clear();

        drawStage(now, pt);
        drawUi(mx, my, now);
    }

    private void drawStage(float now, float pt) {
        Gl.begin2D();
        Draw.rect(0, 0, width, height, 0xFF000000);
        Gl.end2D();
        GlStateManager.clear(GL11.GL_DEPTH_BUFFER_BIT);
        GlStateManager.matrixMode(GL11.GL_PROJECTION);
        GlStateManager.pushMatrix();
        GlStateManager.loadIdentity();
        Project.gluPerspective(55, mc.displayWidth / (float) mc.displayHeight, 0.05f, 300f);
        GlStateManager.matrixMode(GL11.GL_MODELVIEW);
        GlStateManager.pushMatrix();
        GlStateManager.loadIdentity();
        // Sky: rotation only.
        GlStateManager.pushMatrix();
        GlStateManager.rotate(pitch, 1, 0, 0);
        GlStateManager.rotate(yaw + spin, 0, 1, 0);
        Skies.drawDome(sky, now, 1f);
        GlStateManager.popMatrix();
        // Camera orbit around the avatar's chest.
        GlStateManager.translate(-0.9f, -1.05f, -dist);
        GlStateManager.rotate(pitch, 1, 0, 0);
        GlStateManager.rotate(yaw + spin, 0, 1, 0);
        GlStateManager.enableDepth();
        GlStateManager.depthMask(true);
        floor(now);
        // Avatar at the origin facing +z.
        figure(0, 0, 0, 0, now, false);
        GlStateManager.pushMatrix();
        Cosmetics.drawFor(Scene.STAGE, 0, 1f);
        GlStateManager.popMatrix();
        // Test figure for kill effects.
        figure(NPC_X, 0, NPC_Z, -35, now, true);
        if (skiff) skiff(now);
        KillEffects.renderStage(pt);
        GlStateManager.matrixMode(GL11.GL_PROJECTION);
        GlStateManager.popMatrix();
        GlStateManager.matrixMode(GL11.GL_MODELVIEW);
        GlStateManager.popMatrix();
        GlStateManager.clear(GL11.GL_DEPTH_BUFFER_BIT);
        GlStateManager.enableTexture2D();
        GlStateManager.disableDepth();
        GlStateManager.disableLighting();
        GlStateManager.color(1, 1, 1, 1);
    }

    private void figure(float x, float y, float z, float bodyYaw, float now, boolean npc) {
        GlStateManager.pushMatrix();
        GlStateManager.translate(x, y, z);
        GlStateManager.rotate(180 - bodyYaw, 0, 1, 0);
        GlStateManager.scale(-0.9375f, -0.9375f, 0.9375f);
        GlStateManager.translate(0, -1.5f, 0);
        GlStateManager.enableRescaleNormal();
        GlStateManager.enableAlpha();
        GlStateManager.disableCull();
        RenderHelper.enableStandardItemLighting();
        Avatar.drawModel(now + (npc ? 1.3f : 0), 1f, 0, npc);
        RenderHelper.disableStandardItemLighting();
        GlStateManager.disableRescaleNormal();
        GlStateManager.popMatrix();
    }

    private void floor(float now) {
        GlStateManager.pushMatrix();
        GlStateManager.disableTexture2D();
        GlStateManager.disableLighting();
        GlStateManager.disableCull();
        GlStateManager.enableBlend();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        Palette pal = Palette.active();
        net.minecraft.client.renderer.Tessellator t = net.minecraft.client.renderer.Tessellator.getInstance();
        net.minecraft.client.renderer.WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_TRIANGLE_FAN, net.minecraft.client.renderer.vertex.DefaultVertexFormats.POSITION_COLOR);
        w.pos(0, 0, 0).color(28, 30, 38, 255).endVertex();
        for (int i = 0; i <= 64; i++) {
            double a = i * Math.PI * 2 / 64;
            w.pos(Math.cos(a) * 6, 0, Math.sin(a) * 6).color(10, 11, 16, 0).endVertex();
        }
        t.draw();
        // Fine rings on the floor.
        GlStateManager.depthMask(false);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        for (int k = 1; k <= 3; k++) {
            w.begin(GL11.GL_TRIANGLE_STRIP, net.minecraft.client.renderer.vertex.DefaultVertexFormats.POSITION_COLOR);
            float r = k * 1.2f;
            int c = pal.ring;
            int a = (int) (60 / k * (0.7f + 0.3f * Math.sin(now * 1.2f + k)));
            for (int i = 0; i <= 96; i++) {
                double an = i * Math.PI * 2 / 96;
                w.pos(Math.cos(an) * (r - 0.01), 0.005, Math.sin(an) * (r - 0.01)).color((c >> 16) & 255, (c >> 8) & 255, c & 255, a).endVertex();
                w.pos(Math.cos(an) * (r + 0.01), 0.005, Math.sin(an) * (r + 0.01)).color((c >> 16) & 255, (c >> 8) & 255, c & 255, a).endVertex();
            }
            t.draw();
        }
        GlStateManager.depthMask(true);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.shadeModel(GL11.GL_FLAT);
        GlStateManager.enableTexture2D();
        GlStateManager.popMatrix();
    }

    /** The Blender-made hover-cart with an accretion ring ("Eclipse Halo") and a floating flame. */
    private void skiff(float now) {
        ObjModel m = ObjModel.get("skiff.obj");
        float bob = 0.06f * (float) Math.sin(now * 1.4f);
        GlStateManager.pushMatrix();
        GlStateManager.translate(0.9f, 0.15f + bob, -3.2f);
        GlStateManager.rotate(35, 0, 1, 0);
        GlStateManager.scale(0.9f, 0.9f, 0.9f);
        GlStateManager.disableLighting();
        GlStateManager.enableBlend();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.disableCull();
        if (m.ok) {
            m.draw("hull", 0xFF1A1D26);
            m.draw("fin", 0xFF262A36);
            m.draw("thruster", 0xFF3A3F4E);
            m.draw("glass", 0xCC9FD8FF);
            GlStateManager.depthMask(false);
            GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
            m.draw("glow", Colors.withAlpha(Palette.active().accent, 0.7f + 0.3f * (float) Math.sin(now * 3)));
        }
        // Accretion ring above: thin disc, blue on the approaching side and pink-red on the receding side.
        GlStateManager.depthMask(false);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        GlStateManager.pushMatrix();
        GlStateManager.translate(0, 2.6f, 0);
        GlStateManager.rotate(14, 1, 0, 0);
        GlStateManager.rotate(now * 25, 0, 1, 0);
        net.minecraft.client.renderer.Tessellator t = net.minecraft.client.renderer.Tessellator.getInstance();
        net.minecraft.client.renderer.WorldRenderer w = t.getWorldRenderer();
        GlStateManager.disableTexture2D();
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        for (int band = 0; band < 3; band++) {
            float r1 = 0.55f + band * 0.18f, r2 = r1 + 0.16f;
            w.begin(GL11.GL_TRIANGLE_STRIP, net.minecraft.client.renderer.vertex.DefaultVertexFormats.POSITION_COLOR);
            for (int i = 0; i <= 72; i++) {
                double a = i * Math.PI * 2 / 72;
                float k = (float) (0.5 + 0.5 * Math.cos(a));
                int c = Colors.mix(0xFF4FA8FF, 0xFFFF4F7A, k);
                int al = (int) ((140 - band * 35) * (0.7f + 0.3f * (float) Math.sin(a * 5 + now * 3)));
                w.pos(Math.cos(a) * r1, 0, Math.sin(a) * r1).color((c >> 16) & 255, (c >> 8) & 255, c & 255, al).endVertex();
                w.pos(Math.cos(a) * r2, 0, Math.sin(a) * r2).color((c >> 16) & 255, (c >> 8) & 255, c & 255, 0).endVertex();
            }
            t.draw();
        }
        GlStateManager.popMatrix();
        // The dark core of the ring and the flame below it.
        Sprites.camera();
        Sprites.beginGlow();
        Sprites.add(0, 2.6f, 0, 0.45f, 0x66FFFFFF);
        float f = 0.9f + 0.1f * (float) Math.sin(now * 9);
        for (int i = 0; i < 6; i++) {
            float u = i / 6f;
            Sprites.add(0.04f * (float) Math.sin(now * 6 + i), 1.6f + u * 0.5f, 0, (0.32f - u * 0.04f) * f, Colors.withAlpha(Colors.mix(0xFFFFE08A, 0xFFFF3B1E, u), 0.7f - u * 0.1f));
        }
        Sprites.add(0, 1.62f, 0, 0.16f, 0xFFFFFFFF);
        Sprites.end();
        // Snow sparkles drifting down around the prop.
        Sprites.beginTwinkle();
        for (int i = 0; i < 24; i++) {
            float life = (now * 0.12f + Sprites.hash(i, 1)) % 1f;
            Sprites.add((Sprites.hash(i, 2) - 0.5f) * 4, 3.2f - life * 3.2f, (Sprites.hash(i, 3) - 0.5f) * 4, 0.05f, Colors.withAlpha(0xFFFFFF, (float) Math.sin(life * Math.PI) * 0.8f));
        }
        Sprites.end();
        GlStateManager.depthMask(true);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.shadeModel(GL11.GL_FLAT);
        GlStateManager.enableTexture2D();
        GlStateManager.enableCull();
        GlStateManager.popMatrix();
    }

    // ---------- UI ----------

    private float u() {
        return height / 100f;
    }

    private void drawUi(int mx, int my, float now) {
        Gl.begin2D();
        float px = 3 * u(), py = 4 * u(), pw = 36 * u();
        Draw.roundRect(px, py, pw, height - 8 * u(), 1.5f * u(), 0xC80E0F13);
        float y = py + 2.5f * u(), x = px + 2 * u(), w = pw - 4 * u();
        GlyphFont.draw("Showroom", x, y, 3.2f * u(), true, 0xFFF2F3F5);
        y += 4.8f * u();
        GlyphFont.draw("Only you see this. Drag to turn, scroll to zoom.", x, y, 1.6f * u(), false, 0xFF8C909A);
        y += 4 * u();
        y = section("PALETTE", x, y);
        String active = CosmeticStyle.INSTANCE == null ? "tidal" : CosmeticStyle.INSTANCE.palette.get();
        float bw = (w - 2 * u()) / 3;
        for (int i = 0; i < Palette.ALL.length; i++) {
            final Palette p = Palette.ALL[i];
            chip(x + i * (bw + u()), y, bw, p.name, p.id.equals(active), p.accent, mx, my, new Runnable() {
                public void run() {
                    CosmeticStyle.INSTANCE.palette.set(p.id);
                    sky = p.sky;
                    VesperClient.config().markDirty();
                }
            });
        }
        y += 5 * u();
        y = section("SKY", x, y);
        for (int i = 0; i < Skies.IDS.length; i++) {
            final String id = Skies.IDS[i];
            float cw = (w - 2 * u()) / 3;
            chip(x + (i % 3) * (cw + u()), y + (i / 3) * 4.4f * u(), cw, Skies.NAMES[i], id.equals(sky), 0, mx, my, new Runnable() {
                public void run() {
                    sky = id;
                }
            });
        }
        y += 9.6f * u();
        y = section("COSMETICS", x, y);
        Cosmetics.Set set = Cosmetics.set();
        Module[] mods = {set.wings, set.comet, set.orbit, set.halo, set.floor, set.glints};
        for (int i = 0; i < mods.length; i++) {
            final Module m = mods[i];
            float cw = (w - u()) / 2;
            chip(x + (i % 2) * (cw + u()), y + (i / 2) * 4.4f * u(), cw, m.name, m.enabled(), 0, mx, my, new Runnable() {
                public void run() {
                    m.toggle();
                    VesperClient.config().markDirty();
                }
            });
        }
        y += 13.6f * u();
        y = section("KILL EFFECTS (DEMO)", x, y);
        for (int i = 0; i < KillEffects.IDS.length; i++) {
            final String id = KillEffects.IDS[i];
            chip(x, y, w, "›  " + KillEffects.NAMES[i], false, 0, mx, my, new Runnable() {
                public void run() {
                    KillEffects.demoStage(id, NPC_X, 0, NPC_Z);
                }
            });
            y += 4.4f * u();
        }
        y += u();
        chip(x, y, w, skiff ? "Hide the Skiff prop" : "Show the Skiff prop", skiff, 0, mx, my, new Runnable() {
            public void run() {
                skiff = !skiff;
            }
        });
        GlyphFont.draw("Esc to go back", x, height - 7 * u(), 1.5f * u(), false, 0xFF5C606A);
        Gl.end2D();
    }

    private float section(String label, float x, float y) {
        GlyphFont.draw(label, x, y, 1.6f * u(), true, 0xFF7C808A);
        return y + 2.8f * u();
    }

    private void chip(float x, float y, float w, String label, boolean on, int dot, int mx, int my, Runnable r) {
        float h = 3.6f * u();
        boolean hov = mx >= x && mx < x + w && my >= y && my < y + h;
        Draw.roundRect(x, y, w, h, 1.1f * u(), on ? 0x40FFFFFF : hov ? 0x2AFFFFFF : 0x16FFFFFF);
        float tx = x + 1.2f * u();
        if (dot != 0) {
            Draw.circle(tx + 0.7f * u(), y + h / 2, 0.7f * u(), dot | 0xFF000000);
            tx += 2.2f * u();
        }
        GlyphFont.draw(label, tx, y + (h - 1.7f * u() * 1.25f) / 2, 1.7f * u(), false, on ? 0xFFFFFFFF : 0xFFC9CCD3);
        hits.add(new Hit(x, y, w, h, r));
    }

    @Override
    public boolean doesGuiPauseGame() {
        return false;
    }
}
