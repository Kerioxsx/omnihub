package dev.vesper.client.gui;

import dev.vesper.client.module.General;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import java.util.Random;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.opengl.GL11;

/**
 * The charcoal backdrop shared by our menus (VISUAL_SPEC §1–2): diagonal
 * gradient, vignette, the satin curtain bands on the right and twinkling
 * four-point stars. Time is global, so moving between our screens is seamless.
 */
public final class MenuBackdrop {
    private MenuBackdrop() {
    }

    private static final int STARS = 7;
    private static final float[] SX = new float[STARS], SY = new float[STARS], SS = new float[STARS], SP = new float[STARS], SF = new float[STARS];
    private static final int MOTES = 12;
    private static final float[] MX = new float[MOTES], MY = new float[MOTES], MA = new float[MOTES];

    static {
        Random r = new Random(0x5E5BE7L);
        for (int i = 0; i < STARS; i++) {
            SX[i] = 0.80f + r.nextFloat() * 0.18f;
            SY[i] = 0.08f + (i + r.nextFloat() * 0.8f) / STARS * 0.84f;
            SS[i] = 0.012f + r.nextFloat() * 0.006f;
            SP[i] = 2.4f + r.nextFloat() * 1.6f;
            SF[i] = r.nextFloat() * 6.28f;
        }
        for (int i = 0; i < MOTES; i++) {
            MX[i] = r.nextFloat();
            MY[i] = r.nextFloat();
            MA[i] = 0.15f + r.nextFloat() * 0.35f;
        }
    }

    /** Gradient + vignette. */
    public static void base(float w, float h, float alpha) {
        int tl = Colors.withAlpha(0x2A2C31, alpha), mid = Colors.withAlpha(0x18191D, alpha), br = Colors.withAlpha(0x0F1013, alpha);
        int tr = Colors.mix(tl, br, 0.5f), bl = tr;
        float cx = w / 2, cy = h / 2;
        // Four triangles-as-quads meeting in the middle: a 135° three-stop gradient.
        Draw.quad(0, 0, w, 0, cx, cy, cx, cy, tl, tr, mid, mid);
        Draw.quad(w, 0, w, h, cx, cy, cx, cy, tr, br, mid, mid);
        Draw.quad(w, h, 0, h, cx, cy, cx, cy, br, bl, mid, mid);
        Draw.quad(0, h, 0, 0, cx, cy, cx, cy, bl, tl, mid, mid);
        float m = Math.max(w, h);
        Draw.ring(cx, cy, 0.45f * m, 0.9f * m, 0x00000000, Colors.withAlpha(0x000000, 0.55f * alpha));
    }

    /** A very faint diagonal sheen that slides across every 9 s (splash). */
    public static void sheen(float w, float h, float t, float alpha) {
        float p = (t % 9f) / 9f;
        float x = -0.5f * h + p * (w + h);
        float band = 0.5f * h, skew = 0.36f * h;
        int c = Colors.withAlpha(0xFFFFFF, 0.035f * alpha);
        Draw.quad(x - band / 2 + skew, 0, x + skew, 0, x, h, x - band / 2, h, 0x00FFFFFF, c, c, 0x00FFFFFF);
        Draw.quad(x + skew, 0, x + band / 2 + skew, 0, x + band / 2, h, x, h, c, 0x00FFFFFF, 0x00FFFFFF, c);
    }

    /** The five wavy curtain bands. */
    public static void curtains(float w, float h, float t, float alpha) {
        if (alpha <= 0.01f) return;
        float tau = General.calm() ? 0 : t;
        Tessellator tes = Tessellator.getInstance();
        WorldRenderer wr = tes.getWorldRenderer();
        int rows = 48;
        float bandW = 0.062f * w;
        for (int i = 4; i >= 0; i--) {
            // Trough (dark strip) first, then the band over it.
            wr.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
            for (int r = 0; r < rows; r++) {
                float y = r * h / (rows - 1);
                float cxl = centre(i, y, h, w, tau) + 0.035f * w;
                col(wr, cxl - 0.0125f * w, y, 0x000A0B0D);
                col(wr, cxl, y, Colors.withAlpha(0x0A0B0D, 0.35f * alpha));
            }
            tes.draw();
            wr.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
            for (int r = 0; r < rows; r++) {
                float y = r * h / (rows - 1);
                float cxl = centre(i, y, h, w, tau) + 0.035f * w;
                col(wr, cxl, y, Colors.withAlpha(0x0A0B0D, 0.35f * alpha));
                col(wr, cxl + 0.0125f * w, y, 0x000A0B0D);
            }
            tes.draw();
            for (int half = 0; half < 2; half++) {
                wr.begin(GL11.GL_TRIANGLE_STRIP, DefaultVertexFormats.POSITION_COLOR);
                for (int r = 0; r < rows; r++) {
                    float y = r * h / (rows - 1);
                    float x = centre(i, y, h, w, tau);
                    float dim = 0.75f + 0.25f * (float) Math.sin(y / h * Math.PI);
                    float left = x - bandW * 0.35f, crest = x, right = x + bandW * 0.65f;
                    int cCrest = Colors.withAlpha(0xD9DBE0, 0.55f * dim * alpha);
                    if (half == 0) {
                        col(wr, left, y, 0x007E828C);
                        col(wr, crest, y, cCrest);
                    } else {
                        col(wr, crest, y, cCrest);
                        col(wr, right, y, 0x0055585F);
                    }
                }
                tes.draw();
            }
        }
    }

    private static float centre(int i, float y, float h, float w, float tau) {
        // Broad folds that stay roughly parallel (small phase step per band), like satin.
        double yy = y / h;
        // Amplitude swells and fades along the height so folds don't look like a sine chart.
        double env = 0.55 + 0.45 * Math.sin(Math.PI * (0.15 + 0.85 * yy) + 0.5 * i);
        double a = 0.026 * w * (1 + 0.12 * i) * env, lambda = 0.58 + 0.025 * i, phi = 0.42 * i, om = 2 * Math.PI / 14;
        return (float) (w * (0.735 + 0.062 * i) + a * Math.sin(2 * Math.PI * yy / lambda + phi + tau * om) + 0.22 * a * Math.sin(2 * Math.PI * yy / 1.1 + 0.6 * phi - tau * om * 0.6));
    }

    private static void col(WorldRenderer wr, float x, float y, int c) {
        wr.pos(x, y, 0).color((c >> 16) & 255, (c >> 8) & 255, c & 255, c >>> 24).endVertex();
    }

    /** The seven twinkling stars along the curtain. One sparkles every 2.5 s. */
    public static void stars(float w, float h, float t, float alpha) {
        int sparkling = (int) (t / 2.5f) % STARS;
        float sp = (t % 2.5f) / 0.3f;
        for (int i = 0; i < STARS; i++) {
            float tw = General.calm() ? 0.6f : 0.35f + 0.45f * (0.5f + 0.5f * (float) Math.sin(t * 6.283f / SP[i] + SF[i]));
            float size = SS[i] * h, rot = 0;
            if (i == sparkling && sp < 1 && !General.calm()) {
                float b = Ease.bell(sp);
                size *= 1 + 0.6f * b;
                rot = (float) (Math.PI / 4 * b);
            }
            Draw.star4(SX[i] * w, SY[i] * h, size, rot, Colors.withAlpha(0xA3A7B0, tw * alpha));
        }
    }

    /** Slow upward-drifting motes (splash). */
    public static void motes(float w, float h, float t, float alpha) {
        for (int i = 0; i < MOTES; i++) {
            float y = (MY[i] - (General.calm() ? 0 : t * 0.004f * 2)) % 1f;
            if (y < 0) y += 1;
            Draw.circle(MX[i] * w, y * h, 0.6f + (i % 2) * 0.4f, Colors.withAlpha(0xFFFFFF, MA[i] * alpha));
        }
    }
}
