package dev.vesper.client;

import dev.vesper.client.combat.Combat;
import dev.vesper.client.config.ConfigStore;
import dev.vesper.client.cosmetic.Cosmetics;
import dev.vesper.client.fx.Fx;
import dev.vesper.client.hud.Hud;
import dev.vesper.client.module.General;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.module.Performance;
import java.io.File;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.event.FMLInitializationEvent;
import net.minecraftforge.fml.common.event.FMLPreInitializationEvent;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/** Entry point: loads the settings and wires the client's event handlers. */
@Mod(modid = VesperClient.MODID, name = VesperClient.NAME, version = VesperClient.VERSION, acceptedMinecraftVersions = "[1.8.9]", acceptableRemoteVersions = "*")
public class VesperClient {
    public static final String MODID = "vesper";
    public static final String NAME = "Vesper Client";
    public static final String VERSION = "0.1.0";
    public static final Logger LOG = LogManager.getLogger(NAME);

    private static ConfigStore config;

    public static ConfigStore config() {
        return config;
    }

    @Mod.EventHandler
    public void preInit(FMLPreInitializationEvent e) {
        config = new ConfigStore(new File(e.getModConfigurationDirectory(), "vesper"));
        config.load();
        LOG.info("Vesper settings loaded from {}", config.file());
    }

    @Mod.EventHandler
    public void init(FMLInitializationEvent e) {
        // Modules first (each package registers its own), then their stored values.
        ModuleRegistry.register(new General());
        ModuleRegistry.register(new Performance());
        Combat.init();
        Fx.init();
        Cosmetics.init();
        Hud.init();
        config.apply();
        Hooks.register();
        net.minecraftforge.client.ClientCommandHandler.instance.registerCommand(new dev.vesper.client.diag.VesperCommand());
        // Our generated textures (font, glow) go away on a resource reload; make them again.
        ((net.minecraft.client.resources.IReloadableResourceManager) net.minecraft.client.Minecraft.getMinecraft().getResourceManager()).registerReloadListener(rm -> Hooks.onResourcesReloaded());
        LOG.info("Vesper Client {} ready", VERSION);
    }
}
