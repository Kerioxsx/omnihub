package dev.vesper.client.world;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.Rng;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.block.Block;
import net.minecraft.block.material.Material;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.multiplayer.WorldClient;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraft.util.BlockPos;
import net.minecraft.util.Vec3;
import net.minecraftforge.client.event.RenderWorldLastEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * Rain and wet ground, drawn by Vesper on your screen only (the world's
 * real weather is unchanged): falling streaks that stop at roofs and splash
 * where they land, a rain sound, and puddles on the ground around you that
 * mirror the sky's colour, shine more at shallow angles and ripple in rain.
 */
public final class WeatherModule extends Module {
    public final ChoiceSetting rain = choice("rain", "Rain", "always", new String[] {"off", "raining", "always"}, new String[] {"Off", "Only when it rains", "Always"});
    public final NumberSetting amount = number("amount", "Rain amount", 0.6, 0.1, 1, 0.05, "×");
    public final NumberSetting wind = number("wind", "Wind", 0.15, -1, 1, 0.05, "");
    public final BoolSetting sound = bool("sound", "Rain sound", true);
    public final ChoiceSetting wet = choice("wet", "Wet ground", "always", new String[] {"off", "raining", "always"}, new String[] {"Off", "Only when it rains", "Always"});
    public final NumberSetting puddles = number("puddles", "Puddles", 0.35, 0.05, 0.9, 0.05, "");
    public final NumberSetting shine = number("shine", "Reflection", 0.7, 0.1, 1, 0.05, "");

    private static final int MAX_DROPS = 900;
    private static final float[] DX = new float[MAX_DROPS], DZ = new float[MAX_DROPS], DP = new float[MAX_DROPS], DS = new float[MAX_DROPS];
    private final Map<Long, Integer> heights = new HashMap<Long, Integer>();
    private int heightAge;
    private float soundTimer;

    // Puddles around the player, rebuilt when they move.
    private double[] pud = new double[0];
    private int pudCount;
    private double builtX = 1e9, builtZ = 1e9;
    private int builtAge;

    static {
        Rng r = new Rng(0x5A1L);
        for (int i = 0; i < MAX_DROPS; i++) {
            DX[i] = r.f();
            DZ[i] = r.f();
            DP[i] = r.f();
            DS[i] = r.range(0.85f, 1.15f);
        }
    }

    public WeatherModule() {
        super("weather", "Rain & wet ground", "Rain that splashes and stops at roofs, a rain sound, and puddles that mirror the sky (only on your screen).", Category.VISUALS, false, true);
    }

    private boolean raining(WorldClient w) {
        return w.getRainStrength(1f) > 0.2f;
    }

    private boolean rainOn(WorldClient w) {
        return rain.is("always") || (rain.is("raining") && raining(w));
    }

    private int ground(WorldClient w, int x, int z) {
        long k = ((long) x << 32) ^ (z & 0xFFFFFFFFL);
        Integer h = heights.get(k);
        if (h == null) {
            h = w.getPrecipitationHeight(new BlockPos(x, 0, z)).getY();
            heights.put(k, h);
        }
        return h;
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END) return;
        Minecraft mc = Minecraft.getMinecraft();
        WorldClient w = mc.theWorld;
        EntityPlayerSP p = mc.thePlayer;
        if (!enabled() || w == null || p == null) return;
        if (++heightAge > 40 || heights.size() > 6000) {
            heights.clear();
            heightAge = 0;
        }
        // The game plays its own rain sound when it really rains.
        if (sound.on() && rain.is("always") && !raining(w) && mc.currentScreen == null) {
            soundTimer -= 0.05f;
            if (soundTimer <= 0) {
                soundTimer = 0.8f + (float) Math.random() * 0.4f;
                boolean covered = ground(w, (int) Math.floor(p.posX), (int) Math.floor(p.posZ)) > p.posY + 1;
                w.playSound(p.posX, p.posY + 4, p.posZ, covered ? "ambient.weather.rain.above" : "ambient.weather.rain", 0.12f * amount.f(), covered ? 0.5f : 1f, false);
            }
        }
    }

    @SubscribeEvent
    public void onWorld(RenderWorldLastEvent e) {
        Minecraft mc = Minecraft.getMinecraft();
        WorldClient w = mc.theWorld;
        EntityPlayerSP p = mc.thePlayer;
        if (!enabled() || w == null || p == null || w.provider.getDimensionId() != 0) return;
        boolean drawRain = rainOn(w) && General.fx();
        boolean drawWet = wet.is("always") || (wet.is("raining") && (raining(w) || rain.is("always")));
        if (!drawRain && !drawWet) return;
        RenderManager rm = mc.getRenderManager();
        Gl.beginGlow();
        View.capture();
        Gl.solidBlend();
        if (drawWet) puddles(w, p, rm, e.partialTicks, drawRain);
        if (drawRain) rain(w, rm, drawWet);
        Gl.additive();
        Gl.endGlow();
    }

    private void rain(WorldClient w, RenderManager rm, boolean onWet) {
        double cx = rm.viewerPosX, cy = rm.viewerPosY, cz = rm.viewerPosZ;
        float t = Ease.seconds();
        int n = Math.round(MAX_DROPS * amount.f() * (General.detail() <= 0 ? 0.35f : General.detail() == 1 ? 0.6f : 1f));
        double box = 26, speed = 17, len = 0.7;
        double wx = wind.f() * 3;
        int body = 0xFFC9D8EA;
        FxDraw.geo();
        for (int i = 0; i < n; i++) {
            double x = DX[i] * box, z = DZ[i] * box;
            x = cx + (((x - cx) % box) + box * 1.5) % box - box / 2;
            z = cz + (((z - cz) % box) + box * 1.5) % box - box / 2;
            int gy = ground(w, (int) Math.floor(x), (int) Math.floor(z));
            double top = Math.max(cy + 14, gy + 2);
            double span = top - gy;
            double fall = (t * speed * DS[i] + DP[i] * span) % span;
            double y = top - fall;
            double since = fall / (speed * DS[i]);
            double ox = x - cx, oz = z - cz;
            if (y > gy + 0.05) {
                double l = len * DS[i];
                FxDraw.line(ox - wx * 0.05, y - cy, oz, ox + wx * 0.05, y - cy + l, oz, 0.022, Colors.alpha(body, 0.45f), Colors.alpha(body, 0));
            }
            // A splash just after the previous drop on this column landed.
            if (since < 0.18 && gy < cy + 6 && gy > cy - 12) {
                float k = (float) (since / 0.18);
                double gyR = gy - cy + 0.03;
                FxDraw.ringH(ox, gyR, oz, 0.03 + 0.18 * k, 0.05 + 0.2 * k, Colors.alpha(body, 0), Colors.alpha(body, 0.5f * (1 - k)), 10);
                if (k < 0.6f) FxDraw.line(ox, gyR, oz, ox + (DX[(i + 7) % MAX_DROPS] - 0.5) * 0.25, gyR + 0.15 * (1 - k), oz + (DZ[(i + 7) % MAX_DROPS] - 0.5) * 0.25, 0.018, Colors.alpha(body, 0.4f * (1 - k)));
            }
        }
        FxDraw.flush();
    }

    /** Finds puddle spots: open tops of solid ground near the player, picked by a stable hash. */
    private void rebuild(WorldClient w, EntityPlayerSP p) {
        int R = General.detail() <= 0 ? 8 : 13;
        int px = (int) Math.floor(p.posX), pz = (int) Math.floor(p.posZ);
        double dens = puddles.f();
        double[] out = new double[(2 * R + 1) * (2 * R + 1) * 4];
        int n = 0;
        for (int x = px - R; x <= px + R; x++) {
            for (int z = pz - R; z <= pz + R; z++) {
                if ((x - px) * (x - px) + (z - pz) * (z - pz) > R * R) continue;
                long h = Rng.hash(x * 73856093L, z * 19349663L);
                double v = ((h >>> 11) & 0xFFFF) / 65535.0;
                // Low-frequency mask so puddles gather in patches, not a checkerboard.
                double patch = 0.5 + 0.5 * Math.sin(x * 0.37 + Math.sin(z * 0.23) * 2) * Math.cos(z * 0.31 + x * 0.11);
                if (v > dens * (0.4 + 1.2 * patch)) continue;
                int y = ground(w, x, z);
                if (Math.abs(y - p.posY) > 8) continue;
                BlockPos below = new BlockPos(x, y - 1, z);
                Block b = w.getBlockState(below).getBlock();
                Material m = b.getMaterial();
                if (!b.isFullCube() || m == Material.water || m == Material.lava || m == Material.leaves || m == Material.ice || m == Material.glass) continue;
                if (!w.isAirBlock(new BlockPos(x, y, z))) continue;
                double r = 0.32 + (((h >>> 30) & 0xFF) / 255.0) * 0.3 + patch * 0.15;
                out[n * 4] = x + 0.5 + (((h >>> 40) & 0xFF) / 255.0 - 0.5) * 0.3;
                out[n * 4 + 1] = y;
                out[n * 4 + 2] = z + 0.5 + (((h >>> 48) & 0xFF) / 255.0 - 0.5) * 0.3;
                out[n * 4 + 3] = r;
                n++;
            }
        }
        pud = out;
        pudCount = n;
        builtX = p.posX;
        builtZ = p.posZ;
        builtAge = 0;
    }

    private void puddles(WorldClient w, EntityPlayerSP p, RenderManager rm, float pt, boolean ripples) {
        if (++builtAge > 120 || Math.abs(p.posX - builtX) > 1.5 || Math.abs(p.posZ - builtZ) > 1.5) rebuild(w, p);
        if (pudCount == 0) return;
        Vec3 sky = w.getSkyColor(p, pt);
        int skyC = Colors.argb(1, (float) Math.min(1, sky.xCoord), (float) Math.min(1, sky.yCoord), (float) Math.min(1, sky.zCoord));
        int deep = Colors.mix(skyC, 0xFF0B1220, 0.72f);
        float t = Ease.seconds();
        float s = shine.f();
        GlStateManager.enableDepth();
        FxDraw.geo();
        for (int i = 0; i < pudCount; i++) {
            double x = pud[i * 4] - rm.viewerPosX, y = pud[i * 4 + 1] - rm.viewerPosY + 0.004, z = pud[i * 4 + 2] - rm.viewerPosZ, r = pud[i * 4 + 3];
            double d2 = x * x + y * y + z * z;
            if (d2 > 260) continue;
            // Fresnel: the shallower you look across the water, the more it mirrors.
            double dist = Math.sqrt(d2);
            float grazing = (float) (1 - Math.abs(y) / Math.max(0.01, dist));
            float fres = Ease.clamp01(0.25f + 0.85f * grazing * grazing);
            float fade = Ease.clamp01((float) (1 - (dist - 10) / 6));
            int mirror = Colors.mix(deep, Colors.mix(skyC, 0xFF1A2638, 0.35f), fres);
            float a = (0.22f + 0.38f * fres) * s * fade;
            FxDraw.ringH(x, y, z, 0, r * 0.78, Colors.alpha(mirror, a), Colors.alpha(mirror, a));
            FxDraw.ringH(x, y, z, r * 0.78, r, Colors.alpha(mirror, a), Colors.alpha(mirror, 0));
            // A soft moving highlight.
            double hx = x + Math.sin(t * 0.5 + i) * r * 0.25, hz = z + Math.cos(t * 0.4 + i * 1.3) * r * 0.25;
            FxDraw.ringH(hx, y + 0.001, hz, 0, r * 0.28, Colors.alpha(0xFFFFFFFF, 0.12f * fres * s * fade), Colors.alpha(0xFFFFFFFF, 0));
            if (ripples) {
                for (int k = 0; k < 2; k++) {
                    double slot = Math.floor(t * 1.6 + i * 0.37 + k * 0.5);
                    double ph = t * 1.6 + i * 0.37 + k * 0.5 - slot;
                    long hh = Rng.hash((long) slot, i * 31L + k);
                    double rx = x + (((hh >>> 8) & 0xFF) / 255.0 - 0.5) * r, rz = z + (((hh >>> 16) & 0xFF) / 255.0 - 0.5) * r;
                    double rr = 0.04 + ph * 0.25;
                    FxDraw.ringH(rx, y + 0.002, rz, rr, rr + 0.02, Colors.alpha(0xFFE6F0FF, 0), Colors.alpha(0xFFE6F0FF, (float) (0.35 * (1 - ph)) * fade));
                }
            }
        }
        FxDraw.flush();
    }
}
