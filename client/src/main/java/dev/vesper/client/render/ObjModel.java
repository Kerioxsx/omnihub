package dev.vesper.client.render;

import dev.vesper.client.VesperClient;
import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import org.lwjgl.opengl.GL11;

/**
 * A tiny Wavefront OBJ loader for Vesper's own meshes (made in Blender, see
 * client/models/). Triangles and quads, grouped by `o`/`g` name; each group
 * is drawn in a colour given at draw time, with simple directional shading
 * baked from the face normals. Read from the mod's own assets.
 */
public final class ObjModel {
    private static final Map<String, ObjModel> CACHE = new HashMap<String, ObjModel>();

    /** Per group: triangle vertex positions (x,y,z ×3 per tri) and a shade per tri. */
    private final Map<String, float[]> tris = new HashMap<String, float[]>();
    private final Map<String, float[]> shade = new HashMap<String, float[]>();
    public final List<String> groups = new ArrayList<String>();
    public final boolean ok;

    private ObjModel(String path) {
        boolean good = false;
        try (InputStream in = ObjModel.class.getResourceAsStream("/assets/vesper/models/" + path)) {
            if (in == null) throw new IllegalStateException("missing");
            BufferedReader r = new BufferedReader(new InputStreamReader(in, StandardCharsets.UTF_8));
            List<float[]> v = new ArrayList<float[]>();
            Map<String, List<Float>> acc = new HashMap<String, List<Float>>();
            String group = "default";
            String line;
            while ((line = r.readLine()) != null) {
                line = line.trim();
                if (line.startsWith("v ")) {
                    String[] p = line.split("\\s+");
                    v.add(new float[] {Float.parseFloat(p[1]), Float.parseFloat(p[2]), Float.parseFloat(p[3])});
                } else if (line.startsWith("o ") || line.startsWith("g ")) {
                    group = line.substring(2).trim();
                    // Blender names objects "Name_Mesh"; keep the part before the first '_' or '.'.
                    int cut = group.indexOf('.');
                    if (cut > 0) group = group.substring(0, cut);
                } else if (line.startsWith("f ")) {
                    String[] p = line.split("\\s+");
                    int n = p.length - 1;
                    int[] idx = new int[n];
                    for (int i = 0; i < n; i++) {
                        String s = p[i + 1];
                        int slash = s.indexOf('/');
                        int k = Integer.parseInt(slash < 0 ? s : s.substring(0, slash));
                        idx[i] = k < 0 ? v.size() + k : k - 1;
                    }
                    List<Float> l = acc.get(group);
                    if (l == null) {
                        l = new ArrayList<Float>();
                        acc.put(group, l);
                        groups.add(group);
                    }
                    for (int i = 1; i + 1 < n; i++) {
                        for (int k : new int[] {idx[0], idx[i], idx[i + 1]}) {
                            float[] q = v.get(k);
                            l.add(q[0]);
                            l.add(q[1]);
                            l.add(q[2]);
                        }
                    }
                }
            }
            for (Map.Entry<String, List<Float>> e : acc.entrySet()) {
                List<Float> l = e.getValue();
                float[] a = new float[l.size()];
                for (int i = 0; i < a.length; i++) a[i] = l.get(i);
                tris.put(e.getKey(), a);
                float[] sh = new float[a.length / 9];
                for (int t = 0; t < sh.length; t++) {
                    int o = t * 9;
                    float ux = a[o + 3] - a[o], uy = a[o + 4] - a[o + 1], uz = a[o + 5] - a[o + 2];
                    float vx = a[o + 6] - a[o], vy = a[o + 7] - a[o + 1], vz = a[o + 8] - a[o + 2];
                    float nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
                    float len = (float) Math.sqrt(nx * nx + ny * ny + nz * nz);
                    if (len < 1e-9f) len = 1;
                    // Two-sided: light from above-front, |dot| so back faces are lit too.
                    float d = Math.abs((nx * 0.35f + ny * 0.85f + nz * 0.4f) / len);
                    sh[t] = 0.55f + 0.45f * d;
                }
                shade.put(e.getKey(), sh);
            }
            good = !tris.isEmpty();
        } catch (Exception ex) {
            VesperClient.LOG.warn("Model {} unavailable ({}); using the built-in shape", path, ex.getMessage());
        }
        ok = good;
    }

    public static ObjModel get(String path) {
        ObjModel m = CACHE.get(path);
        if (m == null) {
            m = new ObjModel(path);
            CACHE.put(path, m);
        }
        return m;
    }

    public boolean has(String group) {
        return tris.containsKey(group);
    }

    /** Draws one group in `argb` (shading multiplies RGB). Untextured; caller sets blending/depth. */
    public void draw(String group, int argb) {
        float[] a = tris.get(group);
        if (a == null) return;
        float[] sh = shade.get(group);
        GlStateManager.disableTexture2D();
        Tessellator t = Tessellator.getInstance();
        WorldRenderer w = t.getWorldRenderer();
        w.begin(GL11.GL_TRIANGLES, DefaultVertexFormats.POSITION_COLOR);
        int r = (argb >> 16) & 255, g = (argb >> 8) & 255, b = argb & 255, al = argb >>> 24;
        for (int i = 0; i < sh.length; i++) {
            float s = sh[i];
            int rr = (int) (r * s), gg = (int) (g * s), bb = (int) (b * s);
            int o = i * 9;
            w.pos(a[o], a[o + 1], a[o + 2]).color(rr, gg, bb, al).endVertex();
            w.pos(a[o + 3], a[o + 4], a[o + 5]).color(rr, gg, bb, al).endVertex();
            w.pos(a[o + 6], a[o + 7], a[o + 8]).color(rr, gg, bb, al).endVertex();
        }
        t.draw();
    }
}
