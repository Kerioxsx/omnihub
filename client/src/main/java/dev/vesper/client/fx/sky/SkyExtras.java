package dev.vesper.client.fx.sky;

import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Textures;
import java.util.Random;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.opengl.GL11;

/**
 * Extra layers over a Vesper sky in the world: daylight that follows the
 * game's clock (with its sun), a moon, a Milky Way band, shooting stars and
 * ambient motes drifting around you. All procedural and seeded.
 */
public final class SkyExtras {
    private SkyExtras() {
    }

    private static final double R = 88;
    private static final int BAND = 420;
    private static final float[] BX = new float[BAND], BY = new float[BAND], BZ = new float[BAND], BB = new float[BAND];
    private static final int MOTES = 90;
    private static final float[] MX = new float[MOTES], MY = new float[MOTES], MZ = new float[MOTES], MP = new float[MOTES];

    static {
        Random r = new Random(0x6A1AL);
        // A great circle tilted 62° across the sky; stars scattered around it.
        double ux = Math.cos(0.4), uy = 0, uz = Math.sin(0.4);
        double vx = -Math.sin(0.4) * Math.cos(1.08), vy = Math.sin(1.08), vz = Math.cos(0.4) * Math.cos(1.08);
        double nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
        for (int i = 0; i < BAND; i++) {
            double a = r.nextDouble() * Math.PI * 2, spread = r.nextGaussian() * 0.13;
            double x = Math.cos(a) * ux + Math.sin(a) * vx + spread * nx, y = Math.cos(a) * uy + Math.sin(a) * vy + spread * ny, z = Math.cos(a) * uz + Math.sin(a) * vz + spread * nz;
            double l = Math.sqrt(x * x + y * y + z * z);
            BX[i] = (float) (x / l);
            BY[i] = (float) (y / l);
            BZ[i] = (float) (z / l);
            BB[i] = (float) (0.25 + r.nextDouble() * 0.75) * (float) Math.exp(-Math.abs(spread) * 6);
        }
        for (int i = 0; i < MOTES; i++) {
            MX[i] = r.nextFloat();
            MY[i] = r.nextFloat();
            MZ[i] = r.nextFloat();
            MP[i] = r.nextFloat() * 6.283f;
        }
    }

    /** How much it is day (0 night … 1 day) at this celestial angle, like the game's own sky light. */
    public static float daylight(float celestialAngle) {
        float f = (float) Math.cos(celestialAngle * Math.PI * 2) * 2 + 0.5f;
        return Math.max(0, Math.min(1, f));
    }

    private static void begin(boolean additive) {
        GlStateManager.disableFog();
        GlStateManager.disableLighting();
        GlStateManager.disableCull();
        GlStateManager.disableAlpha();
        GlStateManager.enableBlend();
        GlStateManager.depthMask(false);
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, additive ? GL11.GL_ONE : GL11.GL_ONE_MINUS_SRC_ALPHA);
    }

    private static void end() {
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.depthMask(true);
        GlStateManager.shadeModel(GL11.GL_FLAT);
        GlStateManager.enableAlpha();
        GlStateManager.enableCull();
        GlStateManager.enableTexture2D();
        GlStateManager.disableBlend();
        GlStateManager.color(1, 1, 1, 1);
    }

    private static void v(WorldRenderer w, double x, double y, double z, int c) {
        w.pos(x, y, z).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    /** A dome gradient over everything (daylight, dimming). */
    public static void overlay(int zenith, int horizon, float alpha) {
        if (alpha <= 0.003f) return;
        begin(false);
        GlStateManager.disableTexture2D();
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        int rings = 8, seg = 32;
        for (int ri = -2; ri < rings; ri++) {
            double e0 = ri / (double) rings * Math.PI / 2, e1 = (ri + 1) / (double) rings * Math.PI / 2;
            float f0 = (float) Math.max(0, e0 / (Math.PI / 2)), f1 = (float) Math.max(0, e1 / (Math.PI / 2));
            int c0 = Colors.alpha(Colors.mix(horizon, zenith, f0), alpha), c1 = Colors.alpha(Colors.mix(horizon, zenith, f1), alpha);
            w.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
            for (int i = 0; i <= seg; i++) {
                double a = i * Math.PI * 2 / seg;
                v(w, Math.cos(a) * Math.cos(e1) * R, Math.sin(e1) * R, Math.sin(a) * Math.cos(e1) * R, c1);
                v(w, Math.cos(a) * Math.cos(e0) * R, Math.sin(e0) * R, Math.sin(a) * Math.cos(e0) * R, c0);
            }
            t.draw();
        }
        end();
    }

    /** Camera-facing quad on the dome (centre direction dx,dy,dz unit, half-size s). */
    private static void billboard(WorldRenderer w, double dx, double dy, double dz, double dist, double s, int c) {
        double x = dx * dist, y = dy * dist, z = dz * dist;
        double rx = dz, rz = -dx, rl = Math.sqrt(rx * rx + rz * rz);
        if (rl < 1e-6) {
            rx = 1;
            rz = 0;
            rl = 1;
        }
        rx /= rl;
        rz /= rl;
        double ux = dy * rz, uy = dz * rx - dx * rz, uz = -dy * rx;
        int r = (c >> 16) & 255, g = (c >> 8) & 255, b = c & 255, a = c >>> 24;
        w.pos(x - (rx + ux) * s, y - uy * s, z - (rz + uz) * s).tex(0, 0).color(r, g, b, a).endVertex();
        w.pos(x + (rx - ux) * s, y - uy * s, z + (rz - uz) * s).tex(1, 0).color(r, g, b, a).endVertex();
        w.pos(x + (rx + ux) * s, y + uy * s, z + (rz + uz) * s).tex(1, 1).color(r, g, b, a).endVertex();
        w.pos(x - (rx - ux) * s, y + uy * s, z - (rz - uz) * s).tex(0, 1).color(r, g, b, a).endVertex();
    }

    /** The sun (day) and a big soft moon (night) at the game's celestial angle. */
    public static void sunAndMoon(float celestialAngle, float day, boolean moon, float moonSize, int accent) {
        double th = celestialAngle * Math.PI * 2;
        double sx = -Math.sin(th), sy = Math.cos(th);
        begin(true);
        GlStateManager.enableTexture2D();
        Minecraft.getMinecraft().getTextureManager().bindTexture(Textures.glow());
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        if (day > 0.01f) {
            billboard(w, sx, sy, 0, R * 0.9, 26, Colors.withAlpha(0xFFE7A8, 0.45f * day));
            billboard(w, sx, sy, 0, R * 0.9, 9, Colors.withAlpha(0xFFFBEA, day));
        }
        if (moon && day < 0.99f) {
            float n = 1 - day;
            double s = 7 * moonSize;
            billboard(w, -sx, -sy, 0, R * 0.9, s * 3.2, Colors.withAlpha(accent & 0xFFFFFF, 0.28f * n));
            billboard(w, -sx, -sy, 0, R * 0.9, s * 1.4, Colors.withAlpha(0xE8EEFF, 0.55f * n));
            billboard(w, -sx, -sy, 0, R * 0.9, s, Colors.withAlpha(0xFFFFFF, 0.9f * n));
        }
        t.draw();
        end();
    }

    /** The Milky Way: a band of faint stars and coloured haze. */
    public static void galaxy(float time, float alpha, int accent) {
        if (alpha <= 0.01f) return;
        begin(true);
        GlStateManager.enableTexture2D();
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        Minecraft.getMinecraft().getTextureManager().bindTexture(Textures.glow());
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        for (int i = 0; i < BAND; i += 12) billboard(w, BX[i], BY[i], BZ[i], R * 0.97, 9 + BB[i] * 8, Colors.withAlpha(accent & 0xFFFFFF, 0.05f * alpha));
        t.draw();
        Minecraft.getMinecraft().getTextureManager().bindTexture(Textures.twinkle());
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        for (int i = 0; i < BAND; i++) {
            if (BY[i] < -0.05f) continue;
            float tw = 0.65f + 0.35f * (float) Math.sin(time * (1 + BB[i]) + i);
            billboard(w, BX[i], BY[i], BZ[i], R * 0.96, 0.35 + BB[i] * 0.55, Colors.withAlpha(0xFFFFFF, BB[i] * tw * alpha));
        }
        t.draw();
        end();
    }

    /** Shooting stars: about `perMinute` a minute, each crossing in under a second. */
    public static void shootingStars(float time, float perMinute, float alpha, int accent) {
        if (alpha <= 0.01f || perMinute <= 0) return;
        double slot = 60.0 / perMinute;
        long now = (long) Math.floor(time / slot);
        begin(true);
        GlStateManager.disableTexture2D();
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        for (long n = now - 1; n <= now; n++) {
            Random r = new Random(n * 0x9E3779B97F4A7C15L + 7);
            double start = n * slot + r.nextDouble() * slot * 0.8, len = 0.6 + r.nextDouble() * 0.5;
            double p = (time - start) / len;
            if (p < 0 || p > 1) continue;
            double az = r.nextDouble() * Math.PI * 2, el = Math.toRadians(30 + r.nextDouble() * 40);
            double dir = r.nextDouble() * Math.PI * 2;
            // Head moves across the dome from (az, el) along `dir`, 35° in all.
            double sweep = Math.toRadians(35);
            double hA = az + Math.cos(dir) * sweep * p, hE = el - Math.abs(Math.sin(dir)) * sweep * 0.6 * p;
            double tp = Math.max(0, p - 0.25);
            double tA = az + Math.cos(dir) * sweep * tp, tE = el - Math.abs(Math.sin(dir)) * sweep * 0.6 * tp;
            float fade = (float) Math.sin(p * Math.PI) * alpha;
            double hx = Math.cos(hA) * Math.cos(hE) * R * 0.9, hy = Math.sin(hE) * R * 0.9, hz = Math.sin(hA) * Math.cos(hE) * R * 0.9;
            double tx = Math.cos(tA) * Math.cos(tE) * R * 0.9, ty = Math.sin(tE) * R * 0.9, tz = Math.sin(tA) * Math.cos(tE) * R * 0.9;
            // A thin quad from tail to head, wider at the head.
            double dx = hx - tx, dy = hy - ty, dz = hz - tz;
            double sx = dy * hz - dz * hy, sy = dz * hx - dx * hz, sz = dx * hy - dy * hx, sl = Math.sqrt(sx * sx + sy * sy + sz * sz);
            if (sl < 1e-6) continue;
            sx /= sl;
            sy /= sl;
            sz /= sl;
            double wh = 0.35, wt = 0.04;
            int head = Colors.withAlpha(0xFFFFFF, fade), tail = Colors.withAlpha(accent & 0xFFFFFF, 0);
            w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_COLOR);
            v(w, tx + sx * wt, ty + sy * wt, tz + sz * wt, tail);
            v(w, hx + sx * wh, hy + sy * wh, hz + sz * wh, head);
            v(w, hx - sx * wh, hy - sy * wh, hz - sz * wh, head);
            v(w, tx - sx * wt, ty - sy * wt, tz - sz * wt, tail);
            t.draw();
        }
        end();
    }

    /**
     * Glowing motes drifting around the camera (world pass). (camX..camZ) is
     * the camera position, (offX..offZ) what to subtract for render space.
     * Kind: 1 fireflies, 2 snow sparkles, 3 embers.
     */
    public static void motes(int kind, float time, double camX, double camY, double camZ, double offX, double offY, double offZ, float amount, int accent) {
        if (kind <= 0 || amount <= 0) return;
        int n = Math.max(4, Math.round(MOTES * amount));
        double box = 18;
        begin(true);
        GlStateManager.enableTexture2D();
        Minecraft.getMinecraft().getTextureManager().bindTexture(Textures.glow());
        net.minecraft.client.renderer.entity.RenderManager rm = Minecraft.getMinecraft().getRenderManager();
        double yaw = Math.toRadians(rm.playerViewY), pitch = Math.toRadians(rm.playerViewX);
        double rx = Math.cos(yaw), rz = Math.sin(yaw);
        double ux = -Math.sin(yaw) * Math.sin(pitch), uy = Math.cos(pitch), uz = Math.cos(yaw) * Math.sin(pitch);
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
        for (int i = 0; i < n; i++) {
            double fall = kind == 2 ? -0.9 : kind == 3 ? 0.8 : 0.08;
            double ph = MP[i];
            double x = MX[i] * box + Math.sin(time * 0.35 + ph) * 1.2;
            double y = MY[i] * box + time * fall;
            double z = MZ[i] * box + Math.cos(time * 0.3 + ph) * 1.2;
            // Wrap into a box that follows the camera.
            x = camX + (((x - camX) % box) + box * 1.5) % box - box / 2;
            y = camY + (((y - camY) % box) + box * 1.5) % box - box / 2;
            z = camZ + (((z - camZ) % box) + box * 1.5) % box - box / 2;
            float tw = kind == 1 ? (float) Math.max(0, Math.sin(time * 1.6 + ph * 3)) : 0.6f + 0.4f * (float) Math.sin(time * 3 + ph);
            int col = kind == 2 ? 0xFFEAF4FF : kind == 3 ? 0xFFFF8A3D : accent;
            double s = kind == 2 ? 0.06 : 0.08;
            int c = Colors.alpha(col, tw * 0.9f);
            int r = (c >> 16) & 255, g = (c >> 8) & 255, b = c & 255, a = c >>> 24;
            double px = x - offX, py = y - offY, pz = z - offZ;
            w.pos(px - (rx + ux) * s, py - uy * s, pz - (rz + uz) * s).tex(0, 0).color(r, g, b, a).endVertex();
            w.pos(px + (rx - ux) * s, py - uy * s, pz + (rz - uz) * s).tex(1, 0).color(r, g, b, a).endVertex();
            w.pos(px + (rx + ux) * s, py + uy * s, pz + (rz + uz) * s).tex(1, 1).color(r, g, b, a).endVertex();
            w.pos(px - (rx - ux) * s, py + uy * s, pz - (rz - uz) * s).tex(0, 1).color(r, g, b, a).endVertex();
        }
        t.draw();
        end();
    }
}
