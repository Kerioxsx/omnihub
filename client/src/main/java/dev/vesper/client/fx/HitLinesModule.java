package dev.vesper.client.fx;

import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.util.Vec3;

/**
 * Lines on hits: an anime-style burst of impact lines, a slash arc across
 * the target in the direction you swung, extra gold lines on critical hits,
 * and optional speed lines at the screen's edges while you sprint. Purely
 * visual; nothing about the hit changes. Only you see them.
 */
public final class HitLinesModule extends Module {
    public static HitLinesModule INSTANCE;

    public final ChoiceSetting style = choice("style", "Style", "both", new String[] {"burst", "slash", "both"}, new String[] {"Impact burst", "Slash arc", "Burst + slash"});
    public final ColorSetting color = color("color", "Colour", 0xFFFFFFFF);
    public final ColorSetting critColor = color("critColor", "Critical hit colour", 0xFFFFC94D);
    public final NumberSetting length = number("length", "Line length", 1.0, 0.4, 2.5, 0.1, "×");
    public final NumberSetting thickness = number("thickness", "Thickness", 1.0, 0.4, 2.5, 0.1, "×");
    public final NumberSetting duration = number("duration", "Duration", 0.3, 0.12, 0.8, 0.02, "s");
    public final BoolSetting critOnly = bool("critOnly", "Only on critical hits", false);
    public final BoolSetting speedLines = bool("speedLines", "Speed lines while sprinting", false);
    public final ColorSetting speedColor = color("speedColor", "Speed line colour", 0x99FFFFFF);

    private static final int MAX = 12;
    private final double[] hx = new double[MAX], hy = new double[MAX], hz = new double[MAX], dx = new double[MAX], dz = new double[MAX];
    private final double[] start = new double[MAX];
    private final float[] hh = new float[MAX];
    private final boolean[] crit = new boolean[MAX];
    private final long[] seed = new long[MAX];
    private int next;
    private final double[] arc = new double[20 * 3];
    private float sprint;

    public HitLinesModule() {
        super("hitlines", "Hit lines", "Impact lines and a slash arc where you hit, gold on critical hits, and speed lines while sprinting. Only you see them.", Category.EFFECTS, true, true);
        INSTANCE = this;
    }

    void on(CombatVisualEvent e) {
        if (!enabled() || !General.fx() || e.type != CombatVisualEvent.Type.HIT_CONFIRMED) return;
        EntityPlayerSP p = Minecraft.getMinecraft().thePlayer;
        if (p == null) return;
        // A critical hit in 1.8: falling, not on the ground, not in water, not on a ladder, not riding.
        boolean isCrit = p.fallDistance > 0 && !p.onGround && !p.isOnLadder() && !p.isInWater() && p.ridingEntity == null;
        if (critOnly.on() && !isCrit) return;
        int i = next;
        next = (next + 1) % MAX;
        hx[i] = e.x;
        hy[i] = e.y + e.height * 0.55;
        hz[i] = e.z;
        hh[i] = (float) e.height;
        Vec3 look = p.getLookVec();
        double l = Math.sqrt(look.xCoord * look.xCoord + look.zCoord * look.zCoord);
        dx[i] = l < 1e-6 ? 0 : look.xCoord / l;
        dz[i] = l < 1e-6 ? 1 : look.zCoord / l;
        start[i] = Particles.now();
        crit[i] = isCrit;
        seed[i] = (long) (start[i] * 1000) ^ i * 0x9E3779B9L;
    }

    /** World pass, inside a glow block after View.capture(). */
    void renderWorld(double vx, double vy, double vz) {
        if (!enabled()) return;
        double now = Particles.now();
        float dur = duration.f();
        boolean any = false;
        for (int i = 0; i < MAX; i++) if (start[i] > 0 && now - start[i] < dur * 1.6) any = true;
        if (!any) return;
        FxDraw.geo();
        for (int i = 0; i < MAX; i++) {
            if (start[i] <= 0) continue;
            float t = (float) ((now - start[i]) / dur);
            if (t >= 1.6f) continue;
            int col = crit[i] ? critColor.argb() : color.argb();
            double cx = hx[i] - vx, cy = hy[i] - vy, cz = hz[i] - vz;
            float len = length.f() * (crit[i] ? 1.35f : 1f), th = thickness.f();
            Rng r = new Rng(seed[i]);
            if (!style.is("slash") && t < 1) {
                int n = crit[i] ? 16 : 10;
                for (int k = 0; k < n; k++) {
                    double a = k * Math.PI * 2 / n + r.signed() * 0.25;
                    double ca = Math.cos(a), sa = Math.sin(a);
                    double ux = View.rx * ca + View.ux * sa, uy = View.ry * ca + View.uy * sa, uz = View.rz * ca + View.uz * sa;
                    float e = Ease.outCubic(t);
                    double r0 = (0.35 + 0.5 * e) * len * (0.8 + r.f() * 0.4), r1 = r0 + (0.55 - 0.4 * t) * len * (0.7 + r.f() * 0.6);
                    float a1 = (1 - t) * (1 - t);
                    FxDraw.line(cx + ux * r0, cy + uy * r0, cz + uz * r0, cx + ux * r1, cy + uy * r1, cz + uz * r1, 0.045 * th, Colors.alpha(col, a1), Colors.alpha(col, 0));
                }
            }
            if (!style.is("burst") && t < 1.6f) {
                // The arc lies across the target, perpendicular to your swing, tilted.
                double px = -dz[i], pz = dx[i];
                double tilt = (r.f() - 0.5) * 0.9 + 0.45;
                double head = Math.min(1, t * 1.6), tail = Math.max(0, t * 1.6 - 0.75);
                if (head > tail + 0.02) {
                    int n = 20;
                    double rad = 0.75 * Math.max(0.8, hh[i] * 0.6);
                    for (int k = 0; k < n; k++) {
                        double f = tail + (head - tail) * k / (n - 1);
                        double ang = -1.1 + 2.2 * f;
                        double sx = Math.cos(ang) * rad, sy = Math.sin(ang) * rad;
                        arc[k * 3] = cx + px * sx - dx[i] * 0.25;
                        arc[k * 3 + 1] = cy + sy * Math.cos(tilt) + sx * Math.sin(tilt) * 0.3;
                        arc[k * 3 + 2] = cz + pz * sx - dz[i] * 0.25;
                    }
                    float fa = 1 - Ease.span(t, 0.9f, 1.6f);
                    FxDraw.ribbon(arc, n, 0.02 * th, 0.16 * th * len, Colors.alpha(col, 0), Colors.alpha(col, fa));
                    FxDraw.ribbon(arc, n, 0.01 * th, 0.05 * th * len, Colors.alpha(0xFFFFFFFF, 0), Colors.alpha(0xFFFFFFFF, fa));
                }
            }
        }
        FxDraw.flush();
    }

    /** Speed lines at the screen's edges while sprinting (HUD pass). */
    void renderHud(float partialTicks) {
        EntityPlayerSP p = Minecraft.getMinecraft().thePlayer;
        if (!enabled() || !speedLines.on() || p == null || General.calm()) return;
        double v = Math.sqrt(p.motionX * p.motionX + p.motionZ * p.motionZ);
        float target = p.isSprinting() ? (float) Math.min(1, Math.max(0, (v - 0.12) / 0.2)) : 0;
        sprint = Ease.approach(sprint, target, 3f, 1 / 60f);
        if (sprint < 0.02f) return;
        ScaledResolution sr = new ScaledResolution(Minecraft.getMinecraft());
        float w = sr.getScaledWidth(), h = sr.getScaledHeight(), cx = w / 2, cy = h / 2;
        float maxR = (float) Math.sqrt(cx * cx + cy * cy);
        long frame = (long) (Ease.seconds() * 24);
        Rng r = new Rng(frame * 0x2545F4914F6CDD1DL);
        Gl.begin2D();
        int n = 28;
        for (int k = 0; k < n; k++) {
            double a = r.f() * Math.PI * 2;
            float r0 = maxR * (0.62f + r.f() * 0.18f), r1 = maxR * 1.02f;
            float al = sprint * (0.35f + r.f() * 0.65f);
            Draw.line(cx + Math.cos(a) * r0, cy + Math.sin(a) * r0, cx + Math.cos(a) * r1, cy + Math.sin(a) * r1, 0.6f + r.f() * 1.2f, Colors.alpha(speedColor.argb(), 0), Colors.alpha(speedColor.argb(), al));
        }
        Gl.end2D();
    }
}
