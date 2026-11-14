package dev.vesper.client;

import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.gui.TitleScreen;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.render.GlyphFont;
import dev.vesper.client.render.Textures;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiMainMenu;
import net.minecraftforge.client.event.GuiOpenEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.event.world.WorldEvent;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.InputEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import net.minecraftforge.fml.common.network.FMLNetworkEvent;
import org.lwjgl.input.Keyboard;

/** Client-wide Forge wiring (title swap, saving, keys, cleanup). Packages register their own handlers. */
public final class Hooks {
    private static final Hooks INSTANCE = new Hooks();
    private int lastW, lastH;

    private Hooks() {
    }

    static void register() {
        MinecraftForge.EVENT_BUS.register(INSTANCE);
        FMLCommonHandler.instance().bus().register(INSTANCE);
    }

    /** Swaps the vanilla title for ours. Ours is not a GuiMainMenu, so this can't loop. */
    @SubscribeEvent
    public void onGuiOpen(GuiOpenEvent e) {
        if (e.gui instanceof GuiMainMenu && TitleScreen.enabled()) e.gui = new TitleScreen();
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END) return;
        CombatBus.deliver();
        if (VesperClient.config() != null) VesperClient.config().tick();
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.displayWidth != lastW || mc.displayHeight != lastH) {
            // New window size or GUI scale: re-rasterise the smooth font.
            lastW = mc.displayWidth;
            lastH = mc.displayHeight;
            GlyphFont.clear();
        }
    }

    /** Module toggle keys and the all-effects key, only while playing (not in menus). */
    @SubscribeEvent
    public void onKey(InputEvent.KeyInputEvent e) {
        if (!Keyboard.getEventKeyState() || Minecraft.getMinecraft().currentScreen != null) return;
        int k = Keyboard.getEventKey();
        if (k <= 0) return;
        General g = General.INSTANCE;
        if (g != null && g.effectsKey.key() == k) {
            g.effects.toggle();
            VesperClient.config().markDirty();
            return;
        }
        for (Module m : ModuleRegistry.all()) {
            if (m.toggleKey.key() == k) {
                m.toggle();
                VesperClient.config().markDirty();
            }
        }
    }

    @SubscribeEvent
    public void onWorldUnload(WorldEvent.Unload e) {
        if (e.world.isRemote) CombatBus.clear();
    }

    @SubscribeEvent
    public void onDisconnect(FMLNetworkEvent.ClientDisconnectionFromServerEvent e) {
        CombatBus.clear();
        if (VesperClient.config() != null) VesperClient.config().save();
    }

    /** After a resource reload our generated textures are gone. */
    public static void onResourcesReloaded() {
        Textures.reset();
        GlyphFont.clear();
    }
}
