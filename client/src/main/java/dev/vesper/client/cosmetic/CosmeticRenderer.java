package dev.vesper.client.cosmetic;

import dev.vesper.client.module.General;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.ObjModel;
import net.minecraft.client.renderer.GlStateManager;
import org.lwjgl.opengl.GL11;

/**
 * Draws every enabled cosmetic for one pose. The matrix origin is at the
 * player's feet (+y up, 1 unit = 1 block, not yet rotated by body yaw).
 * The same code serves the in-world player, the title preview and the
 * showroom stage. Restores all GL state it changes.
 */
public final class CosmeticRenderer {
    private CosmeticRenderer() {
    }

    /** Finger-tip positions in the wing mesh (from client/models/vesper_models.py, OBJ axes). */
    private static final float[][] WING_TIPS = {{-1.30f, 0.78f, -0.30f}, {-1.42f, 0.22f, -0.36f}, {-1.18f, -0.30f, -0.34f}, {-0.78f, -0.62f, -0.26f}};

    public static void draw(Pose p) {
        if (!General.fx()) return;
        Cosmetics.Set c = Cosmetics.set();
        if (c == null) return;
        float level = CosmeticStyle.level() * p.alpha;
        if (level <= 0.01f) return;
        GlStateManager.pushMatrix();
        GlStateManager.disableLighting();
        GlStateManager.disableCull();
        GlStateManager.enableBlend();
        GlStateManager.disableAlpha();
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        GlStateManager.enableDepth();
        float drop = p.sneakDrop();

        // Solid pass (depth writes on): wings, crown, token discs.
        if (c.wings.enabled()) wings(p, c.wings, level, drop, true);
        if (c.comet.enabled() && c.comet.crown.on()) crown(p, c.comet, level, drop);

        // Glow pass: additive, no depth writes.
        GlStateManager.depthMask(false);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        Sprites.camera();
        if (c.wings.enabled()) wings(p, c.wings, level, drop, false);
        if (c.floor.enabled()) floorRing(p, c.floor, level);
        if (c.halo.enabled()) halo(p, c.halo, level, drop);
        if (c.orbit.enabled()) orbit(p, c.orbit, level, drop);
        if (c.comet.enabled()) comet(p, c.comet, level, drop);
        if (c.glints.enabled()) glints(p, c.glints, level);
        Sprites.end();
        Solid.end();

        GlStateManager.depthMask(true);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
        GlStateManager.shadeModel(GL11.GL_FLAT);
        GlStateManager.enableAlpha();
        GlStateManager.enableCull();
        GlStateManager.enableTexture2D();
        GlStateManager.disableBlend();
        GlStateManager.color(1, 1, 1, 1);
        GlStateManager.popMatrix();
    }

    // ---------- wings ----------

    private static void wings(Pose p, Cosmetics.Wings m, float level, float drop, boolean solid) {
        Palette pal = Palette.pick(m.palette.get());
        Palette.Wing col = Palette.wing(m.colourway.get(), pal);
        ObjModel model = ObjModel.get("wing.obj");
        float size = m.size.f();
        float ph = p.scene == null ? 0 : p.scene.wingPhase;
        float amp = m.flap.f() * (0.6f + 0.4f * (p.scene == null ? 0 : p.scene.fastBlend));
        float flap = (float) Math.sin(ph * Math.PI * 2) * 28 * amp;
        float fold = 18 + (float) Math.cos(ph * Math.PI * 2) * 10 * amp;
        if (p.sneak() > 0) fold += 25 * p.sneak();
        GlStateManager.pushMatrix();
        GlStateManager.rotate(-p.bodyYaw, 0, 1, 0);
        GlStateManager.translate(0, 1.32f - drop, -0.16f);
        if (p.sneak() > 0) GlStateManager.rotate(28 * p.sneak(), 1, 0, 0);
        for (int side = 0; side < 2; side++) {
            GlStateManager.pushMatrix();
            // The OBJ wing is the right wing: it extends -X (the player's right) and sweeps -Z (behind).
            if (side == 1) GlStateManager.scale(-1, 1, 1);
            GlStateManager.translate(-0.06f, 0, 0);
            GlStateManager.rotate(-fold, 0, 1, 0);
            GlStateManager.rotate(-(flap + 6), 0, 0, 1);
            GlStateManager.scale(size, size, size);
            if (solid) {
                if (model.ok) {
                    model.draw("membrane", Colors.withAlpha(col.membrane, 0.86f * level));
                    model.draw("bone", Colors.withAlpha(col.rib, level));
                    model.draw("claw", Colors.withAlpha(0xE8E8F0, level));
                } else {
                    fallbackWing(col, level);
                }
            } else {
                Sprites.camera();
                Sprites.beginGlow();
                int edge = Colors.withAlpha(col.edge, 0.85f * level * m.glow.f());
                for (float[] t : WING_TIPS) {
                    Sprites.add(t[0], t[1], t[2], 0.11f, edge);
                    Sprites.add(t[0], t[1], t[2], 0.035f, Colors.withAlpha(0xFFFFFF, level));
                }
                // A soft glow along the leading bone.
                for (int i = 1; i <= 4; i++) Sprites.add(-0.2f * i, 0.13f * i, -0.04f * i, 0.09f, Colors.withAlpha(col.edge, 0.25f * level * m.glow.f()));
                Sprites.end();
            }
            GlStateManager.popMatrix();
        }
        GlStateManager.popMatrix();
    }

    /** A flat triangle fan if the mesh is missing. */
    private static void fallbackWing(Palette.Wing col, float level) {
        Solid.begin(GL11.GL_TRIANGLES);
        float[][] pts = {{0, 0, 0}, {-0.8f, 0.52f, -0.16f}, {-1.3f, 0.78f, -0.3f}, {-1.42f, 0.22f, -0.36f}, {-1.18f, -0.3f, -0.34f}, {-0.78f, -0.62f, -0.26f}, {-0.1f, -0.3f, -0.06f}};
        int c = Colors.withAlpha(col.membrane, 0.85f * level);
        for (int i = 1; i + 1 < pts.length; i++) {
            Solid.v(pts[0][0], pts[0][1], pts[0][2], c);
            Solid.v(pts[i][0], pts[i][1], pts[i][2], c);
            Solid.v(pts[i + 1][0], pts[i + 1][1], pts[i + 1][2], c);
        }
        Solid.end();
    }

    // ---------- comet plume + crown ----------

    private static float cometBase(Pose p, float drop) {
        return 2.28f - drop + 0.04f * (float) Math.sin(p.time * 1.6f);
    }

    private static void crown(Pose p, Cosmetics.Comet m, float level, float drop) {
        Palette pal = Palette.pick(m.palette.get());
        ObjModel model = ObjModel.get("crown.obj");
        if (!model.ok) return;
        GlStateManager.pushMatrix();
        GlStateManager.translate(0, cometBase(p, drop) - 0.24f, 0);
        GlStateManager.rotate(p.time * 20, 0, 1, 0);
        GlStateManager.scale(1.1f, 1.1f, 1.1f);
        model.draw("band", Colors.withAlpha(pal.wingShadow, level));
        model.draw("thorn", Colors.withAlpha(Colors.mix(pal.wingRib, pal.accent, 0.35f), level));
        GlStateManager.popMatrix();
    }

    private static void comet(Pose p, Cosmetics.Comet m, float level, float drop) {
        Palette pal = Palette.pick(m.palette.get());
        float base = cometBase(p, drop);
        float len = m.length.f();
        String v = m.variant.get();
        float t = p.time;
        Sprites.beginGlow();
        // Tail: rises up and slightly back, swaying.
        int n = Sprites.seg(40);
        float px = 0, py = base, pz = 0;
        for (int i = 1; i <= n; i++) {
            float u = i / (float) n;
            float sway = (float) Math.sin(t * 2.2f - u * 4) * 0.12f * u + Sprites.noise(t * 1.3f + u * 3) * 0.06f * u;
            float x = sway, y = base + u * len, z = -u * 0.18f * len + (float) Math.cos(t * 1.7f - u * 3) * 0.05f * u;
            int c = u < 0.35f ? Colors.mix(pal.tailHot, pal.tailMid, u / 0.35f) : Colors.mix(pal.tailMid, pal.tailCold, (u - 0.35f) / 0.65f);
            float w = (v.equals("wisp") ? 0.10f : 0.22f) * (1 - u * 0.85f);
            Sprites.line(px, py, pz, x, y, z, w, Colors.withAlpha(c, (1 - u) * 0.9f * level));
            px = x;
            py = y;
            pz = z;
        }
        // Head.
        float head = v.equals("orb") ? 0.26f : 0.18f;
        float pulse = 1 + 0.08f * (float) Math.sin(t * 5);
        Sprites.add(0, base, 0, head * 2.6f * pulse, Colors.withAlpha(pal.accent, 0.45f * level * m.glow.f()));
        Sprites.add(0, base, 0, head * pulse, Colors.withAlpha(pal.coreTint, level));
        Sprites.add(0, base, 0, head * 0.45f, Colors.withAlpha(0xFFFFFF, level));
        Sprites.end();
        // Sparks peeling off the tail (deterministic by time, no allocation).
        Sprites.beginTwinkle();
        int sparks = Math.round(m.sparks.f() * (General.detail() == 0 ? 0.5f : 1f));
        for (int i = 0; i < sparks; i++) {
            float life = (t * 0.7f + Sprites.hash(i, 3)) % 1f;
            float sx = (Sprites.hash(i, 1) - 0.5f) * 0.5f * life, sz = (Sprites.hash(i, 2) - 0.5f) * 0.5f * life;
            Sprites.add(sx, base + life * len * 0.9f, sz - life * 0.15f, 0.05f * (1 - life), Colors.withAlpha(pal.spark, (1 - life) * level));
        }
        Sprites.end();
    }

    // ---------- orbit tokens ----------

    private static void orbit(Pose p, Cosmetics.Orbit m, float level, float drop) {
        Palette pal = Palette.pick(m.palette.get());
        int n = m.count.i();
        float r = m.radius.f(), y0 = 1.05f - drop;
        float spin = p.time * m.speed.f() * 60;
        for (int i = 0; i < n; i++) {
            double a = Math.toRadians(spin + i * 360.0 / n);
            float x = (float) Math.cos(a) * r, z = (float) Math.sin(a) * r;
            float y = y0 + 0.08f * (float) Math.sin(p.time * 2 + i);
            float s = 0.13f * m.size.f();
            // Disc (camera-facing): edge glow, fill, glyph.
            Sprites.beginGlow();
            Sprites.add(x, y, z, s * 2.4f, Colors.withAlpha(pal.glow, 0.35f * level));
            Sprites.end();
            Solid.begin(GL11.GL_TRIANGLES);
            disc(x, y, z, s, Colors.withAlpha(pal.token, 0.9f * level), Colors.withAlpha(pal.tokenEdge, 0.9f * level));
            glyph(i % 6, x, y, z, s * 0.62f, Colors.withAlpha(0xFFFFFF, level));
            Solid.end();
        }
    }

    private static void disc(float x, float y, float z, float r, int fill, int edge) {
        int seg = Sprites.seg(20);
        for (int i = 0; i < seg; i++) {
            double a0 = i * Math.PI * 2 / seg, a1 = (i + 1) * Math.PI * 2 / seg;
            float c0 = (float) Math.cos(a0), s0 = (float) Math.sin(a0), c1 = (float) Math.cos(a1), s1 = (float) Math.sin(a1);
            bv(x, y, z, 0, 0, fill);
            bv(x, y, z, c0 * r * 0.82f, s0 * r * 0.82f, fill);
            bv(x, y, z, c1 * r * 0.82f, s1 * r * 0.82f, fill);
            // rim
            bv(x, y, z, c0 * r * 0.82f, s0 * r * 0.82f, edge);
            bv(x, y, z, c0 * r, s0 * r, edge);
            bv(x, y, z, c1 * r, s1 * r, edge);
            bv(x, y, z, c0 * r * 0.82f, s0 * r * 0.82f, edge);
            bv(x, y, z, c1 * r, s1 * r, edge);
            bv(x, y, z, c1 * r * 0.82f, s1 * r * 0.82f, edge);
        }
    }

    /** A vertex on the camera plane through (x,y,z), offset (u,v) along the screen axes. */
    private static void bv(float x, float y, float z, float u, float v, int c) {
        Solid.v(x + Sprites.rx * u + Sprites.ux * v, y + Sprites.ry * u + Sprites.uy * v, z + Sprites.rz * u + Sprites.uz * v, c);
    }

    private static void bline(float x, float y, float z, float u0, float v0, float u1, float v1, float w, int c) {
        float du = u1 - u0, dv = v1 - v0, l = (float) Math.sqrt(du * du + dv * dv);
        if (l < 1e-6f) return;
        float nu = -dv / l * w / 2, nv = du / l * w / 2;
        bv(x, y, z, u0 + nu, v0 + nv, c);
        bv(x, y, z, u1 + nu, v1 + nv, c);
        bv(x, y, z, u1 - nu, v1 - nv, c);
        bv(x, y, z, u0 + nu, v0 + nv, c);
        bv(x, y, z, u1 - nu, v1 - nv, c);
        bv(x, y, z, u0 - nu, v0 - nv, c);
    }

    /** Six original geometric glyphs (no logos or symbols with outside meaning). */
    private static void glyph(int k, float x, float y, float z, float s, int c) {
        float w = s * 0.18f;
        switch (k) {
            case 0: // stacked chevrons
                bline(x, y, z, -s * 0.6f, -s * 0.1f, 0, s * 0.45f, w, c);
                bline(x, y, z, 0, s * 0.45f, s * 0.6f, -s * 0.1f, w, c);
                bline(x, y, z, -s * 0.6f, -s * 0.6f, 0, -s * 0.05f, w, c);
                bline(x, y, z, 0, -s * 0.05f, s * 0.6f, -s * 0.6f, w, c);
                break;
            case 1: // hollow rhombus with a dot
                bline(x, y, z, 0, s * 0.7f, s * 0.55f, 0, w, c);
                bline(x, y, z, s * 0.55f, 0, 0, -s * 0.7f, w, c);
                bline(x, y, z, 0, -s * 0.7f, -s * 0.55f, 0, w, c);
                bline(x, y, z, -s * 0.55f, 0, 0, s * 0.7f, w, c);
                bline(x, y, z, -w, 0, w, 0, w * 1.6f, c);
                break;
            case 2: // three slanted bars
                for (int i = -1; i <= 1; i++) bline(x, y, z, -s * 0.55f, i * s * 0.38f - s * 0.12f, s * 0.55f, i * s * 0.38f + s * 0.12f, w, c);
                break;
            case 3: // split bolt
                bline(x, y, z, s * 0.2f, s * 0.7f, -s * 0.25f, 0, w, c);
                bline(x, y, z, -s * 0.25f, 0, s * 0.25f, 0, w, c);
                bline(x, y, z, s * 0.25f, 0, -s * 0.2f, -s * 0.7f, w, c);
                break;
            case 4: // open hexagon
                for (int i = 0; i < 5; i++) {
                    double a0 = Math.PI / 2 + i * Math.PI / 3, a1 = a0 + Math.PI / 3;
                    bline(x, y, z, (float) Math.cos(a0) * s * 0.65f, (float) Math.sin(a0) * s * 0.65f, (float) Math.cos(a1) * s * 0.65f, (float) Math.sin(a1) * s * 0.65f, w, c);
                }
                break;
            default: // four-point spark
                bline(x, y, z, 0, s * 0.75f, 0, -s * 0.75f, w, c);
                bline(x, y, z, -s * 0.75f, 0, s * 0.75f, 0, w, c);
                break;
        }
    }

    // ---------- halo, floor ring, glints ----------

    private static void halo(Pose p, Cosmetics.Halo m, float level, float drop) {
        Palette pal = Palette.pick(m.palette.get());
        float y = 2.12f - drop + 0.03f * (float) Math.sin(p.time * 1.8f);
        float r = 0.3f;
        Solid.begin(GL11.GL_TRIANGLES);
        int seg = Sprites.seg(40);
        int c = Colors.withAlpha(pal.ring, 0.9f * level);
        for (int i = 0; i < seg; i++) {
            double a0 = i * Math.PI * 2 / seg, a1 = (i + 1) * Math.PI * 2 / seg;
            float c0 = (float) Math.cos(a0), s0 = (float) Math.sin(a0), c1 = (float) Math.cos(a1), s1 = (float) Math.sin(a1);
            float r1 = r - 0.025f, r2 = r + 0.025f;
            Solid.v(c0 * r1, y, s0 * r1, c);
            Solid.v(c0 * r2, y, s0 * r2, c);
            Solid.v(c1 * r2, y, s1 * r2, c);
            Solid.v(c0 * r1, y, s0 * r1, c);
            Solid.v(c1 * r2, y, s1 * r2, c);
            Solid.v(c1 * r1, y, s1 * r1, c);
        }
        Solid.end();
        Sprites.beginGlow();
        for (int i = 0; i < 12; i++) {
            double a = i * Math.PI / 6 + p.time * 0.8;
            Sprites.add((float) Math.cos(a) * r, y, (float) Math.sin(a) * r, 0.1f, Colors.withAlpha(pal.glow, 0.35f * level));
        }
        Sprites.end();
    }

    private static void floorRing(Pose p, Cosmetics.FloorRing m, float level) {
        Palette pal = Palette.pick(m.palette.get());
        float y = p.ground + 0.02f;
        float r = m.radius.f();
        Solid.begin(GL11.GL_TRIANGLES);
        int seg = Sprites.seg(48);
        float rot = p.time * 0.6f;
        for (int i = 0; i < seg; i++) {
            if (i % 4 == 3) continue; // dashes
            double a0 = rot + i * Math.PI * 2 / seg, a1 = rot + (i + 1) * Math.PI * 2 / seg;
            float c0 = (float) Math.cos(a0), s0 = (float) Math.sin(a0), c1 = (float) Math.cos(a1), s1 = (float) Math.sin(a1);
            int c = Colors.withAlpha(pal.ring, 0.75f * level);
            float r1 = r - 0.02f, r2 = r + 0.02f;
            Solid.v(c0 * r1, y, s0 * r1, c);
            Solid.v(c0 * r2, y, s0 * r2, c);
            Solid.v(c1 * r2, y, s1 * r2, c);
            Solid.v(c0 * r1, y, s0 * r1, c);
            Solid.v(c1 * r2, y, s1 * r2, c);
            Solid.v(c1 * r1, y, s1 * r1, c);
        }
        // Soft inner disc.
        int in = Colors.withAlpha(pal.glow, 0.18f * level), out = Colors.withAlpha(pal.glow, 0);
        for (int i = 0; i < seg; i++) {
            double a0 = i * Math.PI * 2 / seg, a1 = (i + 1) * Math.PI * 2 / seg;
            Solid.v(0, y, 0, in);
            Solid.v((float) Math.cos(a0) * r, y, (float) Math.sin(a0) * r, out);
            Solid.v((float) Math.cos(a1) * r, y, (float) Math.sin(a1) * r, out);
        }
        Solid.end();
    }

    private static void glints(Pose p, Cosmetics.Glints m, float level) {
        Palette pal = Palette.pick("shared");
        Sprites.beginTwinkle();
        int n = Math.round(m.amount.f() * (General.detail() == 0 ? 0.5f : 1f));
        for (int i = 0; i < n; i++) {
            float cyc = 2.2f + Sprites.hash(i, 9) * 1.5f;
            float life = ((p.time + Sprites.hash(i, 7) * cyc) % cyc) / cyc;
            float a = (float) Math.sin(life * Math.PI);
            double ang = Sprites.hash(i, 5) * Math.PI * 2;
            float r = 0.4f + Sprites.hash(i, 6) * 0.5f;
            Sprites.add((float) Math.cos(ang) * r, 0.2f + Sprites.hash(i, 8) * 1.8f + life * 0.3f, (float) Math.sin(ang) * r, 0.06f * a, Colors.withAlpha(pal.spark, a * level));
        }
        Sprites.end();
    }
}
