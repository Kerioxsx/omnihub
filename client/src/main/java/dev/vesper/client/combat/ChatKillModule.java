package dev.vesper.client.combat;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import net.minecraft.client.Minecraft;
import net.minecraft.entity.Entity;
import net.minecraftforge.client.event.ClientChatReceivedEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;

/**
 * Opt-in, best-effort kill detection from server chat messages such as
 * "Alex was slain by You". Off by default. A message only counts when you
 * hit that player within the last 5 seconds; the event is labelled
 * BEST_EFFORT / SERVER_MESSAGE so effects can treat it accordingly.
 */
public final class ChatKillModule extends Module {
    /** The victim must have been hit by you this recently. */
    static final long HIT_WINDOW_MS = 5000;

    public ChatKillModule() {
        super("chatKills", "Kill messages from chat",
            "Best effort: plays kill effects when a server's chat says you killed a player you hit in the last 5 s. "
                + "Only common English wordings are understood; servers differ, so it can miss kills or (rarely) misread one.",
            Category.EFFECTS, false, false);
        MinecraftForge.EVENT_BUS.register(this);
    }

    @SubscribeEvent
    public void onChat(ClientChatReceivedEvent e) {
        if (!enabled() || e.message == null || e.type == 2) return; // 2 = action bar
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.thePlayer == null || mc.theWorld == null) return;
        String victim = KillMessageParser.victimKilledBy(e.message.getUnformattedText(), mc.thePlayer.getName());
        if (victim == null) return;
        Combat.Hit hit = Combat.recentHit(victim, HIT_WINDOW_MS);
        if (hit == null || !Combat.claimKill(hit.uuid)) return;
        // Victim's last known position: live if still loaded, else where you last hit it.
        double x = hit.x, y = hit.y, z = hit.z, ht = hit.height;
        Entity live = mc.theWorld.getEntityByID(hit.entityId);
        if (live != null) {
            x = live.posX;
            y = live.posY;
            z = live.posZ;
            ht = live.height;
        }
        CombatBus.post(new CombatVisualEvent(CombatVisualEvent.Type.PLAYER_KILL_CONFIRMED, CombatVisualEvent.Confidence.BEST_EFFORT, CombatVisualEvent.Source.SERVER_MESSAGE,
            mc.thePlayer.getUniqueID(), hit.uuid, hit.entityId, x, y, z, ht, hit.player));
    }
}
