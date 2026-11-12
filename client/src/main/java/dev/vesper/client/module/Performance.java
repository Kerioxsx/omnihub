package dev.vesper.client.module;

import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.NumberSetting;
import net.minecraft.client.Minecraft;
import net.minecraft.entity.EntityLivingBase;
import net.minecraft.entity.item.EntityArmorStand;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraftforge.client.event.RenderLivingEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.EventPriority;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.opengl.Display;

/**
 * Performance: skips drawing far-away mobs and armour stands (players are
 * always drawn, so nothing in a fight is hidden), and caps the frame rate
 * in menus and while the game is in the background so it stops heating
 * the PC for nothing. Your frame-rate setting comes back the moment you play.
 */
public final class Performance extends Module {
    public static Performance INSTANCE;

    public final BoolSetting cullEntities = bool("cullEntities", "Skip far mobs and armour stands", true);
    public final NumberSetting cullDistance = number("cullDistance", "…beyond", 48, 16, 128, 4, " blocks");
    public final BoolSetting capBackground = bool("capBackground", "Low FPS when the game is in the background", true);
    public final NumberSetting backgroundFps = number("backgroundFps", "Background FPS", 15, 5, 60, 5, "");
    public final BoolSetting capMenus = bool("capMenus", "Cap FPS on the title screen and menus", true);
    public final NumberSetting menuFps = number("menuFps", "Menu FPS", 60, 30, 144, 5, "");

    private int savedLimit = -1;

    public Performance() {
        super("performance", "Performance", "Draws less of what you can't use and saves power in menus and in the background.", Category.GENERAL, true, true);
        INSTANCE = this;
        cullDistance.visibleWhen(() -> cullEntities.on());
        backgroundFps.visibleWhen(() -> capBackground.on());
        menuFps.visibleWhen(() -> capMenus.on());
        MinecraftForge.EVENT_BUS.register(this);
        FMLCommonHandler.instance().bus().register(this);
    }

    @SubscribeEvent(priority = EventPriority.HIGH)
    public void onRenderLiving(RenderLivingEvent.Pre<EntityLivingBase> e) {
        if (!enabled() || !cullEntities.on()) return;
        EntityLivingBase ent = e.entity;
        if (ent instanceof EntityPlayer) return;
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.thePlayer == null) return;
        double d = cullDistance.get();
        double dist2 = ent.getDistanceSqToEntity(mc.thePlayer);
        // Armour stands (often hundreds of holograms in lobbies) use half the distance.
        if (ent instanceof EntityArmorStand) d *= 0.5;
        if (dist2 > d * d) e.setCanceled(true);
    }

    @SubscribeEvent
    public void onTick(TickEvent.RenderTickEvent e) {
        if (e.phase != TickEvent.Phase.START) return;
        Minecraft mc = Minecraft.getMinecraft();
        int want = -1;
        if (enabled()) {
            boolean background = !Display.isActive();
            // Never while a settings screen is open: it would show (and save) our cap as yours.
            boolean settings = mc.currentScreen instanceof net.minecraft.client.gui.GuiOptions || mc.currentScreen instanceof net.minecraft.client.gui.GuiVideoSettings;
            boolean menu = mc.theWorld == null && mc.currentScreen != null && !settings;
            if (settings) background = false;
            if (capBackground.on() && background) want = backgroundFps.i();
            else if (capMenus.on() && menu) want = menuFps.i();
        }
        if (want > 0) {
            if (savedLimit < 0) savedLimit = mc.gameSettings.limitFramerate;
            mc.gameSettings.limitFramerate = Math.min(savedLimit, want);
        } else if (savedLimit >= 0) {
            mc.gameSettings.limitFramerate = savedLimit;
            savedLimit = -1;
        }
    }

    /** The player's own limit (not our temporary cap), for saving options. */
    public static void restore() {
        if (INSTANCE != null && INSTANCE.savedLimit >= 0) {
            Minecraft.getMinecraft().gameSettings.limitFramerate = INSTANCE.savedLimit;
            INSTANCE.savedLimit = -1;
        }
    }
}
