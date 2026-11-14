package dev.vesper.client.visual;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.KeySetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Ease;
import java.lang.reflect.Field;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.EntityRenderer;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.entity.Entity;
import net.minecraftforge.client.event.EntityViewRenderEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.InputEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.input.Keyboard;

/**
 * POV: how the third-person camera sits. Distance, an over-the-shoulder
 * offset and height, a smooth glide out of first person when you press F5,
 * F5 without the front view, and an optional freelook key (look around in
 * third person without turning). Only the camera moves; where you face,
 * what the server sees and your aim stay the same.
 */
public final class PovModule extends Module {
    public final NumberSetting distance = number("distance", "Third-person distance", 4, 1, 12, 0.25, " blocks");
    public final NumberSetting shoulder = number("shoulder", "Over-the-shoulder offset", 0, -1.5, 1.5, 0.05, "");
    public final NumberSetting height = number("height", "Camera height", 0, -1, 1.5, 0.05, "");
    public final BoolSetting glide = bool("glide", "Smooth glide when switching view", true);
    public final BoolSetting skipFront = bool("skipFront", "F5 skips the front view", false);
    public final BoolSetting frontToo = bool("frontToo", "Offset the front view too", false);
    public final BoolSetting freelook = bool("freelook", "Freelook key (check the server's rules)", false);
    public final KeySetting freelookKey = add(new KeySetting("freelookKey", "Freelook key (hold)", Keyboard.KEY_LMENU));

    private static Field dist, distTemp;
    private static boolean looked;

    private int lastView;
    private float shown = 4, glideFrom = -1, glideAt;
    private long last;

    // Freelook state.
    private boolean looking;
    private int viewBefore;
    private float camYaw, camPitch, realYaw, realPitch, realPrevYaw, realPrevPitch;

    public PovModule() {
        super("pov", "POV & camera", "Third-person distance, over-the-shoulder camera, a smooth F5 glide and freelook.", Category.VISUALS, true, true);
    }

    private static void find() {
        if (looked) return;
        looked = true;
        dist = field("thirdPersonDistance", "field_78490_B");
        distTemp = field("thirdPersonDistanceTemp", "field_78491_C");
    }

    private static Field field(String... names) {
        for (String n : names) {
            try {
                Field f = EntityRenderer.class.getDeclaredField(n);
                f.setAccessible(true);
                return f;
            } catch (Exception ignored) {
                // try the next name
            }
        }
        return null;
    }

    private static void setDistance(EntityRenderer r, float d) {
        try {
            if (dist != null) dist.setFloat(r, d);
            if (distTemp != null) distTemp.setFloat(r, d);
        } catch (Exception ignored) {
            // leave Minecraft's own distance
        }
    }

    @Override
    protected void onDisable() {
        Minecraft mc = Minecraft.getMinecraft();
        stopLooking(mc);
        if (mc.entityRenderer != null) {
            find();
            setDistance(mc.entityRenderer, 4);
        }
    }

    @SubscribeEvent
    public void onKey(InputEvent.KeyInputEvent e) {
        Minecraft mc = Minecraft.getMinecraft();
        // F5 straight from the back view to first person.
        if (enabled() && skipFront.on() && mc.gameSettings.thirdPersonView == 2 && lastView == 1 && !looking) mc.gameSettings.thirdPersonView = 0;
    }

    @SubscribeEvent
    public void onRender(TickEvent.RenderTickEvent e) {
        Minecraft mc = Minecraft.getMinecraft();
        if (!enabled() || mc.thePlayer == null || mc.entityRenderer == null) return;
        Entity p = mc.thePlayer;
        if (e.phase == TickEvent.Phase.START) {
            long now = System.nanoTime();
            float dt = last == 0 ? 0.016f : Math.min(0.1f, (now - last) / 1e9f);
            last = now;
            int view = mc.gameSettings.thirdPersonView;
            boolean wantLook = freelook.on() && freelookKey.key() > 0 && mc.currentScreen == null && Keyboard.isKeyDown(freelookKey.key());
            if (wantLook && !looking) {
                looking = true;
                viewBefore = view;
                camYaw = p.rotationYaw;
                camPitch = p.rotationPitch;
                if (view == 0) mc.gameSettings.thirdPersonView = 1;
            } else if (!wantLook && looking) {
                stopLooking(mc);
            }
            view = mc.gameSettings.thirdPersonView;
            if (view != 0 && lastView == 0 && glide.on()) {
                glideFrom = 0.4f;
                glideAt = Ease.seconds();
            }
            lastView = view;
            float target = distance.f();
            if (glideFrom >= 0) {
                float k = Ease.span(Ease.seconds(), glideAt, glideAt + 0.32f);
                shown = glideFrom + (target - glideFrom) * Ease.outCubic(k);
                if (k >= 1) glideFrom = -1;
            } else {
                shown = Ease.approach(shown, target, 14, dt);
            }
            find();
            setDistance(mc.entityRenderer, shown);
            if (looking) {
                // The mouse turns the camera this frame, not the player.
                realYaw = p.rotationYaw;
                realPitch = p.rotationPitch;
                realPrevYaw = p.prevRotationYaw;
                realPrevPitch = p.prevRotationPitch;
                p.rotationYaw = p.prevRotationYaw = camYaw;
                p.rotationPitch = p.prevRotationPitch = camPitch;
            }
        } else if (looking) {
            camYaw = p.rotationYaw;
            camPitch = Math.max(-90, Math.min(90, p.rotationPitch));
            p.rotationYaw = realYaw;
            p.rotationPitch = realPitch;
            p.prevRotationYaw = realPrevYaw;
            p.prevRotationPitch = realPrevPitch;
        }
    }

    private void stopLooking(Minecraft mc) {
        if (!looking) return;
        looking = false;
        mc.gameSettings.thirdPersonView = viewBefore;
    }

    @SubscribeEvent
    public void onCamera(EntityViewRenderEvent.CameraSetup e) {
        Minecraft mc = Minecraft.getMinecraft();
        int view = mc.gameSettings.thirdPersonView;
        if (!enabled() || view == 0 || (view == 2 && !frontToo.on())) return;
        float s = shoulder.f(), h = height.f();
        if (s == 0 && h == 0) return;
        // Fired before the camera turns, so this moves it across the screen.
        GlStateManager.translate(-s, -h, 0);
    }
}
