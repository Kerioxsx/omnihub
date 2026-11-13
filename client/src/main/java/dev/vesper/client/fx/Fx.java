package dev.vesper.client.fx;

import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.fx.sky.WorldSkyModule;
import dev.vesper.client.module.General;
import dev.vesper.client.module.ModuleRegistry;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraftforge.client.event.RenderWorldLastEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.event.world.WorldEvent;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import net.minecraftforge.fml.common.network.FMLNetworkEvent;

/** Effects entry point: registers the effect modules and draws world effects. */
public final class Fx {
    private static KillFxModule kill;
    private static HitSparkModule spark;
    private static WorldSkyModule sky;
    private static HitLinesModule lines;

    private Fx() {
    }

    public static void init() {
        kill = ModuleRegistry.register(new KillFxModule());
        spark = ModuleRegistry.register(new HitSparkModule());
        sky = ModuleRegistry.register(new WorldSkyModule());
        lines = ModuleRegistry.register(new HitLinesModule());
        CombatBus.listen(new CombatBus.Listener() {
            public void on(CombatVisualEvent e) {
                kill.on(e);
                spark.on(e);
                lines.on(e);
            }
        });
        Fx h = new Fx();
        MinecraftForge.EVENT_BUS.register(h);
        FMLCommonHandler.instance().bus().register(h);
    }

    @SubscribeEvent
    public void onRenderWorld(RenderWorldLastEvent e) {
        if (!General.fx()) {
            KillEffects.clearWorld();
            return;
        }
        RenderManager rm = Minecraft.getMinecraft().getRenderManager();
        sky.renderWorld(rm.viewerPosX, rm.viewerPosY, rm.viewerPosZ);
        KillEffects.renderWorld(rm.viewerPosX, rm.viewerPosY, rm.viewerPosZ);
        dev.vesper.client.render.Gl.beginGlow();
        View.capture();
        lines.renderWorld(rm.viewerPosX, rm.viewerPosY, rm.viewerPosZ);
        dev.vesper.client.render.Gl.endGlow();
    }

    @SubscribeEvent
    public void onHud(net.minecraftforge.client.event.RenderGameOverlayEvent.Post e) {
        if (e.type == net.minecraftforge.client.event.RenderGameOverlayEvent.ElementType.ALL && General.fx()) lines.renderHud(e.partialTicks);
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase == TickEvent.Phase.END) sky.tick();
    }

    @SubscribeEvent
    public void onWorldUnload(WorldEvent.Unload e) {
        if (!e.world.isRemote) return;
        KillEffects.clearWorld();
        kill.clear();
    }

    @SubscribeEvent
    public void onDisconnect(FMLNetworkEvent.ClientDisconnectionFromServerEvent e) {
        KillEffects.clearWorld();
        kill.clear();
    }
}
