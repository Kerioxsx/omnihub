package dev.vesper.client.fx.sky;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.WorldClient;
import net.minecraftforge.client.IRenderHandler;

/**
 * Replaces the overworld sky with one of Vesper's skies, with optional
 * daylight that follows the game's clock, a moon, a Milky Way band,
 * shooting stars, hidden clouds and motes drifting around you. Sky only:
 * terrain, players and everything else draw as normal. Only you see it.
 */
public final class WorldSkyModule extends Module {
    public static WorldSkyModule INSTANCE;

    public final ChoiceSetting sky = choice("sky", "Sky", "nocturne", Skies.IDS, Skies.NAMES);
    public final ChoiceSetting time = choice("time", "Time of day", "night", new String[] {"night", "follow"}, new String[] {"Always this sky", "Daylight follows the game's clock"});
    public final BoolSetting moon = bool("moon", "Moon", true);
    public final NumberSetting moonSize = number("moonSize", "Moon size", 1.0, 0.5, 2.5, 0.1, "×");
    public final BoolSetting galaxy = bool("galaxy", "Milky Way band", true);
    public final BoolSetting shooting = bool("shooting", "Shooting stars", true);
    public final NumberSetting shootingRate = number("shootingRate", "Shooting stars per minute", 8, 1, 40, 1, "");
    public final ChoiceSetting clouds = choice("clouds", "Clouds", "vanilla", new String[] {"vanilla", "hidden"}, new String[] {"Minecraft's", "Hidden"});
    public final ChoiceSetting motes = choice("motes", "Around you", "none", new String[] {"none", "fireflies", "snow", "embers"}, new String[] {"Nothing", "Fireflies", "Snow sparkles", "Embers"});
    public final NumberSetting motesAmount = number("motesAmount", "Amount around you", 0.6, 0.1, 1.0, 0.05, "×");
    public final NumberSetting speed = number("speed", "Animation speed", 1.0, 0, 3, 0.1, "×");
    public final NumberSetting brightness = number("brightness", "Brightness", 1.0, 0.3, 1.6, 0.05, "×");

    private WorldClient applied;

    private static final int[] ACCENT = {0xFF6FB7FF, 0xFFFF5A4A, 0xFFD27BFF, 0xFFFFC46B, 0xFF7CF5C8, 0xFFFF8AD8, 0xFFFFD0E0, 0xFF5CFFE0};

    private final IRenderHandler handler = new IRenderHandler() {
        @Override
        public void render(float partialTicks, WorldClient world, Minecraft mc) {
            float celestial = world.getCelestialAngle(partialTicks);
            float t = Ease.seconds() * speed.f();
            boolean follow = time.is("follow");
            float day = follow ? SkyExtras.daylight(celestial) : 0;
            int accent = accent();
            Skies.drawDome(sky.get(), t, 1f);
            float night = 1 - day;
            if (galaxy.on()) SkyExtras.galaxy(t, night * (sky.is("galaxy") ? 1f : 0.55f), accent);
            if (shooting.on() && !General.calm()) SkyExtras.shootingStars(Ease.seconds(), shootingRate.f(), night, accent);
            // Daylight: a clear sky over the night one (stars fade out), tinted by the sky's accent at dusk.
            if (day > 0) SkyExtras.overlay(0xFF3D7BD9, Colors.mix(0xFFA6CFFF, accent, 0.25f * (1 - day)), day);
            if (follow || moon.on()) SkyExtras.sunAndMoon(follow ? celestial : 0.62f, day, moon.on(), moonSize.f(), accent);
            float b = brightness.f();
            if (b < 1) SkyExtras.overlay(0xFF000000, 0xFF000000, 1 - b);
            else if (b > 1) SkyExtras.overlay(accent, 0xFFFFFFFF, (b - 1) * 0.25f);
        }
    };

    private final IRenderHandler noClouds = new IRenderHandler() {
        @Override
        public void render(float partialTicks, WorldClient world, Minecraft mc) {
        }
    };

    public WorldSkyModule() {
        super("worldsky", "World sky", "Your own sky in the overworld, with daylight, moon, Milky Way, shooting stars and motes (only you see it; the world itself is unchanged).", Category.VISUALS, false, true);
        INSTANCE = this;
    }

    public int accent() {
        return ACCENT[Math.max(0, Math.min(ACCENT.length - 1, Skies.index(sky.get())))];
    }

    /** Puts the sky renderer on (or takes it off) the current world as needed. */
    public void tick() {
        WorldClient w = Minecraft.getMinecraft().theWorld;
        boolean want = enabled() && General.fx() && w != null && w.provider.getDimensionId() == 0;
        if (want && (applied != w || w.provider.getSkyRenderer() != handler)) {
            w.provider.setSkyRenderer(handler);
            applied = w;
        } else if (!want && w != null && w.provider.getSkyRenderer() == handler) {
            w.provider.setSkyRenderer(null);
            applied = null;
        }
        boolean hide = want && clouds.is("hidden");
        if (w != null) {
            if (hide && w.provider.getCloudRenderer() != noClouds) w.provider.setCloudRenderer(noClouds);
            else if (!hide && w.provider.getCloudRenderer() == noClouds) w.provider.setCloudRenderer(null);
        }
    }

    /** Motes around the camera, drawn in the world pass (render space = camera at vx,vy,vz). */
    public void renderWorld(double vx, double vy, double vz) {
        if (!enabled() || !General.fx() || motes.is("none")) return;
        WorldClient w = Minecraft.getMinecraft().theWorld;
        if (w == null || w.provider.getDimensionId() != 0) return;
        float amount = motesAmount.f() * (General.detail() == 0 ? 0.4f : 1f);
        SkyExtras.motes(motes.index(), Ease.seconds() * speed.f(), vx, vy, vz, vx, vy, vz, amount, accent());
    }
}
