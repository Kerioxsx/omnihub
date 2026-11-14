package dev.vesper.client.fx.sky;

import dev.vesper.client.module.General;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.Textures;
import java.util.Random;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.opengl.GL11;

/**
 * Procedural skies (VISUAL_SPEC §5): Nocturne (blue starfield),
 * Bloodmoon (crimson storm), Riftstorm (violet rift), Inferno (orange
 * clouds), Aurora Veil (purple aurora), Galaxy Core, Sunset and Abyss. Nothing is a copied texture:
 * gradients, seeded stars and soft glow clouds.
 */
public final class Skies {
    private Skies() {
    }

    public static final String[] IDS = {"nocturne", "bloodmoon", "riftstorm", "inferno", "aurora", "galaxy", "sunset", "abyss"};
    public static final String[] NAMES = {"Nocturne", "Bloodmoon", "Riftstorm", "Inferno", "Aurora Veil", "Galaxy Core", "Sunset", "Abyss"};

    /** zenith, mid, horizon, cloud colour, cloud alpha, star alpha, accent (aurora/rift). */
    private static final int[][] STYLE = {
        {0xFF050A1C, 0xFF0F1E44, 0xFF27406E, 0xFF35507E, 18, 100, 0xFF6FB7FF},
        {0xFF12030A, 0xFF4A0A18, 0xFF8E1F22, 0xFFB8323A, 55, 30, 0xFFFF5A4A},
        {0xFF0B0418, 0xFF2A0F4A, 0xFF5B2486, 0xFF7B3FB8, 45, 55, 0xFFD27BFF},
        {0xFF2A0A04, 0xFF8A2E08, 0xFFE0701A, 0xFFFF9A2E, 85, 0, 0xFFFFC46B},
        {0xFF080B26, 0xFF1C1748, 0xFF3D2A6E, 0xFF5A3B8E, 15, 100, 0xFF7CF5C8},
        {0xFF03020A, 0xFF120A2A, 0xFF2B1650, 0xFF4A2A80, 10, 100, 0xFFFF8AD8},
        {0xFF1E1250, 0xFF8A3B8E, 0xFFFF9A6B, 0xFFFFB38A, 60, 25, 0xFFFFD0E0},
        {0xFF01060C, 0xFF04223A, 0xFF0A5566, 0xFF1A7F8A, 25, 70, 0xFF5CFFE0},
    };

    private static final int STARS = 220, CLOUDS = 26;
    private static final float[] SX = new float[STARS], SY = new float[STARS], SZ = new float[STARS], SB = new float[STARS];
    private static final float[] CX = new float[CLOUDS], CY = new float[CLOUDS], CS = new float[CLOUDS], CV = new float[CLOUDS];

    static {
        Random r = new Random(0xA57E5L);
        for (int i = 0; i < STARS; i++) {
            // Upper hemisphere directions, denser toward the zenith.
            double u = 0.08 + 0.92 * Math.sqrt(r.nextDouble()), th = r.nextDouble() * Math.PI * 2, c = Math.sqrt(1 - u * u);
            SX[i] = (float) (Math.cos(th) * c);
            SY[i] = (float) u;
            SZ[i] = (float) (Math.sin(th) * c);
            SB[i] = 0.3f + r.nextFloat() * 0.7f;
        }
        for (int i = 0; i < CLOUDS; i++) {
            CX[i] = r.nextFloat();
            CY[i] = 0.35f + r.nextFloat() * 0.6f;
            CS[i] = 0.12f + r.nextFloat() * 0.22f;
            CV[i] = 0.004f + r.nextFloat() * 0.008f;
        }
    }

    static int index(String id) {
        for (int i = 0; i < IDS.length; i++) if (IDS[i].equals(id)) return i;
        return 0;
    }

    /** The sky as a 2D backdrop filling (x, y, w, h). Restores GL state. */
    public static void drawBackdrop(String id, float x, float y, float w, float h, float time) {
        int k = index(id);
        int[] s = STYLE[k];
        float t = General.calm() ? 0 : time;
        Gl.begin2D();
        Draw.gradientV(x, y, x + w, y + h * 0.55f, s[0], s[1]);
        Draw.gradientV(x, y + h * 0.55f, x + w, y + h, s[1], s[2]);
        // Stars (upper part), twinkling.
        if (s[5] > 0) {
            for (int i = 0; i < STARS; i++) {
                float sx = x + ((SX[i] * 0.5f + 0.5f) * 1.3f - 0.15f + 0.02f * (SZ[i])) * w;
                float sy = y + (1 - SY[i]) * 0.85f * h;
                if (sx < x || sx > x + w) continue;
                float tw = 0.55f + 0.45f * (float) Math.sin(t * (1.5f + SB[i] * 2) + i);
                Draw.circle(sx, sy, 0.35f + SB[i] * 0.55f, Colors.withAlpha(0xFFFFFF, SB[i] * tw * s[5] / 100f));
            }
        }
        // Aurora / rift ribbons.
        if (k == 4 || k == 2 || k == 0) ribbons2D(k, x, y, w, h, t, s[6]);
        // Clouds: soft glows drifting sideways, lower half.
        for (int i = 0; i < CLOUDS && s[4] > 0; i++) {
            float cx = x + (((CX[i] + t * CV[i]) % 1.2f) - 0.1f) * w;
            float cy = y + CY[i] * h;
            Draw.glow(cx, cy, CS[i] * h * 1.6f, Colors.withAlpha(s[3], s[4] / 100f * 0.5f));
        }
        if (k == 1) {
            // Bloodmoon: a pale red moon with a halo.
            Draw.glow(x + w * 0.74f, y + h * 0.22f, h * 0.22f, Colors.withAlpha(0xFF6A5A, 0.35f));
            Draw.circle(x + w * 0.74f, y + h * 0.22f, h * 0.055f, 0xFFF2C7BE);
        }
        if (k == 3) {
            // Inferno: ember sparks rising.
            for (int i = 0; i < 40; i++) {
                float p = ((t * (0.05f + (i % 7) * 0.01f) + i * 0.137f) % 1f);
                Draw.circle(x + ((i * 0.618f) % 1f) * w, y + (1 - p) * h, 0.8f, Colors.withAlpha(0xFFD27A, (1 - p) * 0.8f));
            }
        }
        Gl.end2D();
    }

    private static void ribbons2D(int k, float x, float y, float w, float h, float t, int accent) {
        int bands = k == 4 ? 3 : k == 2 ? 2 : 1;
        Tessellator tes = Tessellator.getInstance();
        WorldRenderer wr = tes.getWorldRenderer();
        for (int b = 0; b < bands; b++) {
            int c = k == 4 && b == 1 ? 0xFFB57CFF : accent;
            float base = (k == 2 ? 0.35f : 0.28f) + b * 0.09f, amp = 0.06f, len = k == 0 ? 0.04f : 0.16f;
            wr.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
            for (int i = 0; i <= 40; i++) {
                float u = i / 40f;
                float yy = y + (base + amp * (float) Math.sin(u * 7 + t * 0.4f + b * 1.7f)) * h;
                float a = (float) Math.sin(u * Math.PI) * (k == 0 ? 0.12f : 0.35f);
                int top = Colors.withAlpha(c, 0), mid = Colors.withAlpha(c, a);
                v(wr, x + u * w, yy - len * h, top);
                v(wr, x + u * w, yy, mid);
            }
            tes.draw();
        }
    }

    private static void v(WorldRenderer wr, double x, double y, int c) {
        wr.pos(x, y, 0).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    private static void v3(WorldRenderer wr, double x, double y, double z, int c) {
        wr.pos(x, y, z).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    /**
     * The sky as a camera-centred dome in the current 3D matrix (rotation
     * only is used; draw it first). Restores GL state.
     */
    public static void drawDome(String id, float time, float alpha) {
        int k = index(id);
        int[] s = STYLE[k];
        float t = General.calm() ? 0 : time;
        GlStateManager.pushMatrix();
        GlStateManager.disableFog();
        GlStateManager.disableTexture2D();
        GlStateManager.disableLighting();
        GlStateManager.disableCull();
        GlStateManager.disableAlpha();
        GlStateManager.enableBlend();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.depthMask(false);
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        double r = 90;
        Tessellator tes = Tessellator.getInstance();
        WorldRenderer wr = tes.getWorldRenderer();
        int rings = 10, seg = 32;
        // Below the horizon the horizon colour continues (darker) so the floor never shows sky.
        for (int ri = -rings; ri < rings; ri++) {
            double e0 = ri / (double) rings * Math.PI / 2, e1 = (ri + 1) / (double) rings * Math.PI / 2;
            int c0 = Colors.alpha(elev(s, e0), alpha), c1 = Colors.alpha(elev(s, e1), alpha);
            wr.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
            for (int i = 0; i <= seg; i++) {
                double a = i * Math.PI * 2 / seg;
                v3(wr, Math.cos(a) * Math.cos(e1) * r, Math.sin(e1) * r, Math.sin(a) * Math.cos(e1) * r, c1);
                v3(wr, Math.cos(a) * Math.cos(e0) * r, Math.sin(e0) * r, Math.sin(a) * Math.cos(e0) * r, c0);
            }
            tes.draw();
        }
        // Aurora / rift curtains: vertical translucent bands over the north.
        if (k == 4 || k == 2) {
            int bands = k == 4 ? 3 : 2;
            for (int b = 0; b < bands; b++) {
                int c = k == 4 && b == 1 ? 0xFFB57CFF : s[6];
                wr.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
                for (int i = 0; i <= 48; i++) {
                    double a = -1.2 + i / 48.0 * 2.4 + b * 0.3;
                    double rr = r * 0.85, wave = Math.sin(i * 0.4 + t * 0.5 + b) * 6;
                    double cx = Math.sin(a) * rr, cz = -Math.cos(a) * rr + wave;
                    float fa = (float) Math.sin(i / 48.0 * Math.PI) * 0.4f * alpha;
                    v3(wr, cx, 18 + b * 6, cz, Colors.withAlpha(c, fa));
                    v3(wr, cx, 45 + b * 6 + wave, cz, Colors.withAlpha(c, 0));
                }
                tes.draw();
            }
        }
        // Stars and clouds as sprites.
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        GlStateManager.enableTexture2D();
        if (s[5] > 0) {
            Minecraft.getMinecraft().getTextureManager().bindTexture(Textures.twinkle());
            wr.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
            for (int i = 0; i < STARS; i++) {
                float tw = 0.55f + 0.45f * (float) Math.sin(t * (1.5f + SB[i] * 2) + i);
                billboard(wr, SX[i] * r * 0.95, SY[i] * r * 0.95, SZ[i] * r * 0.95, 0.7 + SB[i] * 1.1, Colors.withAlpha(0xFFFFFF, SB[i] * tw * alpha * s[5] / 100f));
            }
            tes.draw();
        }
        if (s[4] > 0) {
            Minecraft.getMinecraft().getTextureManager().bindTexture(Textures.glow());
            wr.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
            for (int i = 0; i < CLOUDS; i++) {
                double a = (CX[i] + t * CV[i] * 0.3) * Math.PI * 2, el = 0.05 + (1 - CY[i]) * 0.5;
                double rr = r * 0.8;
                billboard(wr, Math.cos(a) * Math.cos(el) * rr, Math.sin(el) * rr, Math.sin(a) * Math.cos(el) * rr, CS[i] * 70, Colors.withAlpha(s[3], s[4] / 100f * 0.45f * alpha));
            }
            tes.draw();
        }
        if (k == 1) {
            Minecraft.getMinecraft().getTextureManager().bindTexture(Textures.glow());
            wr.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_COLOR);
            billboard(wr, 40, 50, -50, 22, Colors.withAlpha(0xFF6A5A, 0.4f * alpha));
            billboard(wr, 40, 50, -50, 6, Colors.withAlpha(0xFFE6DA, alpha));
            tes.draw();
        }
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.depthMask(true);
        GlStateManager.shadeModel(GL11.GL_FLAT);
        GlStateManager.enableAlpha();
        GlStateManager.enableCull();
        GlStateManager.disableBlend();
        GlStateManager.color(1, 1, 1, 1);
        GlStateManager.popMatrix();
    }

    private static int elev(int[] s, double e) {
        double f = e / (Math.PI / 2);
        if (f < 0) return Colors.mix(s[2], 0xFF05060A, (float) Math.min(1, -f * 4));
        return f < 0.35 ? Colors.mix(s[2], s[1], (float) (f / 0.35)) : Colors.mix(s[1], s[0], (float) ((f - 0.35) / 0.65));
    }

    /** A quad facing the dome centre (the camera). */
    private static void billboard(WorldRenderer wr, double x, double y, double z, double s, int c) {
        double l = Math.sqrt(x * x + y * y + z * z);
        double nx = x / l, ny = y / l, nz = z / l;
        // Right = up × n, up' = n × right.
        double rx = nz, ry = 0, rz = -nx, rl = Math.sqrt(rx * rx + rz * rz);
        if (rl < 1e-6) {
            rx = 1;
            rz = 0;
            rl = 1;
        }
        rx /= rl;
        rz /= rl;
        double ux = ny * rz - nz * ry, uy = nz * rx - nx * rz, uz = nx * ry - ny * rx;
        int r = (c >> 16) & 255, g = (c >> 8) & 255, b = c & 255, a = c >>> 24;
        wr.pos(x - (rx + ux) * s, y - (ry + uy) * s, z - (rz + uz) * s).tex(0, 0).color(r, g, b, a).endVertex();
        wr.pos(x + (rx - ux) * s, y + (ry - uy) * s, z + (rz - uz) * s).tex(1, 0).color(r, g, b, a).endVertex();
        wr.pos(x + (rx + ux) * s, y + (ry + uy) * s, z + (rz + uz) * s).tex(1, 1).color(r, g, b, a).endVertex();
        wr.pos(x - (rx - ux) * s, y - (ry - uy) * s, z - (rz - uz) * s).tex(0, 1).color(r, g, b, a).endVertex();
    }
}
