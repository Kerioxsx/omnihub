package dev.vesper.client.hud;

import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.render.Gl;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraftforge.client.event.MouseEvent;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/** HUD entry point (owner: hud-pvp): registers HUD element modules and its render handler. */
public final class Hud {
    private static final List<HudElement> ELEMENTS = new ArrayList<HudElement>();

    // CPS: timestamps of presses in a ring buffer per button (display only).
    private static final long[] LEFT = new long[64], RIGHT = new long[64];
    private static int leftAt, rightAt;

    private static long worldJoinMs;

    private Hud() {
    }

    public static void init() {
        add(new StatusBarElement());
        add(new FpsElement());
        add(new PingElement());
        add(new CpsElement());
        add(new KeystrokesElement());
        add(new ArmorElement());
        add(new PotionsElement());
        add(new CoordsElement());
        add(new SessionElement());
        add(new TargetInfoElement());
        InfoElements.init();
        MinecraftForge.EVENT_BUS.register(new ForgeEvents());
        FMLCommonHandler.instance().bus().register(new FmlEvents());
    }

    /** Adds an element registered elsewhere (the info readouts). */
    public static void addElement(HudElement e) {
        add(e);
    }

    private static void add(HudElement e) {
        ELEMENTS.add(ModuleRegistry.register(e));
    }

    public static List<HudElement> elements() {
        return Collections.unmodifiableList(ELEMENTS);
    }

    /** Clicks of a mouse button (0 left, 1 right) within the last second. */
    public static int cps(int button) {
        long[] buf = button == 0 ? LEFT : RIGHT;
        long cut = System.currentTimeMillis() - 1000;
        int n = 0;
        for (long t : buf) if (t > cut) n++;
        return n;
    }

    /** System time the current world was joined (0 when not in a world). */
    public static long worldJoinMs() {
        return worldJoinMs;
    }

    /** Screen x of an element whose scaled width is `w` (place.x is a fraction of the free space). */
    static float screenX(HudLayout.Place p, float w, float sw) {
        return Math.max(0, p.x * Math.max(0, sw - w));
    }

    static float screenY(HudLayout.Place p, float h, float sh) {
        return Math.max(0, p.y * Math.max(0, sh - h));
    }

    static void render(float sw, float sh) {
        for (int i = 0; i < ELEMENTS.size(); i++) {
            HudElement el = ELEMENTS.get(i);
            if (!el.enabled()) continue;
            HudLayout.Place p = HudLayout.get(el.id);
            if (!p.shown) continue;
            el.prepare(p.scale);
            el.refresh(false);
            if (!el.hasContent()) continue;
            float w = el.width() * p.scale, h = el.height() * p.scale;
            Gl.begin2D();
            try {
                el.renderAt(screenX(p, w, sw), screenY(p, h, sh), p.scale, false, 1f);
            } finally {
                Gl.end2D();
            }
        }
    }

    public static final class ForgeEvents {
        @SubscribeEvent
        public void onOverlay(RenderGameOverlayEvent.Post e) {
            if (e.type != RenderGameOverlayEvent.ElementType.ALL) return;
            Minecraft mc = Minecraft.getMinecraft();
            if (mc.gameSettings.hideGUI || mc.gameSettings.showDebugInfo || mc.thePlayer == null) return;
            if (mc.currentScreen instanceof HudEditorScreen) return; // the editor draws its own copies
            ScaledResolution sr = e.resolution;
            render(sr.getScaledWidth(), sr.getScaledHeight());
        }

        /** In-game mouse presses (no screen open), counted for the CPS display only. */
        @SubscribeEvent
        public void onMouse(MouseEvent e) {
            if (!e.buttonstate) return;
            long now = System.currentTimeMillis();
            if (e.button == 0) {
                LEFT[leftAt] = now;
                leftAt = (leftAt + 1) % LEFT.length;
            } else if (e.button == 1) {
                RIGHT[rightAt] = now;
                rightAt = (rightAt + 1) % RIGHT.length;
            }
        }
    }

    public static final class FmlEvents {
        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            if (e.phase != TickEvent.Phase.END) return;
            boolean inWorld = Minecraft.getMinecraft().theWorld != null;
            if (inWorld && worldJoinMs == 0) worldJoinMs = System.currentTimeMillis();
            else if (!inWorld && worldJoinMs != 0) {
                worldJoinMs = 0;
                java.util.Arrays.fill(LEFT, 0);
                java.util.Arrays.fill(RIGHT, 0);
            }
        }
    }
}
