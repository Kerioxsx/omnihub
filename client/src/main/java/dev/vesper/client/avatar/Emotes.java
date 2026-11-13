package dev.vesper.client.avatar;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.KeySetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import dev.vesper.client.render.ObjModel;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.model.ModelPlayer;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraftforge.client.event.RenderPlayerEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.InputEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.input.Keyboard;
import org.lwjgl.opengl.GL11;

/**
 * Emotes for your own player: flex, wave, dance, salute, cheer, and a car
 * flex where an (original, unbranded) car drops in next to you with neon
 * underglow while you lean, arms crossed. Pick them from a wheel (B). Only
 * you see them; moving or attacking stops the emote.
 */
public final class Emotes extends Module {
    public static Emotes INSTANCE;

    public static final String[] IDS = {"flex", "wave", "dance", "salute", "cheer", "carflex"};
    public static final String[] NAMES = {"Flex", "Wave", "Dance", "Salute", "Cheer", "Car flex"};

    public final KeySetting wheelKey = add(new KeySetting("wheelKey", "Emote wheel key", Keyboard.KEY_B));
    public final BoolSetting thirdPerson = bool("thirdPerson", "Turn the camera to face you", true);
    public final NumberSetting duration = number("duration", "Length (0 = until you move)", 8, 0, 30, 1, "s");
    public final ChoiceSetting car = choice("car", "Car", "super", new String[] {"super", "muscle", "hyper"}, new String[] {"Supercar", "Muscle car", "Hypercar"});
    public final BoolSetting carWorld = bool("carWorld", "Car in the world colour", false);
    public final ColorSetting carColor = color("carColor", "Car colour", 0xFFE11D2E);
    public final BoolSetting underglow = bool("underglow", "Neon underglow", true);

    private String current;
    private float start;
    private int prevView = -1;

    public Emotes() {
        super("emotes", "Emotes", "Flex, wave, dance, salute, cheer, or flex a car next to you. Press B for the emote wheel. Only you see them.", Category.COSMETICS, true, true);
        INSTANCE = this;
    }

    public boolean active() {
        return enabled() && current != null;
    }

    public void play(String id) {
        Minecraft mc = Minecraft.getMinecraft();
        if (!enabled() || mc.thePlayer == null) return;
        VesperPlayerModel.install();
        current = id;
        start = Ease.seconds();
        if (thirdPerson.on() && prevView < 0) {
            prevView = mc.gameSettings.thirdPersonView;
            mc.gameSettings.thirdPersonView = 2;
        }
    }

    public void stop() {
        current = null;
        Minecraft mc = Minecraft.getMinecraft();
        if (prevView >= 0) {
            mc.gameSettings.thirdPersonView = prevView;
            prevView = -1;
        }
    }

    @SubscribeEvent
    public void onKey(InputEvent.KeyInputEvent e) {
        Minecraft mc = Minecraft.getMinecraft();
        int k = wheelKey.key();
        if (enabled() && k > 0 && Keyboard.getEventKey() == k && Keyboard.getEventKeyState() && mc.currentScreen == null && mc.thePlayer != null) mc.displayGuiScreen(new Wheel());
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END || current == null) return;
        Minecraft mc = Minecraft.getMinecraft();
        EntityPlayerSP p = mc.thePlayer;
        if (p == null) {
            stop();
            return;
        }
        boolean moved = p.movementInput != null && (Math.abs(p.movementInput.moveForward) > 0.01f || Math.abs(p.movementInput.moveStrafe) > 0.01f || p.movementInput.jump);
        boolean timeUp = duration.f() > 0 && Ease.seconds() - start > duration.f();
        if (moved || timeUp || mc.gameSettings.keyBindAttack.isKeyDown() || !enabled()) stop();
    }

    /** Applies the current emote to the model (radians; right arm Z+ swings outward). */
    void pose(ModelPlayer m) {
        float t = Ease.seconds() - start;
        float in = Ease.outCubic(Ease.clamp01(t / 0.25f));
        if ("flex".equals(current)) {
            float pulse = 0.12f * (float) Math.sin(t * 7);
            m.bipedRightArm.rotateAngleX = 0;
            m.bipedLeftArm.rotateAngleX = 0;
            m.bipedRightArm.rotateAngleZ = (2.35f + pulse) * in;
            m.bipedLeftArm.rotateAngleZ = -(2.35f + pulse) * in;
            m.bipedRightArm.rotateAngleY = 0;
            m.bipedLeftArm.rotateAngleY = 0;
            m.bipedHead.rotateAngleX = -0.2f * in;
        } else if ("wave".equals(current)) {
            m.bipedRightArm.rotateAngleX = -2.85f * in;
            m.bipedRightArm.rotateAngleZ = (0.25f + 0.35f * (float) Math.sin(t * 9)) * in;
            m.bipedHead.rotateAngleZ = 0.1f * (float) Math.sin(t * 4.5);
        } else if ("dance".equals(current)) {
            float s = (float) Math.sin(t * 8);
            m.bipedRightArm.rotateAngleX = s * 1.3f;
            m.bipedLeftArm.rotateAngleX = -s * 1.3f;
            m.bipedRightArm.rotateAngleZ = 0.4f + 0.3f * (float) Math.cos(t * 8);
            m.bipedLeftArm.rotateAngleZ = -0.4f - 0.3f * (float) Math.cos(t * 8);
            m.bipedRightLeg.rotateAngleX = -s * 0.45f;
            m.bipedLeftLeg.rotateAngleX = s * 0.45f;
            m.bipedHead.rotateAngleY = 0.45f * (float) Math.sin(t * 4);
            m.bipedBody.rotateAngleY = 0.18f * (float) Math.sin(t * 4);
        } else if ("salute".equals(current)) {
            m.bipedRightArm.rotateAngleX = -2.1f * in;
            m.bipedRightArm.rotateAngleY = -0.65f * in;
            m.bipedRightArm.rotateAngleZ = 0.15f;
            m.bipedHead.rotateAngleX = 0;
            m.bipedHead.rotateAngleY = 0;
        } else if ("cheer".equals(current)) {
            float b = (float) Math.abs(Math.sin(t * 6));
            m.bipedRightArm.rotateAngleX = -2.9f * in;
            m.bipedLeftArm.rotateAngleX = -2.9f * in;
            m.bipedRightArm.rotateAngleZ = (0.3f + 0.2f * b) * in;
            m.bipedLeftArm.rotateAngleZ = -(0.3f + 0.2f * b) * in;
            m.bipedHead.rotateAngleX = -0.35f * b;
        } else if ("carflex".equals(current)) {
            m.bipedRightArm.rotateAngleX = -0.95f * in;
            m.bipedLeftArm.rotateAngleX = -0.95f * in;
            m.bipedRightArm.rotateAngleY = -0.75f * in;
            m.bipedLeftArm.rotateAngleY = 0.75f * in;
            m.bipedRightArm.rotateAngleZ = 0;
            m.bipedLeftArm.rotateAngleZ = 0;
            m.bipedHead.rotateAngleY = 0.35f * in;
            m.bipedRightLeg.rotateAngleY = 0.15f;
            m.bipedRightLeg.rotateAngleX = -0.12f;
        }
    }

    /** The car next to you during the car flex (drawn after your player). */
    @SubscribeEvent
    public void onRenderPlayer(RenderPlayerEvent.Post e) {
        Minecraft mc = Minecraft.getMinecraft();
        if (!active() || !"carflex".equals(current) || e.entityPlayer != mc.thePlayer || mc.currentScreen != null) return;
        ObjModel model = ObjModel.get("car_" + car.get() + ".obj");
        if (!model.ok) return;
        float t = Ease.seconds() - start;
        float drop = 1 - Ease.outBack(Ease.clamp01(t / 0.55f));
        float pt = e.partialRenderTick;
        EntityPlayerSP p = mc.thePlayer;
        float bodyYaw = p.prevRenderYawOffset + (p.renderYawOffset - p.prevRenderYawOffset) * pt;
        int body = carWorld.on() ? General.accent() : carColor.argb();
        GlStateManager.pushMatrix();
        GlStateManager.translate(e.x, e.y, e.z);
        GlStateManager.rotate(-bodyYaw, 0, 1, 0);
        // Parked on your right, turned a little toward the camera, dropping in from above.
        GlStateManager.translate(-2.5, drop * 3.5, 0.6);
        GlStateManager.rotate(-25, 0, 1, 0);
        GlStateManager.disableLighting();
        GlStateManager.disableCull();
        GlStateManager.enableBlend();
        GlStateManager.disableAlpha();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        GlStateManager.disableTexture2D();
        GlStateManager.enableDepth();
        model.draw("body", body);
        model.draw("glass", 0xF0121A24);
        model.draw("wheels", 0xFF16161A);
        model.draw("lights", 0xFFFFF7DD);
        model.draw("tail", 0xFFFF2A2A);
        if (underglow.on()) {
            GlStateManager.depthMask(false);
            GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
            View.capture();
            FxDraw.geo();
            int g = General.accent();
            float pulse = 0.75f + 0.25f * (float) Math.sin(t * 3);
            FxDraw.ringH(0, 0.04, 0, 0, 1.9, Colors.alpha(g, 0.55f * pulse), Colors.alpha(g, 0), 40);
            FxDraw.line(-1.02, 0.2, -2.0, -1.02, 0.2, 2.0, 0.05, Colors.alpha(g, pulse));
            FxDraw.line(1.02, 0.2, -2.0, 1.02, 0.2, 2.0, 0.05, Colors.alpha(g, pulse));
            FxDraw.flush();
            GlStateManager.depthMask(true);
            GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        }
        GlStateManager.shadeModel(GL11.GL_FLAT);
        GlStateManager.enableAlpha();
        GlStateManager.enableCull();
        GlStateManager.enableTexture2D();
        GlStateManager.disableBlend();
        GlStateManager.color(1, 1, 1, 1);
        GlStateManager.popMatrix();
    }

    /** The emote wheel: six choices around the centre; click one, or Esc. */
    static final class Wheel extends GuiScreen {
        private final float opened = Ease.seconds();

        @Override
        public boolean doesGuiPauseGame() {
            return false;
        }

        private int hovered(int mx, int my) {
            float cx = width / 2f, cy = height / 2f;
            float dx = mx - cx, dy = my - cy;
            if (dx * dx + dy * dy < 30 * 30) return -1;
            double a = Math.atan2(dy, dx) + Math.PI / 2 + Math.PI / IDS.length;
            if (a < 0) a += Math.PI * 2;
            return (int) (a / (Math.PI * 2 / IDS.length)) % IDS.length;
        }

        @Override
        public void drawScreen(int mx, int my, float pt) {
            float k = Ease.outBack(Ease.clamp01((Ease.seconds() - opened) / 0.2f));
            float cx = width / 2f, cy = height / 2f, r = 78 * k;
            int accent = General.accent();
            int h = hovered(mx, my);
            Gl.begin2D();
            Draw.circle(cx, cy, r + 34, 0x99101018);
            Draw.ring(cx, cy, r + 32, r + 34, Colors.alpha(accent, 0.6f), Colors.alpha(accent, 0.6f));
            for (int i = 0; i < IDS.length; i++) {
                double a = -Math.PI / 2 + i * Math.PI * 2 / IDS.length;
                float x = cx + (float) Math.cos(a) * r, y = cy + (float) Math.sin(a) * r;
                Draw.circle(x, y, 24, i == h ? Colors.alpha(accent, 0.85f) : 0xCC1C1C26);
            }
            Gl.end2D();
            for (int i = 0; i < IDS.length; i++) {
                double a = -Math.PI / 2 + i * Math.PI * 2 / IDS.length;
                float x = cx + (float) Math.cos(a) * r, y = cy + (float) Math.sin(a) * r;
                GlyphFont.drawCentered(NAMES[i], x, y - 5, 10, true, i == h ? 0xFF0B0B10 : 0xFFFFFFFF);
            }
            GlyphFont.drawCentered("Emotes", cx, cy - 6, 11, true, 0xFFFFFFFF);
        }

        @Override
        protected void mouseClicked(int mx, int my, int button) {
            int h = hovered(mx, my);
            mc.displayGuiScreen(null);
            if (h >= 0 && INSTANCE != null) INSTANCE.play(IDS[h]);
        }
    }
}
