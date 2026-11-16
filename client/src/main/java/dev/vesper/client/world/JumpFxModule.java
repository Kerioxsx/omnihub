package dev.vesper.client.world;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.Particles;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraftforge.client.event.RenderWorldLastEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * Every jump leaves a neon mark on the floor where you took off (a
 * six-point star by default, or a ring, rune circle, triangle or diamond),
 * in your world colour; landings can send out a shockwave ring. Only you
 * see it.
 */
public final class JumpFxModule extends Module {
    public final ChoiceSetting shape = choice("shape", "Shape", "star6", new String[] {"star6", "runes", "ring", "triangle", "diamond"},
        new String[] {"Six-point star", "Rune circle", "Ring", "Triangle", "Diamond"});
    public final BoolSetting useWorld = bool("useWorld", "Use the world colour", true);
    public final ColorSetting color = color("color", "Colour (when not the world colour)", 0xFFB46BFF);
    public final NumberSetting size = number("size", "Size", 1.1, 0.4, 3.0, 0.05, " blocks");
    public final NumberSetting duration = number("duration", "Stays for", 1.2, 0.3, 4.0, 0.1, "s");
    public final BoolSetting spin = bool("spin", "Slowly turning", true);
    public final BoolSetting sparks = bool("sparks", "Sparks rising", true);
    public final BoolSetting landing = bool("landing", "Shockwave when you land from high up", true);

    private static final int MAX = 10;
    private final double[] mx = new double[MAX], my = new double[MAX], mz = new double[MAX], start = new double[MAX];
    private final boolean[] land = new boolean[MAX];
    private int next;
    private boolean wasOnGround = true;
    private float fall;

    public JumpFxModule() {
        super("jumpfx", "Jump marks", "A neon mark on the floor every time you jump (six-point star, runes, ring…) in your world colour.", Category.EFFECTS, true, true);
    }

    private int col() {
        return useWorld.on() ? General.accent() : color.argb();
    }

    private void add(double x, double y, double z, boolean isLanding) {
        int i = next;
        next = (next + 1) % MAX;
        mx[i] = x;
        my[i] = y;
        mz[i] = z;
        start[i] = Particles.now();
        land[i] = isLanding;
        if (!isLanding && sparks.on() && General.fx()) {
            int n = General.detail() <= 0 ? 6 : 14;
            for (int k = 0; k < n; k++) {
                double a = k * Math.PI * 2 / n;
                double r = size.f() * 0.8;
                int p = Particles.spawn(Particles.WORLD, Particles.TEX_GLOW, Particles.now(), x + Math.cos(a) * r, y + 0.05, z + Math.sin(a) * r, 0.7f);
                Particles.vel(p, 0, 1.2f + (k % 3) * 0.3f, 0, 1.5f);
                Particles.size(p, 0.09f, 0.02f);
                Particles.color(p, col(), col() & 0xFFFFFF);
            }
        }
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END) return;
        EntityPlayerSP p = Minecraft.getMinecraft().thePlayer;
        if (p == null || !enabled()) {
            wasOnGround = true;
            return;
        }
        if (wasOnGround && !p.onGround && p.motionY > 0.2 && !p.capabilities.isFlying && !p.isInWater()) add(p.posX, p.posY - p.motionY, p.posZ, false);
        if (!p.onGround) fall = Math.max(fall, p.fallDistance);
        if (!wasOnGround && p.onGround) {
            if (landing.on() && fall > 2.5f) add(p.posX, p.posY, p.posZ, true);
            fall = 0;
        }
        wasOnGround = p.onGround;
    }

    @SubscribeEvent
    public void onWorld(RenderWorldLastEvent e) {
        if (!enabled() || !General.fx()) return;
        double now = Particles.now();
        float dur = duration.f();
        boolean any = false;
        for (int i = 0; i < MAX; i++) if (start[i] > 0 && now - start[i] < (land[i] ? 0.7 : dur)) any = true;
        if (!any) return;
        RenderManager rm = Minecraft.getMinecraft().getRenderManager();
        Gl.beginGlow();
        View.capture();
        FxDraw.geo();
        int c = col();
        for (int i = 0; i < MAX; i++) {
            if (start[i] <= 0) continue;
            float t = (float) (now - start[i]);
            double x = mx[i] - rm.viewerPosX, y = my[i] - rm.viewerPosY + 0.03, z = mz[i] - rm.viewerPosZ;
            if (land[i]) {
                if (t >= 0.7f) continue;
                float k = Ease.outCubic(t / 0.7f);
                FxDraw.glowRingH(x, y, z, 0.3 + 3.2 * k * size.f(), 0.06, Colors.alpha(c, 1 - k), 56);
                continue;
            }
            if (t >= dur) continue;
            float grow = Ease.outBack(Ease.clamp01(t / 0.22f));
            float a = 1 - Ease.span(t, dur * 0.6f, dur);
            double r = size.f() * grow;
            double rot = spin.on() ? t * 0.6 : 0;
            mark(x, y, z, r, rot, c, a);
        }
        FxDraw.flush();
        Gl.endGlow();
    }

    /** One mark lying flat: a soft glow under neon lines (wide faint + thin bright). */
    private void mark(double x, double y, double z, double r, double rot, int c, float a) {
        FxDraw.ringH(x, y - 0.005, z, 0, r * 1.05, Colors.alpha(c, 0.22f * a), Colors.alpha(c, 0));
        for (int pass = 0; pass < 2; pass++) {
            double wd = pass == 0 ? 0.14 : 0.04;
            int cc = pass == 0 ? Colors.alpha(c, 0.3f * a) : Colors.alpha(Colors.mix(c, 0xFFFFFFFF, 0.35f), a);
            String s = shape.get();
            if ("star6".equals(s)) {
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.82, 3, rot, wd, cc);
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.82, 3, rot + Math.PI, wd, cc);
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r, 48, 0, wd, cc);
                if (pass == 1) FxDraw.disc(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.09, cc, cc, 16);
            } else if ("runes".equals(s)) {
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r, 48, 0, wd, cc);
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.72, 40, 0, wd, cc);
                for (int k = 0; k < 12; k++) {
                    double ang = rot + k * Math.PI / 6;
                    double ca = Math.cos(ang), sa = Math.sin(ang), r0 = r * 0.76, r1 = r * 0.96;
                    FxDraw.line(x + ca * r0, y, z + sa * r0, x + ca * r1, y, z + sa * r1, wd * 0.8, cc);
                    double cb = Math.cos(ang + 0.13), sb = Math.sin(ang + 0.13);
                    if (k % 2 == 0) FxDraw.line(x + ca * r1, y, z + sa * r1, x + cb * r0 * 1.12, y, z + sb * r0 * 1.12, wd * 0.8, cc);
                }
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.45, 4, rot, wd, cc);
            } else if ("triangle".equals(s)) {
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.9, 3, rot, wd, cc);
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r, 48, 0, wd, cc);
            } else if ("diamond".equals(s)) {
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.9, 4, rot, wd, cc);
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.55, 4, rot + Math.PI / 4, wd, cc);
            } else {
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r, 48, 0, wd, cc);
                FxDraw.polygon(x, y, z, 1, 0, 0, 0, 0, 1, r * 0.6, 40, 0, wd, cc);
            }
        }
    }
}
