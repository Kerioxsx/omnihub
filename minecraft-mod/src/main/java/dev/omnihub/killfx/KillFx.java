package dev.omnihub.killfx;

import java.io.File;
import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityLivingBase;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraftforge.client.event.ClientChatReceivedEvent;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.event.entity.player.AttackEntityEvent;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.event.FMLInitializationEvent;
import net.minecraftforge.fml.common.event.FMLPreInitializationEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * Kill effects for 1.8.9, client side, so they work on any server.
 *
 * A kill is something you hit in the last few seconds that then dies: its
 * health reaches zero, it starts dying, or (servers that remove dead
 * players at once) it vanishes right after a hit that left it low, or the
 * chat says it was killed by you.
 */
@Mod(modid = "omnihubkillfx", name = "OmniHub Kill Effects", version = "1.0.0", acceptedMinecraftVersions = "[1.8.9]", acceptableRemoteVersions = "*")
public class KillFx {
    /** How long after a hit a death still counts as yours, in ticks. */
    private static final int WINDOW = 80;

    private Settings settings;
    private final Effects effects = new Effects();
    private final Map<Integer, Hit> hits = new HashMap<Integer, Hit>();
    private int tick;
    private int markUntil;
    private int streak;
    private int streakUntil;

    private static final class Hit {
        int at;
        double x, y, z, height;
        float health = Float.MAX_VALUE;
        String name;
    }

    @Mod.EventHandler
    public void preInit(FMLPreInitializationEvent e) {
        settings = new Settings(new File(e.getModConfigurationDirectory(), "omnihub-killfx.properties"));
        settings.refresh();
    }

    @Mod.EventHandler
    public void init(FMLInitializationEvent e) {
        MinecraftForge.EVENT_BUS.register(this);
        FMLCommonHandler.instance().bus().register(this);
    }

    @SubscribeEvent
    public void onAttack(AttackEntityEvent e) {
        Minecraft mc = Minecraft.getMinecraft();
        if (e.entityPlayer != mc.thePlayer || !(e.target instanceof EntityLivingBase)) return;
        if (settings.playersOnly && !(e.target instanceof EntityPlayer)) return;
        Hit h = hits.get(e.target.getEntityId());
        if (h == null) {
            h = new Hit();
            hits.put(e.target.getEntityId(), h);
        }
        h.at = tick;
        h.name = e.target instanceof EntityPlayer ? e.target.getName() : null;
        remember(h, (EntityLivingBase) e.target);
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END) return;
        tick++;
        if (tick % 40 == 0) settings.refresh();
        effects.tick();
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.theWorld == null) {
            hits.clear();
            return;
        }
        Iterator<Map.Entry<Integer, Hit>> it = hits.entrySet().iterator();
        while (it.hasNext()) {
            Map.Entry<Integer, Hit> en = it.next();
            Hit h = en.getValue();
            if (tick - h.at > WINDOW) {
                it.remove();
                continue;
            }
            Entity ent = mc.theWorld.getEntityByID(en.getKey());
            if (ent instanceof EntityLivingBase) {
                EntityLivingBase l = (EntityLivingBase) ent;
                if (l.getHealth() <= 0 || l.deathTime > 0 || l.isDead) {
                    kill(h);
                    it.remove();
                } else {
                    remember(h, l);
                }
            } else if (ent == null) {
                // Gone: a kill if it was low and we hit it just now.
                if (tick - h.at <= 20 && h.health <= 8) kill(h);
                it.remove();
            }
        }
    }

    @SubscribeEvent
    public void onChat(ClientChatReceivedEvent e) {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.thePlayer == null || e.message == null) return;
        String text = e.message.getUnformattedText();
        String me = mc.thePlayer.getName();
        if (!text.contains(me) || text.startsWith(me)) return;
        for (Iterator<Map.Entry<Integer, Hit>> it = hits.entrySet().iterator(); it.hasNext(); ) {
            Hit h = it.next().getValue();
            // "<victim> was killed by <you>" and the like.
            if (h.name != null && text.startsWith(h.name + " ") && text.indexOf(me) > h.name.length()) {
                kill(h);
                it.remove();
                return;
            }
        }
    }

    @SubscribeEvent
    public void onOverlay(RenderGameOverlayEvent.Post e) {
        if (e.type != RenderGameOverlayEvent.ElementType.ALL || !settings.killMark || tick > markUntil) return;
        Minecraft mc = Minecraft.getMinecraft();
        ScaledResolution sr = new ScaledResolution(mc);
        String text = streak > 1 ? "☠ Kill x" + streak : "☠ Kill";
        int left = markUntil - tick;
        int alpha = Math.max(16, Math.min(255, left * 16));
        int rgb = settings.color & 0xFFFFFF;
        mc.fontRendererObj.drawStringWithShadow(text, sr.getScaledWidth() / 2f - mc.fontRendererObj.getStringWidth(text) / 2f, sr.getScaledHeight() / 2f + 12, (alpha << 24) | rgb);
    }

    private static void remember(Hit h, EntityLivingBase l) {
        h.x = l.posX;
        h.y = l.posY;
        h.z = l.posZ;
        h.height = l.height;
        h.health = l.getHealth();
    }

    private void kill(Hit h) {
        if (!settings.enabled) return;
        effects.play(settings, h.x, h.y, h.z, h.height);
        streak = tick <= streakUntil ? streak + 1 : 1;
        streakUntil = tick + 200;
        markUntil = tick + 30;
    }
}
