package dev.vesper.client.world;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
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

/**
 * Neon glyphs floating in the world around you: original line symbols
 * drifting in a slow ring, bobbing and turning, in your world colour.
 */
public final class WorldGlyphsModule extends Module {
    public final NumberSetting count = number("count", "Glyphs", 10, 3, 24, 1, "");
    public final NumberSetting radius = number("radius", "Distance", 5, 2, 14, 0.5, " blocks");
    public final NumberSetting size = number("size", "Size", 0.6, 0.2, 2, 0.05, " blocks");
    public final NumberSetting speed = number("speed", "Speed", 1, 0, 3, 0.1, "×");
    public final NumberSetting height = number("height", "Height", 1.6, -1, 6, 0.1, " blocks");
    public final BoolSetting useWorld = bool("useWorld", "Use the world colour", true);
    public final ColorSetting color = color("color", "Colour (when not the world colour)", 0xFFFFC94D);

    /** Original glyphs as line segments in a unit box (-1..1): x0, y0, x1, y1 per segment. */
    private static final float[][] GLYPHS = {
        {-0.7f, -0.8f, 0, 0.8f, 0, 0.8f, 0.7f, -0.8f, -0.45f, -0.2f, 0.45f, -0.2f},
        {0, -0.9f, 0, 0.9f, -0.7f, 0.4f, 0, 0, 0, 0, 0.7f, 0.4f},
        {-0.7f, 0.7f, 0.7f, 0.7f, 0.7f, 0.7f, -0.7f, -0.7f, -0.7f, -0.7f, 0.7f, -0.7f},
        {-0.6f, -0.9f, -0.6f, 0.9f, 0.6f, -0.9f, 0.6f, 0.9f, -0.6f, 0, 0.6f, 0.3f},
        {0, 0.9f, 0.8f, 0, 0.8f, 0, 0, -0.9f, 0, -0.9f, -0.8f, 0, -0.8f, 0, 0, 0.9f, 0, 0.35f, 0, -0.35f},
        {-0.8f, 0.6f, 0, -0.8f, 0, -0.8f, 0.8f, 0.6f, 0, -0.8f, 0, 0.9f},
        {-0.7f, -0.7f, 0.7f, 0.7f, -0.7f, 0.7f, 0.7f, -0.7f, -0.9f, 0, -0.5f, 0, 0.5f, 0, 0.9f, 0},
        {-0.5f, 0.9f, 0.5f, 0.9f, 0, 0.9f, 0, -0.9f, -0.6f, -0.3f, 0.6f, -0.3f, -0.6f, -0.3f, -0.6f, -0.7f},
    };

    public WorldGlyphsModule() {
        super("worldglyphs", "World glyphs", "Neon glyphs floating around you in your world colour. Only you see them.", Category.VISUALS, false, true);
    }

    @SubscribeEvent
    public void onWorld(RenderWorldLastEvent e) {
        EntityPlayerSP p = Minecraft.getMinecraft().thePlayer;
        if (!enabled() || !General.fx() || p == null) return;
        RenderManager rm = Minecraft.getMinecraft().getRenderManager();
        float pt = e.partialTicks;
        double px = p.prevPosX + (p.posX - p.prevPosX) * pt - rm.viewerPosX;
        double py = p.prevPosY + (p.posY - p.prevPosY) * pt - rm.viewerPosY;
        double pz = p.prevPosZ + (p.posZ - p.prevPosZ) * pt - rm.viewerPosZ;
        float t = General.calm() ? 0 : Ease.seconds() * speed.f();
        int c = useWorld.on() ? General.accent() : color.argb();
        int n = count.i();
        Gl.beginGlow();
        View.capture();
        FxDraw.geo();
        for (int i = 0; i < n; i++) {
            double ang = i * Math.PI * 2 / n + t * 0.12;
            double r = radius.f() * (0.85 + 0.15 * Math.sin(i * 1.7));
            double gx = px + Math.cos(ang) * r, gz = pz + Math.sin(ang) * r;
            double gy = py + height.f() + Math.sin(t * 0.9 + i * 1.3) * 0.25 + (i % 3) * 0.35;
            float s = size.f() * 0.5f;
            float rot = t * 0.4f + i;
            float pulse = 0.7f + 0.3f * (float) Math.sin(t * 2 + i);
            float[] g = GLYPHS[i % GLYPHS.length];
            double cr = Math.cos(rot) * s, sr = Math.sin(rot) * s;
            for (int pass = 0; pass < 2; pass++) {
                double wd = pass == 0 ? 0.09 * size.f() : 0.025 * size.f();
                int cc = pass == 0 ? Colors.alpha(c, 0.25f * pulse) : Colors.alpha(Colors.mix(c, 0xFFFFFFFF, 0.3f), pulse);
                for (int k = 0; k + 3 < g.length; k += 4) {
                    double ax = g[k] * cr - g[k + 1] * sr, ay = g[k] * sr + g[k + 1] * cr;
                    double bx = g[k + 2] * cr - g[k + 3] * sr, by = g[k + 2] * sr + g[k + 3] * cr;
                    FxDraw.line(gx + View.rx * ax + View.ux * ay, gy + View.ry * ax + View.uy * ay, gz + View.rz * ax + View.uz * ay,
                        gx + View.rx * bx + View.ux * by, gy + View.ry * bx + View.uy * by, gz + View.rz * bx + View.uz * by, wd, cc);
                }
            }
        }
        FxDraw.flush();
        Gl.endGlow();
    }
}
