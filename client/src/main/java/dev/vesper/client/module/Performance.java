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
    public final BoolSetting boost = bool("boost", "FPS boost game settings (yours come back when off)", false);
    public final NumberSetting boostDistance = number("boostDistance", "…render distance at most", 8, 2, 16, 1, " chunks");

    private int savedLimit = -1;

    public Performance() {
        super("performance", "Performance", "Draws less of what you can't use and saves power in menus and in the background.", Category.GENERAL, true, true);
        INSTANCE = this;
        cullDistance.visibleWhen(() -> cullEntities.on());
        backgroundFps.visibleWhen(() -> capBackground.on());
        menuFps.visibleWhen(() -> capMenus.on());
        boostDistance.visibleWhen(() -> boost.on());
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

    // ------------------------------------------------------------ FPS boost

    private static java.io.File boostFile() {
        return new java.io.File(Minecraft.getMinecraft().mcDataDir, "config/vesper/fps-boost-restore.properties");
    }

    @SubscribeEvent
    public void onClientTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END) return;
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.gameSettings == null || mc.renderGlobal == null) return;
        boolean want = enabled() && boost.on();
        java.io.File f = boostFile();
        if (want && !f.exists()) applyBoost(mc, f);
        else if (!want && f.exists()) restoreBoost(mc, f);
    }

    /** Saves your graphics settings, then switches to fast ones. */
    private void applyBoost(Minecraft mc, java.io.File f) {
        net.minecraft.client.settings.GameSettings g = mc.gameSettings;
        java.util.Properties p = new java.util.Properties();
        p.setProperty("fancyGraphics", String.valueOf(g.fancyGraphics));
        p.setProperty("ambientOcclusion", String.valueOf(g.ambientOcclusion));
        p.setProperty("clouds", String.valueOf(g.clouds));
        p.setProperty("particles", String.valueOf(g.particleSetting));
        p.setProperty("entityShadows", String.valueOf(g.entityShadows));
        p.setProperty("renderDistance", String.valueOf(g.renderDistanceChunks));
        p.setProperty("useVbo", String.valueOf(g.useVbo));
        p.setProperty("fboEnable", String.valueOf(g.fboEnable));
        p.setProperty("vsync", String.valueOf(g.enableVsync));
        try {
            f.getParentFile().mkdirs();
            java.io.FileOutputStream out = new java.io.FileOutputStream(f);
            p.store(out, "Your settings before Vesper's FPS boost; put back when it is turned off");
            out.close();
        } catch (java.io.IOException ex) {
            return;
        }
        g.fancyGraphics = false;
        g.ambientOcclusion = 0;
        g.clouds = 0;
        g.particleSetting = 2;
        g.entityShadows = false;
        g.renderDistanceChunks = Math.min(g.renderDistanceChunks, boostDistance.i());
        g.useVbo = true;
        g.fboEnable = true;
        g.enableVsync = false;
        Display.setVSyncEnabled(false);
        g.saveOptions();
        mc.renderGlobal.loadRenderers();
    }

    private void restoreBoost(Minecraft mc, java.io.File f) {
        net.minecraft.client.settings.GameSettings g = mc.gameSettings;
        java.util.Properties p = new java.util.Properties();
        try {
            java.io.FileInputStream in = new java.io.FileInputStream(f);
            p.load(in);
            in.close();
            g.fancyGraphics = Boolean.parseBoolean(p.getProperty("fancyGraphics", "true"));
            g.ambientOcclusion = Integer.parseInt(p.getProperty("ambientOcclusion", "2"));
            g.clouds = Integer.parseInt(p.getProperty("clouds", "2"));
            g.particleSetting = Integer.parseInt(p.getProperty("particles", "0"));
            g.entityShadows = Boolean.parseBoolean(p.getProperty("entityShadows", "true"));
            g.renderDistanceChunks = Integer.parseInt(p.getProperty("renderDistance", "12"));
            g.useVbo = Boolean.parseBoolean(p.getProperty("useVbo", "true"));
            g.fboEnable = Boolean.parseBoolean(p.getProperty("fboEnable", "true"));
            g.enableVsync = Boolean.parseBoolean(p.getProperty("vsync", "true"));
            Display.setVSyncEnabled(g.enableVsync);
            g.saveOptions();
            mc.renderGlobal.loadRenderers();
        } catch (Exception ex) {
            // A broken file: keep the current settings.
        }
        f.delete();
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
