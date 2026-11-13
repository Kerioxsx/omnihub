package dev.vesper.client.visual;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiChat;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraftforge.client.event.GuiOpenEvent;
import net.minecraftforge.client.event.GuiScreenEvent;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;

/** Motion blur and the soft blur behind in-game menus. */
public final class BlurModules {
    private BlurModules() {
    }

    /**
     * Motion blur: each frame keeps a little of the last one, so fast turns
     * smear smoothly. Drawn under the HUD, so text stays sharp.
     */
    public static final class Motion extends Module {
        public final NumberSetting amount = number("amount", "Strength", 0.5, 0.1, 0.9, 0.05, "");
        private final FrameGrab grab = new FrameGrab();

        public Motion() {
            super("motionblur", "Motion blur", "Smooth trails when you turn quickly (under the HUD, which stays sharp).", Category.VISUALS, false, true);
        }

        @Override
        protected void onDisable() {
            grab.forget();
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Pre e) {
            if (e.type != RenderGameOverlayEvent.ElementType.ALL) return;
            if (!enabled() || General.calm()) {
                grab.forget();
                return;
            }
            if (grab.ready()) {
                grab.begin();
                grab.draw(amount.f() * 0.94f, 0, 0);
                grab.end();
            }
            grab.copy();
        }
    }

    /**
     * Menu blur: in-game menus (pause, inventory, chests…) blur the world
     * behind them, fading in smoothly, with your own tint.
     */
    public static final class Menu extends Module {
        public final NumberSetting radius = number("radius", "Blur", 8, 2, 20, 1, "px");
        public final NumberSetting fade = number("fade", "Fade in", 0.25, 0, 0.8, 0.05, "s");
        public final ColorSetting tint = color("tint", "Tint", 0x55101018);
        public final BoolSetting containers = bool("containers", "Behind inventories and chests too", true);
        private final FrameGrab grab = new FrameGrab();
        private float opened = -1;

        public Menu() {
            super("menublur", "Menu blur", "A smooth blur behind in-game menus, fading in, with your own tint.", Category.VISUALS, true, true);
        }

        @SubscribeEvent
        public void onOpen(GuiOpenEvent e) {
            Minecraft mc = Minecraft.getMinecraft();
            if (e.gui == null) opened = -1;
            else if (mc.currentScreen == null || opened < 0) opened = Ease.seconds();
        }

        @SubscribeEvent
        public void onBackground(GuiScreenEvent.BackgroundDrawnEvent e) {
            Minecraft mc = Minecraft.getMinecraft();
            if (!enabled() || mc.theWorld == null || e.gui instanceof GuiChat) return;
            if (!containers.on() && e.gui instanceof net.minecraft.client.gui.inventory.GuiContainer) return;
            if (opened < 0) opened = Ease.seconds();
            float f = fade.f() <= 0 ? 1 : Ease.outCubic(Ease.clamp01((Ease.seconds() - opened) / fade.f()));
            if (General.calm()) f = 1;
            int scale = new ScaledResolution(mc).getScaleFactor();
            float rad = radius.f() * f / scale;
            grab.copy();
            grab.begin();
            grab.draw(1, 0, 0);
            // Progressive average of offset copies around a disc (a soft, round blur).
            int taps = General.detail() <= 0 ? 8 : 16;
            for (int k = 1; k <= taps; k++) {
                double a = k * 2.39996323, r = rad * Math.sqrt(k / (double) taps);
                grab.draw(1f / (k + 1), Math.cos(a) * r, Math.sin(a) * r);
            }
            grab.end();
            ScaledResolution sr = new ScaledResolution(mc);
            Gl.begin2D();
            Draw.rect(0, 0, sr.getScaledWidth(), sr.getScaledHeight(), Colors.alpha(tint.argb(), f));
            Gl.end2D();
        }
    }
}
