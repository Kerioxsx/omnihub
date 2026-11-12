package dev.vesper.client.fx;

import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import java.util.HashMap;
import java.util.Map;
import java.util.Random;
import net.minecraft.client.Minecraft;
import net.minecraft.client.audio.PositionedSoundRecord;
import net.minecraft.util.ResourceLocation;

/**
 * Kill effects: plays the chosen effect where something you killed fell.
 * Only on kills the game confirms (singleplayer / LAN host), or also on
 * best-effort chat-parsed kills if you allow it. Never on a target simply
 * disappearing.
 */
public final class KillFxModule extends Module {
    public static KillFxModule INSTANCE;

    private static final String[] CHOICE_IDS = new String[KillEffects.IDS.length + 1], CHOICE_NAMES = new String[KillEffects.IDS.length + 1];

    static {
        System.arraycopy(KillEffects.IDS, 0, CHOICE_IDS, 0, KillEffects.IDS.length);
        System.arraycopy(KillEffects.NAMES, 0, CHOICE_NAMES, 0, KillEffects.NAMES.length);
        CHOICE_IDS[KillEffects.IDS.length] = "random";
        CHOICE_NAMES[KillEffects.IDS.length] = "Random";
    }

    public final ChoiceSetting effect = choice("effect", "Effect", "astral_collapse", CHOICE_IDS, CHOICE_NAMES);
    public final BoolSetting[] allowed = new BoolSetting[KillEffects.IDS.length];
    public final NumberSetting intensity = number("intensity", "Intensity", 1.0, 0.2, 1.5, 0.05, "×");
    public final NumberSetting duration = number("duration", "Length", 1.0, 0.5, 1.5, 0.05, "×");
    public final BoolSetting useTint = bool("useTint", "Own colour", false);
    public final ColorSetting tint = color("tint", "Colour", 0xFF38C6F4);
    public final ChoiceSetting sound = choice("sound", "Sound", "random.orb",
        new String[] {"none", "random.orb", "random.levelup", "note.pling", "mob.wither.shoot", "fireworks.blast", "ambient.weather.thunder"},
        new String[] {"None", "Orb", "Level up", "Pling", "Wither", "Firework", "Thunder"});
    public final NumberSetting maxAtOnce = number("maxAtOnce", "At most at once", 3, 1, 5, 1, "");
    public final ChoiceSetting trigger = choice("trigger", "Play on", "confirmed",
        new String[] {"confirmed", "confirmed_or_best_effort"},
        new String[] {"Confirmed kills (singleplayer/LAN)", "Also best-effort chat kills"});

    private final Map<Integer, Long> recent = new HashMap<Integer, Long>();
    private final Random rnd = new Random();

    public KillFxModule() {
        super("killfx", "Kill effects", "An original effect where a player or mob you killed fell. Only you see it.", Category.EFFECTS, true, true);
        INSTANCE = this;
        for (int i = 0; i < KillEffects.IDS.length; i++) allowed[i] = bool("on_" + KillEffects.IDS[i], KillEffects.NAMES[i] + " in Random", true);
        tint.visibleWhen(() -> useTint.on());
        for (BoolSetting b : allowed) b.visibleWhen(() -> effect.is("random"));
    }

    public KillSettings killSettings() {
        KillSettings s = new KillSettings();
        s.intensity = intensity.f();
        s.duration = duration.f();
        s.useTint = useTint.on();
        s.tint = tint.argb();
        s.calm = General.calm();
        s.reducedFlashes = General.INSTANCE == null || General.INSTANCE.reducedFlashes.on();
        return s;
    }

    private String pick() {
        if (!effect.is("random")) return effect.get();
        int n = 0;
        for (BoolSetting b : allowed) if (b.on()) n++;
        if (n == 0) return KillEffects.IDS[0];
        int k = rnd.nextInt(n);
        for (int i = 0; i < allowed.length; i++) if (allowed[i].on() && k-- == 0) return KillEffects.IDS[i];
        return KillEffects.IDS[0];
    }

    void on(CombatVisualEvent e) {
        if (!enabled() || !General.fx()) return;
        boolean ok = e.type == CombatVisualEvent.Type.DEMO
            || e.type == CombatVisualEvent.Type.PLAYER_KILL_CONFIRMED && (e.confidence == CombatVisualEvent.Confidence.CONFIRMED || e.confidence == CombatVisualEvent.Confidence.BEST_EFFORT && trigger.is("confirmed_or_best_effort"));
        if (!ok) return;
        long now = System.currentTimeMillis();
        if (e.targetEntityId >= 0) {
            Long last = recent.get(e.targetEntityId);
            if (last != null && now - last < 2000) return; // debounce duplicates
            recent.put(e.targetEntityId, now);
            if (recent.size() > 64) recent.clear();
        }
        KillEffects.play(Particles.WORLD, pick(), e.x, e.y, e.z, e.height, killSettings(), maxAtOnce.i());
        if (!sound.is("none")) Minecraft.getMinecraft().getSoundHandler().playSound(PositionedSoundRecord.create(new ResourceLocation(sound.get()), 1.0f));
    }

    void clear() {
        recent.clear();
    }
}
