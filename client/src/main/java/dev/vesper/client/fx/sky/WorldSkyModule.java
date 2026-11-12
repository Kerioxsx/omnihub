package dev.vesper.client.fx.sky;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.render.Ease;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.WorldClient;
import net.minecraftforge.client.IRenderHandler;

/**
 * Replaces the overworld sky with one of Vesper's skies. Sky only: terrain,
 * players and everything else draw as normal. Only you see it.
 */
public final class WorldSkyModule extends Module {
    public final ChoiceSetting sky = choice("sky", "Sky", "nocturne", Skies.IDS, Skies.NAMES);
    private WorldClient applied;

    private final IRenderHandler handler = new IRenderHandler() {
        @Override
        public void render(float partialTicks, WorldClient world, Minecraft mc) {
            Skies.drawDome(sky.get(), Ease.seconds(), 1f);
        }
    };

    public WorldSkyModule() {
        super("worldsky", "World sky", "Your own sky in the overworld (only you see it; the world itself is unchanged).", Category.VISUALS, false, true);
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
    }
}
