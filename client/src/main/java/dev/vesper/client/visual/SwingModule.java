package dev.vesper.client.visual;

import dev.vesper.client.VesperClient;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Ease;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.renderer.EntityRenderer;
import net.minecraft.client.renderer.ItemRenderer;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * How your sword and items swing in first person: several swing styles, the
 * 1.7 block-hit (swinging while blocking), swing speed (only the look; the
 * game's own timing is unchanged), size and position, and no lowering when
 * you switch items. Purely visual.
 */
public final class SwingModule extends Module {
    public static SwingModule INSTANCE;

    public static final String[] STYLES = {"vanilla", "classic", "smooth", "spin", "stab", "chop", "swirl", "flick"};
    public final ChoiceSetting style = choice("style", "Swing style", "smooth", STYLES,
        new String[] {"1.8 (Minecraft)", "1.7 classic", "Smooth", "Spin", "Stab", "Chop", "Swirl", "Flick"});
    public final ChoiceSetting block = choice("block", "While blocking", "swing", new String[] {"still", "swing", "push", "tilt"},
        new String[] {"1.8 (still)", "1.7 block-hit (swings)", "Push", "Tilted block-hit"});
    public final NumberSetting speed = number("speed", "Swing speed (look only)", 1.0, 0.4, 2.0, 0.05, "×");
    public final NumberSetting size = number("size", "Item size", 1.0, 0.5, 1.5, 0.05, "×");
    public final NumberSetting x = number("x", "Left / right", 0, -0.6, 0.6, 0.02, "");
    public final NumberSetting y = number("y", "Down / up", 0, -0.6, 0.6, 0.02, "");
    public final NumberSetting z = number("z", "Near / far", 0, -0.6, 0.6, 0.02, "");
    public final BoolSetting noLowering = bool("noLowering", "No lowering when switching items", false);

    private float start = -10;
    private boolean lastIn;
    private int lastInt;
    private boolean installed, failed;

    public SwingModule() {
        super("swing", "Sword animations", "Swing styles, 1.7 block-hit, swing speed, item size and position (first person, only you see it).", Category.VISUALS, true, true);
        INSTANCE = this;
    }

    /** 0..1 through the current swing, smooth between ticks; 0 when not swinging. */
    float progress(EntityPlayerSP p, float partialTicks) {
        if (style.is("vanilla") && Math.abs(speed.f() - 1) < 0.01f) return p.getSwingProgress(partialTicks);
        float len = 0.3f / speed.f();
        float t = (Ease.seconds() - start) / len;
        return t < 0 || t >= 1 ? 0 : t;
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END) return;
        Minecraft mc = Minecraft.getMinecraft();
        if (!installed && !failed && mc.entityRenderer != null) install(mc);
        EntityPlayerSP p = mc.thePlayer;
        if (p == null) return;
        boolean in = p.isSwingInProgress;
        int cur = p.swingProgressInt;
        if (in && (!lastIn || cur < lastInt)) start = Ease.seconds() - Math.max(0, cur) * 0.05f / speed.f();
        lastIn = in;
        lastInt = cur;
    }

    /** Puts Vesper's item renderer in place of Minecraft's (same behaviour when the module is off). */
    private void install(Minecraft mc) {
        try {
            VesperItemRenderer r = new VesperItemRenderer(mc);
            int n = replace(Minecraft.class, mc, r) + replace(EntityRenderer.class, mc.entityRenderer, r);
            installed = n >= 2;
            if (!installed) {
                failed = true;
                VesperClient.LOG.warn("Sword animations: couldn't find the item renderer ({} of 2 places)", n);
            }
        } catch (Throwable t) {
            failed = true;
            VesperClient.LOG.warn("Sword animations unavailable: {}", t.toString());
        }
    }

    private static int replace(Class<?> owner, Object target, ItemRenderer r) throws Exception {
        int n = 0;
        for (Field f : owner.getDeclaredFields()) {
            if (f.getType() != ItemRenderer.class || Modifier.isStatic(f.getModifiers())) continue;
            f.setAccessible(true);
            if (Modifier.isFinal(f.getModifiers())) {
                Field mods = Field.class.getDeclaredField("modifiers");
                mods.setAccessible(true);
                mods.setInt(f, f.getModifiers() & ~Modifier.FINAL);
            }
            f.set(target, r);
            n++;
        }
        return n;
    }
}
