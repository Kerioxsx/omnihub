package dev.vesper.client.visual;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.KeySetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Ease;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiOptions;
import net.minecraft.client.gui.GuiVideoSettings;
import net.minecraft.client.settings.GameSettings;
import net.minecraftforge.client.event.FOVUpdateEvent;
import net.minecraftforge.client.event.MouseEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.input.Keyboard;

/**
 * Field of view past Minecraft's 110 (up to 150), a steady FOV that doesn't
 * jump when you sprint or get Speed, and a smooth zoom key with scroll to
 * zoom further. Only the camera changes; the hand keeps its normal look.
 * The game's own FOV setting is never overwritten: Vesper's value is only
 * used while a frame is drawn.
 */
public final class FovModule extends Module {
    public final BoolSetting custom = bool("custom", "Use my own FOV", false);
    public final NumberSetting fov = number("fov", "FOV", 100, 30, 150, 1, "°");
    public final BoolSetting steady = bool("steady", "Steady FOV (no sprint / Speed zoom)", false);
    public final KeySetting zoomKey = add(new KeySetting("zoomKey", "Zoom key (hold)", Keyboard.KEY_C));
    public final NumberSetting zoom = number("zoom", "Zoom", 4, 1.5, 12, 0.5, "×");
    public final BoolSetting scroll = bool("scroll", "Scroll to zoom further", true);
    public final BoolSetting smooth = bool("smooth", "Smooth zoom", true);
    public final BoolSetting slowMouse = bool("slowMouse", "Slower mouse while zoomed", true);
    public final BoolSetting cinematic = bool("cinematic", "Cinematic camera while zoomed", false);

    private float cur = 1, scrollMul = 1;
    private boolean applied;
    private float userFov, userSens;
    private boolean userSmooth;
    private long last;

    public FovModule() {
        super("fov", "FOV & zoom", "FOV up to 150, a steady FOV while sprinting, and a smooth zoom key (C) with scroll.", Category.VISUALS, true, true);
    }

    private boolean zoomHeld() {
        Minecraft mc = Minecraft.getMinecraft();
        int k = zoomKey.key();
        return enabled() && mc.theWorld != null && mc.currentScreen == null && k > 0 && Keyboard.isKeyDown(k);
    }

    @SubscribeEvent
    public void onRender(TickEvent.RenderTickEvent e) {
        Minecraft mc = Minecraft.getMinecraft();
        GameSettings gs = mc.gameSettings;
        if (e.phase == TickEvent.Phase.START) {
            long now = System.nanoTime();
            float dt = last == 0 ? 0.016f : Math.min(0.1f, (now - last) / 1e9f);
            last = now;
            boolean held = zoomHeld();
            if (!held) scrollMul = 1;
            float target = held ? zoom.f() * scrollMul : 1;
            cur = smooth.on() ? Ease.approach(cur, target, 14f, dt) : target;
            if (Math.abs(cur - target) < 0.002f) cur = target;
            boolean menus = mc.currentScreen instanceof GuiOptions || mc.currentScreen instanceof GuiVideoSettings;
            boolean useCustom = enabled() && custom.on() && !menus;
            if (!useCustom && cur == 1) return;
            userFov = gs.fovSetting;
            userSens = gs.mouseSensitivity;
            userSmooth = gs.smoothCamera;
            applied = true;
            float base = useCustom ? fov.f() : userFov;
            gs.fovSetting = base / cur;
            if (cur > 1.01f) {
                if (slowMouse.on()) {
                    // The game cubes (s·0.6+0.2); divide that by the zoom.
                    double k = (userSens * 0.6 + 0.2) / Math.cbrt(cur);
                    gs.mouseSensitivity = (float) Math.max(-0.33, (k - 0.2) / 0.6);
                }
                if (cinematic.on()) gs.smoothCamera = true;
            }
        } else if (applied) {
            gs.fovSetting = userFov;
            gs.mouseSensitivity = userSens;
            gs.smoothCamera = userSmooth;
            applied = false;
        }
    }

    @SubscribeEvent
    public void onFov(FOVUpdateEvent e) {
        if (enabled() && steady.on()) e.newfov = 1f;
    }

    @SubscribeEvent
    public void onMouse(MouseEvent e) {
        if (e.dwheel == 0 || !scroll.on() || !zoomHeld()) return;
        scrollMul = Math.max(0.4f, Math.min(4f, scrollMul * (e.dwheel > 0 ? 1.25f : 0.8f)));
        e.setCanceled(true);
    }
}
