package dev.vesper.client.hud;

import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.qol.Utilities;
import java.text.SimpleDateFormat;
import java.util.Date;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.entity.Entity;
import net.minecraft.init.Items;
import net.minecraft.item.ItemBlock;
import net.minecraft.item.ItemPotion;
import net.minecraft.item.ItemStack;
import net.minecraft.util.BlockPos;
import net.minecraft.util.EnumFacing;
import net.minecraft.util.MathHelper;
import net.minecraft.util.MovingObjectPosition;
import net.minecraft.world.EnumSkyBlock;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * One-line HUD readouts, each its own element you turn on and place in the
 * HUD editor: where you are, how you move, your gear, the world, the
 * server, and what you did this session. All read from what the game
 * already shows you; none of them change anything.
 */
public final class InfoElements {
    private InfoElements() {
    }

    interface Src {
        String get(Minecraft mc, EntityPlayerSP p);
    }

    static final class Info extends LineElement {
        private final String lab, sampleValue;
        private final Src src;

        Info(String id, String name, String desc, String label, String sample, int i, Src src) {
            super("info_" + id, name, desc, false, i % 2 == 0 ? 0.01f : 0.99f, 0.18f + (i / 2 % 16) * 0.045f);
            this.lab = label;
            this.sampleValue = sample;
            this.src = src;
        }

        @Override
        public boolean hasContent() {
            return true;
        }

        @Override
        protected void compute(boolean sample) {
            label = lab;
            if (sample || mc.thePlayer == null || mc.theWorld == null) {
                value = sampleValue;
                return;
            }
            try {
                String v = src.get(mc, mc.thePlayer);
                value = v == null ? "—" : v;
            } catch (RuntimeException e) {
                value = "—";
            }
        }
    }

    // ---------- this session (since you joined the world) ----------

    static int kills, hits, combo, bestCombo, jumps;
    static double walked;
    private static double lastX = Double.NaN, lastZ;
    private static boolean wasOnGround = true;
    private static int lastHurt;
    private static double speed;
    private static final double[] SPEEDS = new double[10];
    private static int speedIdx;

    public static final class Tracker {
        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            if (e.phase != TickEvent.Phase.END) return;
            EntityPlayerSP p = Minecraft.getMinecraft().thePlayer;
            if (p == null) {
                kills = hits = combo = jumps = 0;
                walked = 0;
                lastX = Double.NaN;
                return;
            }
            if (!Double.isNaN(lastX)) {
                double d = Math.hypot(p.posX - lastX, p.posZ - lastZ);
                if (d < 10) {
                    walked += d;
                    SPEEDS[speedIdx++ % SPEEDS.length] = d * 20;
                    double s = 0;
                    for (double v : SPEEDS) s += v;
                    speed = s / SPEEDS.length;
                }
            }
            lastX = p.posX;
            lastZ = p.posZ;
            if (wasOnGround && !p.onGround && p.motionY > 0.2) jumps++;
            wasOnGround = p.onGround;
            if (p.hurtTime > lastHurt && p.hurtTime == p.maxHurtTime) combo = 0;
            lastHurt = p.hurtTime;
        }
    }

    public static void init() {
        CombatBus.listen(new CombatBus.Listener() {
            public void on(CombatVisualEvent e) {
                if (e.type == CombatVisualEvent.Type.HIT_CONFIRMED) {
                    hits++;
                    combo++;
                    bestCombo = Math.max(bestCombo, combo);
                } else if (e.type == CombatVisualEvent.Type.PLAYER_KILL_CONFIRMED) {
                    kills++;
                }
            }
        });
        Tracker tracker = new Tracker();
        MinecraftForge.EVENT_BUS.register(tracker);
        FMLCommonHandler.instance().bus().register(tracker);
        int i = 0;
        Info[] all = {
            // Where you are.
            new Info("biome", "Biome", "The biome you are standing in.", "Biome", "Plains", i++, (mc, p) -> mc.theWorld.getBiomeGenForCoords(new BlockPos(p)).biomeName),
            new Info("compass", "Compass", "The direction you face, with the exact angle.", "", "N 180°", i++, (mc, p) -> {
                float yaw = ((p.rotationYaw % 360) + 360) % 360;
                String[] d = {"S", "SW", "W", "NW", "N", "NE", "E", "SE"};
                return d[MathHelper.floor_double(yaw / 45 + 0.5) % 8] + " " + Math.round(yaw) + "°";
            }),
            new Info("altitude", "Altitude", "Your height (Y).", "Y", "64", i++, (mc, p) -> String.valueOf(MathHelper.floor_double(p.posY))),
            new Info("chunk", "Chunk", "The chunk you are in and your spot inside it.", "Chunk", "8, -16 (4, 11)", i++, (mc, p) -> (MathHelper.floor_double(p.posX) >> 4) + ", " + (MathHelper.floor_double(p.posZ) >> 4) + " (" + (MathHelper.floor_double(p.posX) & 15) + ", " + (MathHelper.floor_double(p.posZ) & 15) + ")"),
            new Info("light", "Light level", "Block light where you stand (0 to 15).", "Light", "12", i++, (mc, p) -> String.valueOf(mc.theWorld.getLightFor(EnumSkyBlock.BLOCK, new BlockPos(p)))),
            new Info("dimension", "Dimension", "Overworld, Nether or End.", "", "Overworld", i++, (mc, p) -> p.dimension == -1 ? "Nether" : p.dimension == 1 ? "The End" : "Overworld"),
            new Info("rotation", "Yaw & pitch", "Where you look, in degrees.", "Look", "180.0 / 12.5", i++, (mc, p) -> String.format("%.1f / %.1f", MathHelper.wrapAngleTo180_float(p.rotationYaw), p.rotationPitch)),
            new Info("lookblock", "Looking at", "The block or entity under your crosshair.", "", "Oak Planks", i++, (mc, p) -> {
                MovingObjectPosition o = mc.objectMouseOver;
                if (o == null) return "Nothing";
                if (o.typeOfHit == MovingObjectPosition.MovingObjectType.ENTITY && o.entityHit != null) return o.entityHit.getName();
                if (o.typeOfHit == MovingObjectPosition.MovingObjectType.BLOCK) return mc.theWorld.getBlockState(o.getBlockPos()).getBlock().getLocalizedName();
                return "Nothing";
            }),
            new Info("facing", "Block face", "Which side of the block you look at.", "Face", "Up", i++, (mc, p) -> {
                MovingObjectPosition o = mc.objectMouseOver;
                if (o == null || o.typeOfHit != MovingObjectPosition.MovingObjectType.BLOCK || o.sideHit == null) return "—";
                EnumFacing f = o.sideHit;
                String n = f.getName();
                return Character.toUpperCase(n.charAt(0)) + n.substring(1);
            }),
            // How you move.
            new Info("speed", "Speed", "How fast you move (blocks per second).", "Speed", "5.6 b/s", i++, (mc, p) -> String.format("%.1f b/s", speed)),
            new Info("sprint", "Sprint status", "Walking, sprinting, sneaking or toggled sprint.", "", "Sprinting (toggled)", i++, (mc, p) -> {
                String t = Utilities.ToggleSprint.INSTANCE != null && Utilities.ToggleSprint.INSTANCE.sprintToggled() ? " (toggled)" : "";
                if (p.isSneaking()) return "Sneaking" + (Utilities.ToggleSprint.INSTANCE != null && Utilities.ToggleSprint.INSTANCE.sneakToggled() ? " (toggled)" : "");
                if (p.isSprinting()) return "Sprinting" + t;
                return p.moveForward != 0 || p.moveStrafing != 0 ? "Walking" + t : "Standing" + t;
            }),
            new Info("distance", "Distance walked", "How far you have walked since you joined.", "Walked", "1,284 m", i++, (mc, p) -> String.format("%,d m", Math.round(walked))),
            new Info("jumps", "Jumps", "Jumps since you joined.", "Jumps", "42", i++, (mc, p) -> String.valueOf(jumps)),
            new Info("velocity", "Fall speed", "How fast you are going up or down.", "Vertical", "-0.42", i++, (mc, p) -> String.format("%+.2f", p.motionY)),
            // You and your gear.
            new Info("health", "Health", "Your health in hearts.", "Health", "10.0 hearts", i++, (mc, p) -> String.format("%.1f", p.getHealth() / 2f) + " hearts"),
            new Info("food", "Food", "Your hunger bar (out of 10).", "Food", "8.5", i++, (mc, p) -> String.format("%.1f", p.getFoodStats().getFoodLevel() / 2f)),
            new Info("saturation", "Saturation", "Hidden saturation that stops hunger dropping.", "Saturation", "4.2", i++, (mc, p) -> String.format("%.1f", p.getFoodStats().getSaturationLevel())),
            new Info("armorpoints", "Armor points", "Your armor value (out of 20).", "Armor", "15", i++, (mc, p) -> String.valueOf(p.getTotalArmorValue())),
            new Info("xp", "Experience", "Your level and progress to the next.", "Level", "12 (48%)", i++, (mc, p) -> p.experienceLevel + " (" + Math.round(p.experience * 100) + "%)"),
            new Info("durability", "Held item durability", "Uses left on what you hold.", "Durability", "1,204 / 1,561", i++, (mc, p) -> {
                ItemStack s = p.getHeldItem();
                if (s == null || !s.isItemStackDamageable()) return "—";
                return String.format("%,d / %,d", s.getMaxDamage() - s.getItemDamage(), s.getMaxDamage());
            }),
            new Info("arrows", "Arrows", "Arrows in your inventory.", "Arrows", "64", i++, (mc, p) -> String.valueOf(count(p, s -> s.getItem() == Items.arrow))),
            new Info("potions", "Splash potions", "Splash potions in your inventory.", "Pots", "8", i++, (mc, p) -> String.valueOf(count(p, s -> s.getItem() instanceof ItemPotion && ItemPotion.isSplash(s.getMetadata())))),
            new Info("blocks", "Blocks in hand", "How many of the block you hold you have in total.", "Blocks", "128", i++, (mc, p) -> {
                ItemStack h = p.getHeldItem();
                if (h == null || !(h.getItem() instanceof ItemBlock)) return "—";
                return String.valueOf(count(p, s -> s.getItem() == h.getItem() && s.getMetadata() == h.getMetadata()));
            }),
            new Info("gapples", "Golden apples", "Golden apples in your inventory.", "Gapples", "6", i++, (mc, p) -> String.valueOf(count(p, s -> s.getItem() == Items.golden_apple))),
            new Info("pearls", "Ender pearls", "Ender pearls in your inventory.", "Pearls", "3", i++, (mc, p) -> String.valueOf(count(p, s -> s.getItem() == Items.ender_pearl))),
            // The world.
            new Info("gametime", "In-game time", "The time of day in the world.", "Time", "14:30", i++, (mc, p) -> {
                long t = (mc.theWorld.getWorldTime() + 6000) % 24000;
                return String.format("%02d:%02d", t / 1000, (t % 1000) * 60 / 1000);
            }),
            new Info("day", "Day", "Which in-game day it is.", "Day", "37", i++, (mc, p) -> String.valueOf(mc.theWorld.getWorldTime() / 24000 + 1)),
            new Info("moon", "Moon phase", "Tonight's moon.", "Moon", "Full moon", i++, (mc, p) -> new String[] {"Full moon", "Waning gibbous", "Last quarter", "Waning crescent", "New moon", "Waxing crescent", "First quarter", "Waxing gibbous"}[mc.theWorld.getMoonPhase() % 8]),
            new Info("weather", "Weather", "Clear, rain or thunder.", "Weather", "Rain", i++, (mc, p) -> mc.theWorld.isThundering() ? "Thunder" : mc.theWorld.isRaining() ? "Rain" : "Clear"),
            new Info("entities", "Entities", "How many entities are loaded around you.", "Entities", "48", i++, (mc, p) -> String.valueOf(mc.theWorld.loadedEntityList.size())),
            // The server and your PC.
            new Info("server", "Server", "The server you are on.", "", "mc.example.net", i++, (mc, p) -> mc.getCurrentServerData() == null ? "Singleplayer" : mc.getCurrentServerData().serverIP),
            new Info("players", "Players", "Players in the tab list.", "Players", "24", i++, (mc, p) -> p.sendQueue == null ? "—" : String.valueOf(p.sendQueue.getPlayerInfoMap().size())),
            new Info("memory", "Memory", "Memory Minecraft is using.", "Memory", "812 / 2048 MB", i++, (mc, p) -> {
                Runtime r = Runtime.getRuntime();
                return ((r.totalMemory() - r.freeMemory()) >> 20) + " / " + (r.maxMemory() >> 20) + " MB";
            }),
            new Info("clock", "Clock", "The time on your PC.", "", "21:47", i++, (mc, p) -> new SimpleDateFormat("HH:mm").format(new Date())),
            new Info("date", "Date", "Today's date.", "", "Sat 14 Nov", i++, (mc, p) -> new SimpleDateFormat("EEE d MMM").format(new Date())),
            // This session.
            new Info("kills", "Kills", "Kills credited to you since you joined.", "Kills", "7", i++, (mc, p) -> String.valueOf(kills)),
            new Info("hits", "Hits", "Hits you landed since you joined.", "Hits", "152", i++, (mc, p) -> String.valueOf(hits)),
            new Info("combo", "Combo", "Hits in a row without being hit back (best this session).", "Combo", "4 (best 9)", i++, (mc, p) -> combo + " (best " + bestCombo + ")"),
            new Info("target", "Target distance", "How far the player or mob under your crosshair is.", "Target", "2.84 m", i++, (mc, p) -> {
                Entity t = mc.objectMouseOver == null ? null : mc.objectMouseOver.entityHit;
                return t == null ? "—" : String.format("%.2f m", t.getDistanceToEntity(p));
            }),
        };
        for (Info in : all) Hud.addElement(in);
    }

    interface Match {
        boolean is(ItemStack s);
    }

    private static int count(EntityPlayerSP p, Match m) {
        int n = 0;
        for (ItemStack s : p.inventory.mainInventory) if (s != null && m.is(s)) n += s.stackSize;
        return n;
    }

    /** How many readouts there are (for the docs). */
    public static int count() {
        int n = 0;
        for (dev.vesper.client.module.Module m : ModuleRegistry.all()) if (m.id.startsWith("info_")) n++;
        return n;
    }
}
