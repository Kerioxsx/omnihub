package dev.vesper.client.fx;

import java.nio.FloatBuffer;
import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;

/**
 * The camera as seen from the current model-view matrix: its right and up
 * axes (for billboards) and its position (for camera-facing lines). Reading
 * it from the matrix works in the world and on the showroom stage alike,
 * where RenderManager's view angles mean nothing.
 */
public final class View {
    private static final FloatBuffer M = BufferUtils.createFloatBuffer(16);

    /** Camera right / up axes in object space (unit length). */
    public static double rx = 1, ry, rz, ux, uy = 1, uz;
    /** Camera position in object space. */
    public static double cx, cy, cz;

    private View() {
    }

    /** Call after the matrix for the space being drawn is set up. */
    public static void capture() {
        M.clear();
        GL11.glGetFloat(GL11.GL_MODELVIEW_MATRIX, M);
        float m0 = M.get(0), m1 = M.get(1), m2 = M.get(2);
        float m4 = M.get(4), m5 = M.get(5), m6 = M.get(6);
        float m8 = M.get(8), m9 = M.get(9), m10 = M.get(10);
        float tx = M.get(12), ty = M.get(13), tz = M.get(14);
        double s2 = m0 * m0 + m4 * m4 + m8 * m8;
        if (s2 < 1e-12) s2 = 1;
        cx = -(m0 * tx + m1 * ty + m2 * tz) / s2;
        cy = -(m4 * tx + m5 * ty + m6 * tz) / s2;
        cz = -(m8 * tx + m9 * ty + m10 * tz) / s2;
        double rl = Math.sqrt(s2);
        rx = m0 / rl;
        ry = m4 / rl;
        rz = m8 / rl;
        double ul = Math.sqrt(m1 * m1 + m5 * m5 + m9 * m9);
        if (ul < 1e-9) ul = 1;
        ux = m1 / ul;
        uy = m5 / ul;
        uz = m9 / ul;
    }
}
