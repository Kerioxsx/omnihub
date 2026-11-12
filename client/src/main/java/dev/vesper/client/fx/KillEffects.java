package dev.vesper.client.fx;

import dev.vesper.client.fx.effects.AstralCollapse;
import dev.vesper.client.fx.effects.CometAscension;
import dev.vesper.client.fx.effects.FrostConstellation;
import dev.vesper.client.fx.effects.SolarBloom;
import dev.vesper.client.fx.effects.VioletRift;
import dev.vesper.client.module.General;
import dev.vesper.client.render.Gl;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

/**
 * The kill-effect library and its running instances. World effects play at
 * world positions (drawn from Fx's world-render handler); stage effects are
 * for the showroom, relative to the stage matrix.
 */
public final class KillEffects {
    private KillEffects() {
    }

    public static final String[] IDS = {"astral_collapse", "violet_rift", "solar_bloom", "frost_constellation", "comet_ascension"};
    public static final String[] NAMES = {"Astral Collapse", "Violet Rift", "Solar Bloom", "Frost Constellation", "Comet Ascension"};

    private static final List<KillEffect> WORLD = new ArrayList<KillEffect>(), STAGE = new ArrayList<KillEffect>();
    private static long seq = 0x5EED;

    static KillEffect create(String id) {
        if ("violet_rift".equals(id)) return new VioletRift();
        if ("solar_bloom".equals(id)) return new SolarBloom();
        if ("frost_constellation".equals(id)) return new FrostConstellation();
        if ("comet_ascension".equals(id)) return new CometAscension();
        return new AstralCollapse();
    }

    /** Starts an effect in a space; trims the oldest when over the limit. */
    static void play(int space, String id, double x, double y, double z, double height, KillSettings s, int maxAtOnce) {
        List<KillEffect> list = space == Particles.STAGE ? STAGE : WORLD;
        while (list.size() >= Math.max(1, maxAtOnce)) list.remove(0);
        KillEffect e = create(id);
        e.begin(space, x, y, z, height, s, Particles.now(), seq++ * 0x9E3779B97F4A7C15L);
        list.add(e);
    }

    /** A local DEMO at a world position (/vesper demo). */
    public static void demo(String id, double x, double y, double z) {
        KillFxModule m = KillFxModule.INSTANCE;
        play(Particles.WORLD, id, x, y, z, 1.8, m == null ? new KillSettings() : m.killSettings(), m == null ? 3 : m.maxAtOnce.i());
    }

    /** A DEMO on the showroom stage (coordinates in stage space). */
    public static void demoStage(String id, double x, double y, double z) {
        KillFxModule m = KillFxModule.INSTANCE;
        play(Particles.STAGE, id, x, y, z, 1.8, m == null ? new KillSettings() : m.killSettings(), 3);
    }

    /** Draws running world effects; the matrix is the world camera matrix. */
    static void renderWorld(double vx, double vy, double vz) {
        if (WORLD.isEmpty() && Particles.count(Particles.WORLD) == 0) return;
        Gl.beginGlow();
        View.capture();
        double now = Particles.now();
        for (Iterator<KillEffect> it = WORLD.iterator(); it.hasNext(); ) if (!it.next().render(now, vx, vy, vz)) it.remove();
        Particles.render(Particles.WORLD, vx, vy, vz);
        FxDraw.flush();
        Gl.endGlow();
    }

    /** Draws running stage effects relative to the current (stage) matrix. Restores GL state. */
    public static void renderStage(float partialTicks) {
        if (STAGE.isEmpty() && Particles.count(Particles.STAGE) == 0) return;
        Gl.beginGlow();
        View.capture();
        double now = Particles.now();
        for (Iterator<KillEffect> it = STAGE.iterator(); it.hasNext(); ) if (!it.next().render(now, 0, 0, 0)) it.remove();
        Particles.render(Particles.STAGE, 0, 0, 0);
        FxDraw.flush();
        Gl.endGlow();
    }

    /** Kept for API symmetry; world effects render from Fx's handler. */
    public static void renderAt(float partialTicks) {
    }

    public static void clearWorld() {
        WORLD.clear();
        Particles.clear(Particles.WORLD);
    }

    public static void clearStage() {
        STAGE.clear();
        Particles.clear(Particles.STAGE);
    }

    public static boolean busy() {
        return !WORLD.isEmpty() || General.fx() && Particles.count(Particles.WORLD) > 0;
    }
}
