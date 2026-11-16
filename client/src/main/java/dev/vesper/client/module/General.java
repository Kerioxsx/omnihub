package dev.vesper.client.module;

import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.KeySetting;
import dev.vesper.client.module.setting.NumberSetting;
import org.lwjgl.input.Keyboard;

/**
 * Client-wide options every package reads: quality preset, the master
 * effects switch, reduced motion and flashes, the custom title.
 */
public final class General extends Module {
    public static General INSTANCE;

    public final ChoiceSetting quality = choice("quality", "Quality", "balanced",
        new String[] {"low", "competitive", "balanced", "cinematic"},
        new String[] {"Low-end", "Competitive", "Balanced", "Cinematic"});
    public final BoolSetting effects = bool("effects", "All effects", true);
    public final KeySetting effectsKey = add(new KeySetting("effectsKey", "All effects on/off key", Keyboard.KEY_NONE));
    public final BoolSetting reducedMotion = bool("reducedMotion", "Reduced motion", false);
    public final BoolSetting reducedFlashes = bool("reducedFlashes", "Reduced flashes", true);
    public final BoolSetting customTitle = bool("customTitle", "Vesper title screen", true);
    public final BoolSetting splash = bool("splash", "\"Press to enter\" splash at start", true);
    public final BoolSetting playerPreview = bool("playerPreview", "Your player on the title screen", true);
    public final BoolSetting uiSounds = bool("uiSounds", "Menu sounds", true);
    public final NumberSetting uiScale = number("uiScale", "Menu text size", 1.0, 0.8, 1.4, 0.05, "×");
    public final dev.vesper.client.module.setting.ColorSetting accent = color("accent", "World colour (neon)", 0xFF2EE6FF);
    public final BoolSetting rainbow = bool("rainbow", "Rainbow world colour", false);

    public General() {
        super("general", "General", "Quality, effects master switch, motion and the title screen.", Category.GENERAL, true, true);
        INSTANCE = this;
    }

    @Override
    public void setEnabled(boolean on) {
        super.setEnabled(true); // always on
    }

    /** Effects may draw at all (master switch). */
    public static boolean fx() {
        return INSTANCE == null || INSTANCE.effects.on();
    }

    public static boolean calm() {
        return INSTANCE != null && INSTANCE.reducedMotion.on();
    }

    /** Particle budget for the whole client, by quality preset. */
    public static int particleBudget() {
        if (INSTANCE == null) return 600;
        String q = INSTANCE.quality.get();
        return q.equals("low") ? 120 : q.equals("competitive") ? 250 : q.equals("cinematic") ? 1500 : 600;
    }

    /** The neon colour you chose for your world (jump marks, glyphs, outlines…). */
    public static int accent() {
        if (INSTANCE == null) return 0xFF2EE6FF;
        if (INSTANCE.rainbow.on()) {
            float h = (dev.vesper.client.render.Ease.seconds() * 0.08f) % 1f;
            return 0xFF000000 | (java.awt.Color.HSBtoRGB(h, 0.75f, 1f) & 0xFFFFFF);
        }
        return INSTANCE.accent.argb() | 0xFF000000;
    }

    /** 0 low … 3 cinematic: detail level for geometry (segments, trail points). */
    public static int detail() {
        return INSTANCE == null ? 2 : INSTANCE.quality.index();
    }
}
