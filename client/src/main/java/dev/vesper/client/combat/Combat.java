package dev.vesper.client.combat;

import dev.vesper.client.module.ModuleRegistry;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityLivingBase;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.event.entity.living.LivingDeathEvent;
import net.minecraftforge.event.entity.player.AttackEntityEvent;
import net.minecraftforge.event.world.WorldEvent;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.network.FMLNetworkEvent;

/**
 * Combat signal pipeline (owner: hud-pvp). Watches the game and posts
 * {@link CombatVisualEvent}s to {@link CombatBus}; it never changes
 * anything (no packets, aim, reach, timing or entity state).
 *
 * <p>Tiers (ADR-3):
 * <ul>
 * <li>HIT_CONFIRMED / CONFIRMED / LOCAL_ATTACK: you swung at an entity. "Confirmed" means the swing
 * happened on your client, not that the server accepted the hit.</li>
 * <li>PLAYER_KILL_CONFIRMED / CONFIRMED / SINGLEPLAYER: the integrated server's own death event credits you
 * (melee or your projectile).</li>
 * <li>ENTITY_DEATH / CONFIRMED / SINGLEPLAYER: any other death on the integrated server, in your dimension.</li>
 * <li>PLAYER_KILL_CONFIRMED / BEST_EFFORT / SERVER_MESSAGE: opt-in chat parser ({@link ChatKillModule}).</li>
 * </ul>
 * An entity disappearing, unloading or teleporting is never treated as a kill.
 */
public final class Combat {
    /** Duplicate kills of the same victim inside this window are dropped. */
    static final long KILL_DEBOUNCE_MS = 1000;

    /** What we last knew about something you hit. */
    static final class Hit {
        final int entityId;
        final UUID uuid;
        final boolean player;
        double x, y, z, height;
        long time;

        Hit(int entityId, UUID uuid, boolean player) {
            this.entityId = entityId;
            this.uuid = uuid;
            this.player = player;
        }
    }

    private static final Map<String, Hit> HITS = new LinkedHashMap<String, Hit>();
    private static final Map<UUID, Long> RECENT_KILLS = new LinkedHashMap<UUID, Long>();

    private static volatile int lastAttackId = -1;
    private static volatile long lastAttackTime;

    private Combat() {
    }

    public static void init() {
        ModuleRegistry.register(new ChatKillModule());
        MinecraftForge.EVENT_BUS.register(new ForgeEvents());
        FMLCommonHandler.instance().bus().register(new FmlEvents());
    }

    /** Client entity id you last swung at, or -1. */
    public static int lastAttackEntityId() {
        return lastAttackId;
    }

    /** System time of your last swing at an entity (0 = none). */
    public static long lastAttackTime() {
        return lastAttackTime;
    }

    /** Drops all per-world state (world unload / disconnect). Safe from any thread. */
    public static void reset() {
        synchronized (HITS) {
            HITS.clear();
        }
        synchronized (RECENT_KILLS) {
            RECENT_KILLS.clear();
        }
        lastAttackId = -1;
        lastAttackTime = 0;
    }

    /** The last hit on an entity with this name within `maxAgeMs`, or null. */
    static Hit recentHit(String name, long maxAgeMs) {
        synchronized (HITS) {
            Hit h = HITS.get(name.toLowerCase(Locale.ROOT));
            return h != null && System.currentTimeMillis() - h.time <= maxAgeMs ? h : null;
        }
    }

    /** False if a kill of this victim was already reported within the debounce window. Thread-safe. */
    static boolean claimKill(UUID victim) {
        if (victim == null) return true;
        long now = System.currentTimeMillis();
        synchronized (RECENT_KILLS) {
            Iterator<Map.Entry<UUID, Long>> it = RECENT_KILLS.entrySet().iterator();
            while (it.hasNext()) if (now - it.next().getValue() > KILL_DEBOUNCE_MS) it.remove();
            if (RECENT_KILLS.containsKey(victim)) return false;
            RECENT_KILLS.put(victim, now);
            return true;
        }
    }

    private static void remember(Entity t) {
        String key = t.getName().toLowerCase(Locale.ROOT);
        synchronized (HITS) {
            Hit h = HITS.remove(key); // re-insert so the newest is last
            if (h == null || h.entityId != t.getEntityId()) h = new Hit(t.getEntityId(), t.getUniqueID(), t instanceof EntityPlayer);
            h.x = t.posX;
            h.y = t.posY;
            h.z = t.posZ;
            h.height = t.height;
            h.time = System.currentTimeMillis();
            HITS.put(key, h);
            if (HITS.size() > 32) {
                Iterator<String> it = HITS.keySet().iterator();
                it.next();
                it.remove();
            }
        }
    }

    public static final class ForgeEvents {
        /** Your own swing (client side). In singleplayer it also fires for the server-side player; that one is ignored. */
        @SubscribeEvent
        public void onAttack(AttackEntityEvent e) {
            Minecraft mc = Minecraft.getMinecraft();
            if (e.entityPlayer == null || e.entityPlayer != mc.thePlayer || e.target == null) return;
            if (!e.entityPlayer.worldObj.isRemote) return;
            Entity t = e.target;
            lastAttackId = t.getEntityId();
            lastAttackTime = System.currentTimeMillis();
            remember(t);
            CombatBus.post(new CombatVisualEvent(CombatVisualEvent.Type.HIT_CONFIRMED, CombatVisualEvent.Confidence.CONFIRMED, CombatVisualEvent.Source.LOCAL_ATTACK,
                e.entityPlayer.getUniqueID(), t.getUniqueID(), t.getEntityId(), t.posX, t.posY, t.posZ, t.height, t instanceof EntityPlayer));
        }

        /**
         * Deaths on the integrated server (singleplayer / LAN host). Runs on
         * the server thread: only primitives are copied, then posted.
         */
        @SubscribeEvent
        public void onDeath(LivingDeathEvent e) {
            EntityLivingBase dead = e.entityLiving;
            if (dead == null || dead.worldObj == null || dead.worldObj.isRemote) return;
            Minecraft mc = Minecraft.getMinecraft();
            if (!mc.isIntegratedServerRunning()) return;
            EntityPlayerSP me = mc.thePlayer;
            if (me == null) return;
            UUID myId = me.getUniqueID();
            // Only deaths in the dimension you are in (effects play at the body).
            if (dead.worldObj.provider == null || dead.worldObj.provider.getDimensionId() != me.dimension) return;
            Entity killer = e.source == null ? null : e.source.getEntity(); // the shooter for projectiles
            UUID victim = dead.getUniqueID();
            boolean mine = killer instanceof EntityPlayer && myId != null && myId.equals(killer.getUniqueID()) && !myId.equals(victim);
            boolean isPlayer = dead instanceof EntityPlayer;
            if (mine) {
                if (!claimKill(victim)) return;
                CombatBus.post(new CombatVisualEvent(CombatVisualEvent.Type.PLAYER_KILL_CONFIRMED, CombatVisualEvent.Confidence.CONFIRMED, CombatVisualEvent.Source.SINGLEPLAYER,
                    myId, victim, dead.getEntityId(), dead.posX, dead.posY, dead.posZ, dead.height, isPlayer));
            } else {
                CombatBus.post(new CombatVisualEvent(CombatVisualEvent.Type.ENTITY_DEATH, CombatVisualEvent.Confidence.CONFIRMED, CombatVisualEvent.Source.SINGLEPLAYER,
                    killer == null ? null : killer.getUniqueID(), victim, dead.getEntityId(), dead.posX, dead.posY, dead.posZ, dead.height, isPlayer));
            }
        }

        @SubscribeEvent
        public void onWorldUnload(WorldEvent.Unload e) {
            if (e.world.isRemote) reset();
        }
    }

    public static final class FmlEvents {
        @SubscribeEvent
        public void onDisconnect(FMLNetworkEvent.ClientDisconnectionFromServerEvent e) {
            reset();
        }
    }
}
