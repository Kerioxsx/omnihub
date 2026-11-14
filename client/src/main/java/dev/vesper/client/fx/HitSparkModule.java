package dev.vesper.client.fx;

import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import java.util.Random;

/** A small spark where you swing at something. Purely visual; nothing about the hit changes. */
public final class HitSparkModule extends Module {
    public final ColorSetting color = color("color", "Colour", 0xFFFFFFFF);
    public final NumberSetting amount = number("amount", "Sparks", 6, 2, 16, 1, "");
    private final Random rnd = new Random();

    public HitSparkModule() {
        super("hitspark", "Hit sparks", "A small spark of light on the target when you attack. Only you see it.", Category.EFFECTS, true, true);
    }

    void on(CombatVisualEvent e) {
        if (!enabled() || !General.fx() || e.type != CombatVisualEvent.Type.HIT_CONFIRMED) return;
        int n = Math.max(1, Math.round(amount.i() * (General.detail() == 0 ? 0.5f : 1f)));
        double cy = e.y + e.height * 0.6;
        double t = Particles.now();
        for (int k = 0; k < n; k++) {
            int i = Particles.spawn(Particles.WORLD, Particles.TEX_TWINKLE, t, e.x, cy, e.z, 0.25f + rnd.nextFloat() * 0.15f);
            if (i < 0) return;
            float th = rnd.nextFloat() * 6.283f, sp = 1.5f + rnd.nextFloat() * 1.5f;
            Particles.vel(i, (float) Math.cos(th) * sp, (rnd.nextFloat() - 0.3f) * sp, (float) Math.sin(th) * sp, 6f);
            Particles.size(i, 0.07f, 0.02f);
            Particles.color(i, color.argb(), color.argb() & 0x00FFFFFF);
            Particles.spin(i, rnd.nextFloat() * 3, (rnd.nextFloat() - 0.5f) * 8);
        }
    }
}
