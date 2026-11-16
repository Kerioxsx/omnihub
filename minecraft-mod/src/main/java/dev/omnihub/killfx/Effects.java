package dev.omnihub.killfx;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Random;
import net.minecraft.block.Block;
import net.minecraft.client.Minecraft;
import net.minecraft.entity.effect.EntityLightningBolt;
import net.minecraft.init.Blocks;
import net.minecraft.nbt.NBTTagCompound;
import net.minecraft.nbt.NBTTagList;
import net.minecraft.util.EnumParticleTypes;
import net.minecraft.world.World;

/** The kill effects, drawn on this client only. */
public final class Effects {
    static final String[] ALL = {"lightning", "blood", "flames", "firework", "souls", "hearts", "explosion"};

    private final Random rnd = new Random();
    /** Effects that play out over several ticks. */
    private final List<Running> running = new ArrayList<Running>();

    private static final class Running {
        final String kind;
        final double x, y, z, h;
        int age;

        Running(String kind, double x, double y, double z, double h) {
            this.kind = kind;
            this.x = x;
            this.y = y;
            this.z = z;
            this.h = h;
        }
    }

    public void play(Settings s, double x, double y, double z, double height) {
        World w = Minecraft.getMinecraft().theWorld;
        if (w == null) return;
        String kind = s.effect;
        if ("random".equals(kind)) kind = ALL[rnd.nextInt(ALL.length)];
        int n = s.intensity;
        String defaultSound;
        if ("lightning".equals(kind)) {
            w.addWeatherEffect(new EntityLightningBolt(w, x, y, z));
            defaultSound = null; // the bolt has its own thunder
        } else if ("blood".equals(kind)) {
            int red = Block.getStateId(Blocks.redstone_block.getDefaultState());
            for (int i = 0; i < 30 * n; i++) {
                w.spawnParticle(EnumParticleTypes.BLOCK_CRACK, x + gauss(0.3), y + rnd.nextDouble() * height, z + gauss(0.3), gauss(0.25), rnd.nextDouble() * 0.3, gauss(0.25), red);
            }
            for (int i = 0; i < 8 * n; i++) {
                w.spawnParticle(EnumParticleTypes.REDSTONE, x + gauss(0.4), y + rnd.nextDouble() * height, z + gauss(0.4), 0, 0, 0);
            }
            defaultSound = "dig.stone";
        } else if ("flames".equals(kind)) {
            running.add(new Running(kind, x, y, z, height));
            for (int i = 0; i < 6 * n; i++) {
                w.spawnParticle(EnumParticleTypes.LAVA, x + gauss(0.3), y + 0.2, z + gauss(0.3), 0, 0, 0);
            }
            defaultSound = "mob.ghast.fireball";
        } else if ("firework".equals(kind)) {
            w.makeFireworks(x, y + height + 0.5, z, 0, 0, 0, firework(s.color, n));
            defaultSound = null; // the firework makes its own
        } else if ("souls".equals(kind)) {
            running.add(new Running(kind, x, y, z, height));
            defaultSound = "mob.wither.shoot";
        } else if ("hearts".equals(kind)) {
            for (int i = 0; i < 6 * n; i++) {
                w.spawnParticle(EnumParticleTypes.HEART, x + gauss(0.5), y + height * 0.5 + rnd.nextDouble() * height, z + gauss(0.5), 0, 0.1, 0);
            }
            defaultSound = "random.levelup";
        } else {
            for (int i = 0; i < n; i++) {
                w.spawnParticle(EnumParticleTypes.EXPLOSION_LARGE, x + gauss(0.5), y + height * 0.5, z + gauss(0.5), 0, 0, 0);
            }
            for (int i = 0; i < 20 * n; i++) {
                w.spawnParticle(EnumParticleTypes.SMOKE_LARGE, x + gauss(0.5), y + rnd.nextDouble() * height, z + gauss(0.5), gauss(0.1), 0.05, gauss(0.1));
            }
            defaultSound = "random.explode";
        }
        String sound = soundName(s.sound, defaultSound);
        if (sound != null) w.playSound(x, y, z, sound, 1.0F, 0.9F + rnd.nextFloat() * 0.2F, false);
    }

    /** Called every client tick for the effects that play out over time. */
    public void tick() {
        World w = Minecraft.getMinecraft().theWorld;
        if (w == null) {
            running.clear();
            return;
        }
        Iterator<Running> it = running.iterator();
        while (it.hasNext()) {
            Running r = it.next();
            r.age++;
            if ("souls".equals(r.kind)) {
                // A spiral of spirits rising out of the body.
                for (int k = 0; k < 3; k++) {
                    double a = r.age * 0.45 + k * (Math.PI * 2 / 3);
                    double rad = 0.6 - r.age * 0.012;
                    double yy = r.y + r.age * 0.09;
                    w.spawnParticle(EnumParticleTypes.SPELL_WITCH, r.x + Math.cos(a) * rad, yy, r.z + Math.sin(a) * rad, 0, 0.02, 0);
                    if (r.age % 4 == 0) w.spawnParticle(EnumParticleTypes.CLOUD, r.x + Math.cos(a) * rad, yy, r.z + Math.sin(a) * rad, 0, 0.03, 0);
                }
                if (r.age > 40) it.remove();
            } else if ("flames".equals(r.kind)) {
                // A rising ring of fire.
                for (int k = 0; k < 10; k++) {
                    double a = k * (Math.PI * 2 / 10) + r.age * 0.2;
                    double rad = 0.4 + r.age * 0.04;
                    w.spawnParticle(EnumParticleTypes.FLAME, r.x + Math.cos(a) * rad, r.y + r.age * 0.06, r.z + Math.sin(a) * rad, 0, 0.01, 0);
                }
                if (r.age > 20) it.remove();
            } else {
                it.remove();
            }
        }
    }

    private double gauss(double spread) {
        return rnd.nextGaussian() * spread;
    }

    private static String soundName(String choice, String fallback) {
        if (choice == null || choice.equals("default")) return fallback;
        if (choice.equals("none")) return null;
        if (choice.equals("levelup")) return "random.levelup";
        if (choice.equals("orb")) return "random.orb";
        if (choice.equals("anvil")) return "random.anvil_land";
        if (choice.equals("explode")) return "random.explode";
        if (choice.equals("thunder")) return "ambient.weather.thunder";
        if (choice.equals("wither")) return "mob.wither.death";
        if (choice.equals("ding")) return "note.pling";
        return fallback;
    }

    private static NBTTagCompound firework(int color, int n) {
        NBTTagCompound burst = new NBTTagCompound();
        burst.setByte("Type", (byte) (n >= 3 ? 1 : 4)); // large ball / burst
        burst.setBoolean("Flicker", true);
        burst.setBoolean("Trail", n >= 2);
        burst.setIntArray("Colors", new int[] {color});
        burst.setIntArray("FadeColors", new int[] {0xFFFFFF});
        NBTTagList list = new NBTTagList();
        list.appendTag(burst);
        NBTTagCompound root = new NBTTagCompound();
        root.setTag("Explosions", list);
        return root;
    }
}
