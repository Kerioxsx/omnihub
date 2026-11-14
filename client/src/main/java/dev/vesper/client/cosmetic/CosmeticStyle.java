package dev.vesper.client.cosmetic;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.NumberSetting;

/** Shared cosmetic options: the palette every cosmetic follows, intensity, where they show. */
public final class CosmeticStyle extends Module {
    public static CosmeticStyle INSTANCE;

    public final ChoiceSetting palette = choice("palette", "Palette", "tidal", Palette.IDS, Palette.NAMES);
    public final NumberSetting intensity = number("intensity", "Intensity", 100, 0, 100, 5, "%");
    public final BoolSetting inventory = bool("inventory", "Show in the inventory preview", true);

    public CosmeticStyle() {
        super("cosmetic_style", "Cosmetic style", "Palette and intensity shared by all cosmetics. Visible only to you.", Category.COSMETICS, true, true);
        INSTANCE = this;
    }

    @Override
    public void setEnabled(boolean on) {
        super.setEnabled(true); // a settings holder, always on
    }

    /** 0..1 */
    public static float level() {
        return INSTANCE == null ? 1f : INSTANCE.intensity.f() / 100f;
    }
}
