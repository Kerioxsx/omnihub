package dev.vesper.client.visual;

import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.music.WorldLyricsModule;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;

/** Registers the camera, blur, sword, lyrics, world-effect, sound and avatar modules. */
public final class Visuals {
    private Visuals() {
    }

    public static void init() {
        Module[] ms = {new FovModule(), new BlurModules.Menu(), new BlurModules.Motion(), new BlurModules.Speed(), new SwingModule(), new WorldLyricsModule(),
            new dev.vesper.client.world.JumpFxModule(), new dev.vesper.client.world.WorldGlyphsModule(), new dev.vesper.client.world.WeatherModule(),
            new dev.vesper.client.world.MoreVisuals.HitSounds(), new dev.vesper.client.world.MoreVisuals.BlockOutline(), new dev.vesper.client.world.MoreVisuals.TimeChanger(),
            new dev.vesper.client.avatar.Emotes(), new dev.vesper.client.avatar.WraithHead()};
        for (Module m : ms) {
            ModuleRegistry.register(m);
            MinecraftForge.EVENT_BUS.register(m);
            FMLCommonHandler.instance().bus().register(m);
        }
    }
}
