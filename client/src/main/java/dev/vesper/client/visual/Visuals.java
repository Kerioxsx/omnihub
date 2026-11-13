package dev.vesper.client.visual;

import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.music.WorldLyricsModule;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;

/** Registers the camera, blur, sword-animation and world-lyrics modules. */
public final class Visuals {
    private Visuals() {
    }

    public static void init() {
        Module[] ms = {new FovModule(), new BlurModules.Menu(), new BlurModules.Motion(), new SwingModule(), new WorldLyricsModule()};
        for (Module m : ms) {
            ModuleRegistry.register(m);
            MinecraftForge.EVENT_BUS.register(m);
            FMLCommonHandler.instance().bus().register(m);
        }
    }
}
