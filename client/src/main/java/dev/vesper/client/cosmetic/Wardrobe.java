package dev.vesper.client.cosmetic;

import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.Textures;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraftforge.client.event.RenderWorldLastEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.opengl.GL11;

/**
 * The wardrobe: hats, capes, auras, trails, pets and back items, each an
 * original design drawn from shapes and light (no copied art), each with
 * its own colours and size. Like the other cosmetics, only you see them:
 * on your player, in the menu preview and in the showroom.
 */
public final class Wardrobe {
    private Wardrobe() {
    }

    /** One wearable: a kind, a style and two colours. */
    public static final class Item extends Module {
        final String kind, style;
        public final ColorSetting colour, accent;
        public final NumberSetting size;

        Item(String kind, String style, String name, String desc, int c1, int c2) {
            super("w_" + kind + "_" + style, name, desc, Category.COSMETICS, false, true);
            this.kind = kind;
            this.style = style;
            colour = color("colour", "Colour", c1);
            accent = color("accent", "Accent", c2);
            size = number("size", "Size", 1, 0.6, 1.5, 0.05, "×");
        }
    }

    static final List<Item> ITEMS = new ArrayList<Item>();

    private static void add(String kind, String style, String name, String desc, int c1, int c2) {
        ITEMS.add(new Item(kind, style, name, desc, c1, c2));
    }

    public static void init() {
        // Hats.
        add("hat", "crown", "Gilded crown", "A gold crown with five points and glowing gems.", 0xFFE8B83A, 0xFFFF4A6A);
        add("hat", "tophat", "Top hat", "A tall black top hat with a coloured band.", 0xFF15151A, 0xFFB0303A);
        add("hat", "wizard", "Wizard hat", "A tall pointed hat that bends back, with twinkling stars.", 0xFF2A2A7A, 0xFFFFD86B);
        add("hat", "catears", "Cat ears", "Two pointed ears with pink insides.", 0xFF2B2B30, 0xFFFF9EC4);
        add("hat", "foxears", "Fox ears", "Tall orange ears with white tips.", 0xFFE8742A, 0xFFFFFFFF);
        add("hat", "bunnyears", "Bunny ears", "Long ears that flop a little as you move.", 0xFFF2F2F2, 0xFFFFB0C8);
        add("hat", "antlers", "Antlers", "Branching antlers rising from your head.", 0xFF7A5A3A, 0xFFE8D8B8);
        add("hat", "party", "Party hat", "A striped cone with a glowing pompom.", 0xFF3AA0FF, 0xFFFFD23A);
        add("hat", "flowers", "Flower crown", "A ring of small flowers around your head.", 0xFFFF7AB0, 0xFFFFE07A);
        add("hat", "cowboy", "Cowboy hat", "A wide brown brim and a dented crown.", 0xFF7A4E2A, 0xFF3A2A1A);
        // Capes.
        add("cape", "midnight", "Midnight cape", "Deep blue fading to black, with tiny stars.", 0xFF101A44, 0xFF02030A);
        add("cape", "ember", "Ember cape", "Dark red that glows orange toward the hem and flickers.", 0xFF4A0A0A, 0xFFFF7A1A);
        add("cape", "ocean", "Ocean cape", "Blue bands that roll like waves.", 0xFF0A3A7A, 0xFF3AD0FF);
        add("cape", "aurora", "Aurora cape", "Green, teal and violet light that drifts across it.", 0xFF1AE08A, 0xFF8A3AFF);
        add("cape", "galaxy", "Galaxy cape", "Deep violet with stars that twinkle on it.", 0xFF1A0A3A, 0xFFFFFFFF);
        add("cape", "rainbow", "Rainbow cape", "Every colour, slowly cycling.", 0xFFFF3A3A, 0xFF3A3AFF);
        add("cape", "checker", "Checker cape", "A bold two-colour checker pattern.", 0xFF111111, 0xFFF2F2F2);
        add("cape", "royal", "Royal cape", "Purple with a gold trim.", 0xFF4A1A7A, 0xFFE8B83A);
        add("cape", "frost", "Frost cape", "White to ice blue, with glinting crystals.", 0xFFF2FAFF, 0xFF7AC8FF);
        add("cape", "neon", "Neon cape", "Black with glowing edges in your colour.", 0xFF050508, 0xFF2AF0FF);
        // Auras.
        add("aura", "flame", "Flame aura", "Flames rising around you.", 0xFFFF6A1A, 0xFFFFD23A);
        add("aura", "frost", "Frost aura", "Cold sparkles drifting down around you.", 0xFF9AD8FF, 0xFFFFFFFF);
        add("aura", "void", "Void aura", "Violet motes pulled in toward you.", 0xFF6A1AFF, 0xFF2A0A5A);
        add("aura", "toxic", "Toxic aura", "Green bubbles wobbling up around you.", 0xFF6AFF3A, 0xFFD8FF3A);
        add("aura", "holy", "Holy aura", "Golden light rising, with a ring at your feet.", 0xFFFFE07A, 0xFFFFFFFF);
        add("aura", "blood", "Blood aura", "Red drops falling around you.", 0xFFD0102A, 0xFF5A0010);
        add("aura", "electric", "Electric aura", "Small lightning arcs crackling around you.", 0xFF6AE8FF, 0xFFFFFFFF);
        add("aura", "rainbow", "Rainbow aura", "Colour-cycling lights orbiting you.", 0xFFFF3A8A, 0xFF3AFFD8);
        // Trails.
        add("trail", "stardust", "Stardust trail", "Twinkling dust left where you walk.", 0xFFFFE8A0, 0xFFA0C8FF);
        add("trail", "flame", "Flame trail", "Small flames that burn out behind you.", 0xFFFF6A1A, 0xFFFFD23A);
        add("trail", "bubbles", "Bubble trail", "Bubbles that float up behind you.", 0xFF7AD8FF, 0xFFFFFFFF);
        add("trail", "petals", "Petal trail", "Pink petals drifting down behind you.", 0xFFFF9EC4, 0xFFFFFFFF);
        add("trail", "snow", "Snow trail", "Snowflakes settling where you walk.", 0xFFFFFFFF, 0xFFB8E0FF);
        add("trail", "ink", "Ink trail", "Dark violet wisps fading behind you.", 0xFF5A2AAA, 0xFF1A0A3A);
        add("trail", "hearts", "Heart trail", "Little glowing hearts rising behind you.", 0xFFFF3A6A, 0xFFFFB0C8);
        add("trail", "ribbon", "Ribbon trail", "A glowing ribbon of light following your feet.", 0xFF2AF0FF, 0xFFFF3AE0);
        // Pets.
        add("pet", "wisp", "Wisp", "A small glowing wisp floating by your shoulder.", 0xFF7AF0FF, 0xFFFFFFFF);
        add("pet", "cube", "Cube buddy", "A little cube that turns and bobs beside you.", 0xFF3A8AFF, 0xFFFFFFFF);
        add("pet", "crystal", "Floating crystal", "A spinning crystal with a soft glow.", 0xFFB06AFF, 0xFFFFFFFF);
        add("pet", "planet", "Tiny planet", "A small glowing planet with a ring.", 0xFFFF9A3A, 0xFFFFE0B0);
        add("pet", "fireflies", "Fireflies", "A few fireflies wandering around your head.", 0xFFD8FF6A, 0xFFFFFFA0);
        add("pet", "lantern", "Lantern", "A floating lantern with a warm light.", 0xFF3A2A1A, 0xFFFFB03A);
        // Back.
        add("back", "butterfly", "Butterfly wings", "Two pairs of wings that flutter.", 0xFF3AB0FF, 0xFF1A1A5A);
        add("back", "jetpack", "Jetpack", "Two tanks on your back; flames when you run or fly.", 0xFF8A8A92, 0xFFFF7A1A);
        add("back", "backpack", "Backpack", "A small backpack with a coloured flap.", 0xFF6A4A2A, 0xFFD04A2A);
        add("back", "tail", "Fox tail", "A bushy tail that sways as you move.", 0xFFE8742A, 0xFFFFFFFF);
        for (Item i : ITEMS) ModuleRegistry.register(i);
        Trails t = new Trails();
        MinecraftForge.EVENT_BUS.register(t);
        FMLCommonHandler.instance().bus().register(t);
    }

    public static int count() {
        return ITEMS.size();
    }

    // ---------- small shape helpers (inside Solid.begin(GL_TRIANGLES)) ----------

    private static void quad(float ax, float ay, float az, float bx, float by, float bz, float cx, float cy, float cz, float dx, float dy, float dz, int c1, int c2) {
        Solid.triangle(ax, ay, az, bx, by, bz, cx, cy, cz, c1, c1, c2);
        Solid.triangle(ax, ay, az, cx, cy, cz, dx, dy, dz, c1, c2, c2);
    }

    /** A cylinder standing on (x, y, z). */
    private static void cyl(float x, float y, float z, float r, float h, int seg, int side, int top) {
        for (int i = 0; i < seg; i++) {
            double a0 = i * Math.PI * 2 / seg, a1 = (i + 1) * Math.PI * 2 / seg;
            float c0 = (float) Math.cos(a0), s0 = (float) Math.sin(a0), c1 = (float) Math.cos(a1), s1 = (float) Math.sin(a1);
            int sh = Solid.shade(side, 0.7f + 0.3f * (float) Math.max(0, Math.cos((a0 + a1) / 2 - 0.8)));
            quad(x + c0 * r, y, z + s0 * r, x + c1 * r, y, z + s1 * r, x + c1 * r, y + h, z + s1 * r, x + c0 * r, y + h, z + s0 * r, sh, sh);
            Solid.triangle(x, y + h, z, x + c0 * r, y + h, z + s0 * r, x + c1 * r, y + h, z + s1 * r, top, top, top);
        }
    }

    /** A cone from a base circle on (x, y, z) to a tip at (x + tx, y + h, z + tz). */
    private static void cone(float x, float y, float z, float r, float h, float tx, float tz, int seg, int base, int tip) {
        for (int i = 0; i < seg; i++) {
            double a0 = i * Math.PI * 2 / seg, a1 = (i + 1) * Math.PI * 2 / seg;
            int sh = Solid.shade(base, 0.7f + 0.3f * (float) Math.max(0, Math.cos((a0 + a1) / 2 - 0.8)));
            Solid.triangle(x + (float) Math.cos(a0) * r, y, z + (float) Math.sin(a0) * r, x + (float) Math.cos(a1) * r, y, z + (float) Math.sin(a1) * r, x + tx, y + h, z + tz, sh, sh, tip);
        }
    }

    private static void ring(float y, float r1, float r2, int seg, int c) {
        Solid.annulus(0, y, 0, 1, 0, 0, 0, 0, 1, r1, r2, 0, Math.PI * 2, c, c, seg);
    }

    private static int hue(float h, float a) {
        int rgb = java.awt.Color.HSBtoRGB(((h % 1) + 1) % 1, 0.75f, 1f) & 0xFFFFFF;
        return Colors.withAlpha(rgb, a);
    }

    private static float hash(int a, int b) {
        return Sprites.hash(a, b);
    }

    // ---------- drawing (called from CosmeticRenderer) ----------

    /** Solid parts: hats, capes, pets' bodies, back items. */
    static void solid(Pose p, float level, float drop) {
        Solid.begin(GL11.GL_TRIANGLES);
        for (Item it : ITEMS) {
            if (!it.enabled()) continue;
            if (it.kind.equals("hat")) hat(p, it, level, drop);
            else if (it.kind.equals("cape")) cape(p, it, level, drop);
            else if (it.kind.equals("pet")) petSolid(p, it, level, drop);
            else if (it.kind.equals("back")) back(p, it, level, drop);
        }
        Solid.end();
    }

    /** Light: auras, pets' glow, sparkles, the preview's trail. */
    static void glow(Pose p, float level, float drop) {
        for (Item it : ITEMS) {
            if (!it.enabled()) continue;
            if (it.kind.equals("aura")) aura(p, it, level, drop);
            else if (it.kind.equals("pet")) petGlow(p, it, level, drop);
            else if (it.kind.equals("cape")) capeGlow(p, it, level, drop);
            else if (it.kind.equals("hat")) hatGlow(p, it, level, drop);
            else if (it.kind.equals("back") && it.style.equals("jetpack")) jetFlame(p, it, level, drop);
            else if (it.kind.equals("trail") && p.scene != Scene.WORLD) previewTrail(p, it, level);
        }
    }

    /** Body space (after the body turn) to the cosmetic origin's space, for sprites. */
    private static float bodyX(Pose p, float x, float z) {
        double a = Math.toRadians(-p.bodyYaw);
        return (float) (x * Math.cos(a) + z * Math.sin(a));
    }

    private static float bodyZ(Pose p, float x, float z) {
        double a = Math.toRadians(-p.bodyYaw);
        return (float) (-x * Math.sin(a) + z * Math.cos(a));
    }

    // ---------- hats ----------

    private static void headSpace(Pose p, float drop) {
        GlStateManager.rotate(-p.bodyYaw, 0, 1, 0);
        GlStateManager.translate(0, 1.406f - drop, 0);
        GlStateManager.rotate(-p.headYaw, 0, 1, 0);
        GlStateManager.rotate(p.headPitch, 1, 0, 0);
        GlStateManager.translate(0, 0.47f, 0);
    }

    private static void hat(Pose p, Item it, float level, float drop) {
        Solid.end();
        GlStateManager.pushMatrix();
        headSpace(p, drop);
        float s = it.size.f();
        GlStateManager.scale(s, s, s);
        Solid.begin(GL11.GL_TRIANGLES);
        int c = Colors.alpha(it.colour.argb(), level), a = Colors.alpha(it.accent.argb(), level);
        float t = p.time;
        String st = it.style;
        if (st.equals("crown")) {
            cyl(0, -0.02f, 0, 0.255f, 0.09f, 16, c, Colors.alpha(c, 0));
            for (int i = 0; i < 5; i++) {
                double ang = i * Math.PI * 2 / 5 + 0.3;
                Solid.spike((float) Math.cos(ang) * 0.24f, 0.07f, (float) Math.sin(ang) * 0.24f, 0, 0.13f, 0, 0.045f, c, Solid.shade(c, 1.2f));
            }
        } else if (st.equals("tophat")) {
            ring(0.0f, 0.18f, 0.34f, 24, c);
            cyl(0, 0, 0, 0.2f, 0.34f, 18, c, Solid.shade(c, 1.1f));
            cyl(0, 0.015f, 0, 0.205f, 0.06f, 18, a, a);
        } else if (st.equals("wizard")) {
            ring(0.0f, 0.12f, 0.42f, 24, c);
            cone(0, 0, 0, 0.27f, 0.42f, 0, -0.04f, 14, c, c);
            cone(0, 0.42f, -0.04f, 0.08f, 0.2f, 0, -0.16f, 8, c, Solid.shade(c, 1.2f));
        } else if (st.equals("catears") || st.equals("foxears")) {
            boolean fox = st.equals("foxears");
            for (int sd = -1; sd <= 1; sd += 2) {
                float x = sd * 0.16f;
                cone(x, -0.02f, 0.02f, fox ? 0.09f : 0.08f, fox ? 0.24f : 0.17f, sd * 0.03f, 0, 3, c, fox ? a : c);
                if (!fox) cone(x, -0.01f, 0.06f, 0.045f, 0.12f, sd * 0.02f, 0, 3, a, a);
            }
        } else if (st.equals("bunnyears")) {
            float flop = 6 + 10 * (float) Math.sin(t * 2.2f) * (p.fast ? 1.5f : 0.6f);
            for (int sd = -1; sd <= 1; sd += 2) {
                double r = Math.toRadians(sd * 8 + sd * flop * 0.3);
                float ux = (float) Math.sin(r) * 0.17f, uy = (float) Math.cos(r) * 0.17f;
                Solid.box(sd * 0.1f + ux, uy, 0, (float) Math.cos(r) * 0.055f, -(float) Math.sin(r) * 0.055f, 0, ux, uy, 0, 0, 0, 0.022f, c);
                Solid.box(sd * 0.1f + ux, uy, 0.024f, (float) Math.cos(r) * 0.03f, -(float) Math.sin(r) * 0.03f, 0, ux * 0.85f, uy * 0.85f, 0, 0, 0, 0.003f, a);
            }
        } else if (st.equals("antlers")) {
            for (int sd = -1; sd <= 1; sd += 2) {
                float bx = sd * 0.12f;
                Solid.beam(bx, -0.02f, 0, bx + sd * 0.16f, 0.3f, -0.03f, 0.05f, 0.03f, c, c);
                Solid.beam(bx + sd * 0.16f, 0.3f, -0.03f, bx + sd * 0.12f, 0.52f, -0.06f, 0.03f, 0.012f, c, a);
                Solid.beam(bx + sd * 0.08f, 0.14f, -0.01f, bx + sd * 0.02f, 0.3f, 0.06f, 0.025f, 0.01f, c, a);
                Solid.beam(bx + sd * 0.15f, 0.27f, -0.03f, bx + sd * 0.3f, 0.4f, -0.02f, 0.022f, 0.01f, c, a);
            }
        } else if (st.equals("party")) {
            for (int i = 0; i < 8; i++) {
                double a0 = i * Math.PI / 4, a1 = (i + 1) * Math.PI / 4;
                int col = i % 2 == 0 ? c : a;
                Solid.triangle((float) Math.cos(a0) * 0.16f, 0, (float) Math.sin(a0) * 0.16f, (float) Math.cos(a1) * 0.16f, 0, (float) Math.sin(a1) * 0.16f, 0.02f, 0.38f, 0, col, col, col);
            }
        } else if (st.equals("flowers")) {
            for (int i = 0; i < 9; i++) {
                double ang = i * Math.PI * 2 / 9;
                float fx = (float) Math.cos(ang) * 0.25f, fz = (float) Math.sin(ang) * 0.25f;
                int col = i % 3 == 0 ? a : c;
                for (int k = 0; k < 5; k++) {
                    double pa = k * Math.PI * 2 / 5 + i;
                    Solid.disc(fx + (float) Math.cos(pa) * 0.03f, -0.01f, fz + (float) Math.sin(pa) * 0.03f, 1, 0, 0, 0, 0, 1, 0.028f, col, col, 6);
                }
                Solid.disc(fx, 0.0f, fz, 1, 0, 0, 0, 0, 1, 0.018f, 0xFF000000 | 0xFFE03A, 0xFF000000 | 0xFFE03A, 6);
            }
        } else if (st.equals("cowboy")) {
            // Brim curls up at the sides.
            for (int i = 0; i < 28; i++) {
                double a0 = i * Math.PI * 2 / 28, a1 = (i + 1) * Math.PI * 2 / 28;
                float up0 = (float) Math.pow(Math.abs(Math.cos(a0)), 3) * 0.1f, up1 = (float) Math.pow(Math.abs(Math.cos(a1)), 3) * 0.1f;
                quad((float) Math.cos(a0) * 0.2f, 0, (float) Math.sin(a0) * 0.2f, (float) Math.cos(a1) * 0.2f, 0, (float) Math.sin(a1) * 0.2f,
                    (float) Math.cos(a1) * 0.46f, up1, (float) Math.sin(a1) * 0.4f, (float) Math.cos(a0) * 0.46f, up0, (float) Math.sin(a0) * 0.4f, c, Solid.shade(c, 0.85f));
            }
            cyl(0, 0, 0, 0.21f, 0.2f, 16, c, Solid.shade(c, 0.8f));
            cyl(0, 0.01f, 0, 0.215f, 0.04f, 16, a, a);
        }
        Solid.end();
        GlStateManager.popMatrix();
        Solid.begin(GL11.GL_TRIANGLES);
    }

    private static void hatGlow(Pose p, Item it, float level, float drop) {
        if (!(it.style.equals("crown") || it.style.equals("wizard") || it.style.equals("party"))) return;
        // Glow sits on the head top; head turn is small, so body space is close enough.
        float s = it.size.f();
        float y = 1.406f - drop + 0.47f;
        Sprites.beginGlow();
        if (it.style.equals("crown")) {
            for (int i = 0; i < 5; i++) {
                double ang = i * Math.PI * 2 / 5 + 0.3;
                float x = (float) Math.cos(ang) * 0.255f * s, z = (float) Math.sin(ang) * 0.255f * s;
                Sprites.add(bodyX(p, x, z), y + 0.03f * s, bodyZ(p, x, z), 0.05f * s, Colors.alpha(it.accent.argb(), (0.6f + 0.3f * (float) Math.sin(p.time * 3 + i)) * level));
            }
        } else if (it.style.equals("wizard")) {
            for (int i = 0; i < 6; i++) {
                float ph = (p.time * 0.3f + i / 6f) % 1;
                double ang = i * 1.7 + p.time * 0.6;
                float x = (float) Math.cos(ang) * 0.2f * s, z = (float) Math.sin(ang) * 0.2f * s;
                Sprites.add(bodyX(p, x, z), y + ph * 0.55f * s, bodyZ(p, x, z), 0.04f * s, Colors.alpha(it.accent.argb(), (float) Math.sin(ph * Math.PI) * level));
            }
        } else {
            Sprites.add(bodyX(p, 0.02f * s, 0), y + 0.4f * s, bodyZ(p, 0.02f * s, 0), 0.07f * s, Colors.alpha(it.accent.argb(), 0.9f * level));
        }
        Sprites.end();
    }

    // ---------- capes ----------

    private static int capeColour(Item it, float u, float v, float t) {
        int c = it.colour.argb(), a = it.accent.argb();
        String st = it.style;
        if (st.equals("midnight")) return Colors.mix(c, a, v);
        if (st.equals("ember")) return Colors.mix(c, a, Math.max(0, v * v + 0.1f * (float) Math.sin(t * 9 + u * 7)));
        if (st.equals("ocean")) return Colors.mix(c, a, 0.5f + 0.5f * (float) Math.sin(v * 12 - t * 2.5f + u * 2));
        if (st.equals("aurora")) {
            float k = 0.5f + 0.5f * (float) Math.sin(u * 4 + t * 0.9f + v * 3);
            return Colors.mix(Colors.mix(c, 0xFF1AC8C8, v), a, k);
        }
        if (st.equals("galaxy")) return Colors.mix(c, 0xFF05020F, v * 0.8f);
        if (st.equals("rainbow")) return hue(v * 0.8f + u * 0.15f + t * 0.08f, 1);
        if (st.equals("checker")) return ((int) Math.floor(u * 4) + (int) Math.floor(v * 6)) % 2 == 0 ? c : a;
        if (st.equals("royal")) return u < 0.09f || u > 0.91f || v > 0.9f ? a : Colors.mix(c, Solid.shade(c, 0.6f), v);
        if (st.equals("frost")) return Colors.mix(c, a, v);
        return c; // neon
    }

    private static float capeAngle(Pose p) {
        float swing = Math.min(1, p.limbSwingAmount * 1.4f) + (p.fast ? 0.35f : 0);
        return 6 + 48 * Math.min(1.2f, swing) + 12 * p.sneak();
    }

    /** Where a point of the cape is (u across 0..1, v down 0..1), in body space. */
    private static float[] capePoint(Pose p, Item it, float u, float v, float drop) {
        float L = 1.0f * it.size.f(), W = 0.58f;
        float ang = (float) Math.toRadians(capeAngle(p) * (0.55f + 0.45f * v) + Math.sin(p.time * 3.2f + v * 4 + u * 2.5f) * 4 * (0.4f + v));
        float y = 1.38f - drop - v * L * (float) Math.cos(ang);
        float z = -0.16f - v * L * (float) Math.sin(ang);
        return new float[] {(u - 0.5f) * W * (1 - 0.08f * v), y, z};
    }

    private static void cape(Pose p, Item it, float level, float drop) {
        Solid.end();
        GlStateManager.pushMatrix();
        GlStateManager.rotate(-p.bodyYaw, 0, 1, 0);
        Solid.begin(GL11.GL_TRIANGLES);
        int nu = 6, nv = 10;
        for (int j = 0; j < nv; j++) {
            for (int i = 0; i < nu; i++) {
                float u0 = i / (float) nu, u1 = (i + 1) / (float) nu, v0 = j / (float) nv, v1 = (j + 1) / (float) nv;
                float[] a = capePoint(p, it, u0, v0, drop), b = capePoint(p, it, u1, v0, drop), c = capePoint(p, it, u1, v1, drop), d = capePoint(p, it, u0, v1, drop);
                int ca = Colors.alpha(capeColour(it, u0, v0, p.time), level), cc = Colors.alpha(capeColour(it, u1, v1, p.time), level);
                quad(a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2], ca, cc);
            }
        }
        Solid.end();
        GlStateManager.popMatrix();
        Solid.begin(GL11.GL_TRIANGLES);
    }

    private static void capeGlow(Pose p, Item it, float level, float drop) {
        String st = it.style;
        if (!(st.equals("midnight") || st.equals("galaxy") || st.equals("frost") || st.equals("neon") || st.equals("ember"))) return;
        Sprites.beginGlow();
        if (st.equals("neon")) {
            int c = it.accent.argb();
            for (int k = 0; k <= 20; k++) {
                float v = k / 20f;
                for (float u : new float[] {0, 1}) {
                    float[] q = capePoint(p, it, u, v, drop);
                    Sprites.add(bodyX(p, q[0], q[2]), q[1], bodyZ(p, q[0], q[2]), 0.05f, Colors.alpha(c, 0.7f * level));
                }
                float[] q = capePoint(p, it, v, 1, drop);
                Sprites.add(bodyX(p, q[0], q[2]), q[1], bodyZ(p, q[0], q[2]), 0.05f, Colors.alpha(c, 0.7f * level));
            }
        } else {
            int n = st.equals("ember") ? 10 : 16;
            for (int i = 0; i < n; i++) {
                float u = 0.08f + 0.84f * hash(i, 3), v = 0.08f + 0.84f * hash(i, 4);
                float tw = 0.5f + 0.5f * (float) Math.sin(p.time * (2 + hash(i, 5) * 3) + i);
                float[] q = capePoint(p, it, u, v, drop);
                int col = st.equals("ember") ? 0xFFFFA03A : st.equals("frost") ? it.accent.argb() : 0xFFFFFFFF;
                float z = q[2] - 0.01f;
                Sprites.add(bodyX(p, q[0], z), q[1], bodyZ(p, q[0], z), 0.025f + 0.02f * tw, Colors.alpha(col, tw * 0.8f * level));
            }
        }
        Sprites.end();
    }

    // ---------- auras ----------

    private static void aura(Pose p, Item it, float level, float drop) {
        String st = it.style;
        int c = it.colour.argb(), a = it.accent.argb();
        float s = it.size.f(), t = p.time;
        if (st.equals("electric")) {
            Sprites.beginGlow();
            for (int k = 0; k < 4; k++) {
                if (hash((int) (t * 8) + k * 31, 7) > 0.55f) continue;
                double ang = hash((int) (t * 8) + k * 13, 2) * Math.PI * 2;
                float y0 = 0.2f + hash((int) (t * 8) + k, 9) * 1.4f;
                float px = 0, py = y0, pz = 0;
                float r = 0.45f * s;
                for (int seg = 0; seg < 5; seg++) {
                    double aa = ang + seg * 0.25;
                    float nx = (float) Math.cos(aa) * r, nz = (float) Math.sin(aa) * r, ny = py + (hash(seg + k * 7, (int) (t * 8)) - 0.5f) * 0.25f;
                    if (seg > 0) Sprites.line(px, py, pz, nx, ny, nz, 0.03f, Colors.alpha(seg % 2 == 0 ? c : a, 0.9f * level));
                    px = nx;
                    py = ny;
                    pz = nz;
                }
            }
            Sprites.end();
            return;
        }
        Sprites.beginGlow();
        int n = General.detail() == 0 ? 14 : 26;
        for (int i = 0; i < n; i++) {
            float speed = st.equals("frost") || st.equals("blood") ? -0.35f : st.equals("void") ? 0 : 0.55f;
            float ph = ((t * Math.abs(speed == 0 ? 0.4f : speed) + hash(i, 1)) % 1);
            float life = speed < 0 ? 1 - ph : ph;
            double ang = hash(i, 2) * Math.PI * 2 + (st.equals("rainbow") ? t * 1.5 : t * 0.3);
            float r = (0.38f + 0.18f * hash(i, 3)) * s;
            if (st.equals("void")) r *= 1.6f - ph;
            if (st.equals("toxic")) r += 0.05f * (float) Math.sin(t * 5 + i);
            float x = (float) Math.cos(ang) * r, z = (float) Math.sin(ang) * r;
            float y = st.equals("void") ? 0.3f + hash(i, 4) * 1.5f : 0.05f + life * 1.9f;
            int col = st.equals("rainbow") ? hue(hash(i, 6) + t * 0.2f, 1) : Colors.mix(c, a, hash(i, 5));
            float alpha = (float) Math.sin(ph * Math.PI) * level;
            float size = (st.equals("flame") ? 0.12f * (1 - life * 0.5f) : st.equals("toxic") ? 0.07f : 0.06f) * s;
            Sprites.add(x, y, z, size, Colors.alpha(col, alpha * 0.8f));
        }
        if (st.equals("holy")) {
            for (int i = 0; i < 24; i++) {
                double ang = i * Math.PI / 12 + t * 0.5;
                Sprites.add((float) Math.cos(ang) * 0.55f * s, 0.03f, (float) Math.sin(ang) * 0.55f * s, 0.07f, Colors.alpha(c, 0.45f * level));
            }
        }
        Sprites.end();
    }

    // ---------- pets ----------

    private static float[] petPos(Pose p, float drop) {
        float t = p.time;
        float x = 0.72f + 0.05f * (float) Math.sin(t * 0.7f), y = 1.95f - drop + 0.08f * (float) Math.sin(t * 1.7f), z = -0.2f + 0.08f * (float) Math.cos(t * 0.6f);
        return new float[] {bodyX(p, x, z), y, bodyZ(p, x, z)};
    }

    private static void petSolid(Pose p, Item it, float level, float drop) {
        String st = it.style;
        if (!(st.equals("cube") || st.equals("crystal") || st.equals("planet") || st.equals("lantern"))) return;
        float[] q = petPos(p, drop);
        float s = it.size.f(), t = p.time;
        int c = Colors.alpha(it.colour.argb(), level), a = Colors.alpha(it.accent.argb(), level);
        Solid.end();
        GlStateManager.pushMatrix();
        GlStateManager.translate(q[0], q[1], q[2]);
        GlStateManager.rotate(t * 50, 0, 1, 0);
        GlStateManager.scale(s, s, s);
        Solid.begin(GL11.GL_TRIANGLES);
        if (st.equals("cube")) {
            GlStateManager.rotate(25, 1, 0, 1);
            Solid.aabb(-0.09f, -0.09f, -0.09f, 0.09f, 0.09f, 0.09f, c);
        } else if (st.equals("crystal")) {
            Solid.spike(0, 0, 0, 0, 0.18f, 0, 0.07f, c, Solid.shade(c, 1.3f));
            Solid.spike(0, 0, 0, 0, -0.12f, 0, 0.07f, c, Solid.shade(c, 0.7f));
        } else if (st.equals("planet")) {
            Solid.annulus(0, 0, 0, 0.94f, 0.34f, 0, 0, 0, 1, 0.13f, 0.19f, 0, Math.PI * 2, a, Colors.alpha(a, 0.3f), 28);
        } else {
            Solid.aabb(-0.07f, -0.09f, -0.07f, 0.07f, -0.07f, 0.07f, c);
            Solid.aabb(-0.07f, 0.07f, -0.07f, 0.07f, 0.09f, 0.07f, c);
            for (int sx = -1; sx <= 1; sx += 2) for (int sz = -1; sz <= 1; sz += 2) Solid.aabb(sx * 0.07f - 0.01f, -0.07f, sz * 0.07f - 0.01f, sx * 0.07f + 0.01f, 0.07f, sz * 0.07f + 0.01f, c);
        }
        Solid.end();
        GlStateManager.popMatrix();
        Solid.begin(GL11.GL_TRIANGLES);
    }

    private static void petGlow(Pose p, Item it, float level, float drop) {
        float[] q = petPos(p, drop);
        float s = it.size.f(), t = p.time;
        int c = it.colour.argb(), a = it.accent.argb();
        String st = it.style;
        Sprites.beginGlow();
        if (st.equals("fireflies")) {
            for (int i = 0; i < 7; i++) {
                float x = 0.55f * (float) Math.sin(t * (0.5f + hash(i, 1)) + i * 2), y = 1.7f - drop + 0.35f * (float) Math.sin(t * (0.7f + hash(i, 2)) + i), z = 0.55f * (float) Math.cos(t * (0.4f + hash(i, 3)) + i * 3);
                float blink = 0.4f + 0.6f * Math.max(0, (float) Math.sin(t * 3 + i * 1.7f));
                Sprites.add(x, y, z, 0.05f * s, Colors.alpha(Colors.mix(c, a, hash(i, 4)), blink * level));
            }
        } else if (st.equals("wisp")) {
            for (int i = 0; i < 6; i++) {
                float k = i / 6f;
                Sprites.add(q[0] + 0.04f * (float) Math.sin(t * 4 + i), q[1] - k * 0.12f, q[2] + 0.04f * (float) Math.cos(t * 3 + i), (0.12f - k * 0.08f) * s, Colors.alpha(Colors.mix(a, c, k), (0.7f - k * 0.4f) * level));
            }
        } else {
            int col = st.equals("lantern") ? a : c;
            float size = st.equals("planet") ? 0.2f : st.equals("lantern") ? 0.16f : 0.14f;
            Sprites.add(q[0], q[1], q[2], size * s, Colors.alpha(col, (st.equals("planet") ? 0.95f : 0.55f) * level));
            if (st.equals("planet")) Sprites.add(q[0], q[1], q[2], 0.09f * s, Colors.alpha(0xFFFFFFFF, 0.5f * level));
        }
        Sprites.end();
    }

    // ---------- back ----------

    private static void back(Pose p, Item it, float level, float drop) {
        Solid.end();
        GlStateManager.pushMatrix();
        GlStateManager.rotate(-p.bodyYaw, 0, 1, 0);
        float s = it.size.f(), t = p.time;
        GlStateManager.translate(0, 1.05f - drop, -0.17f);
        GlStateManager.scale(s, s, s);
        Solid.begin(GL11.GL_TRIANGLES);
        int c = Colors.alpha(it.colour.argb(), level), a = Colors.alpha(it.accent.argb(), level);
        String st = it.style;
        if (st.equals("butterfly")) {
            float flap = (float) Math.toRadians(25 + 30 * Math.sin(t * (p.fast ? 9 : 4)));
            for (int sd = -1; sd <= 1; sd += 2) {
                float cx = (float) Math.cos(flap), sz = -(float) Math.sin(flap);
                // Upper and lower wings: a fan of triangles, dark edge to bright centre.
                Solid.triangle(0, 0.1f, 0, sd * 0.55f * cx, 0.45f, 0.55f * sz, sd * 0.42f * cx, 0.05f, 0.42f * sz, a, c, c);
                Solid.triangle(0, 0.1f, 0, sd * 0.42f * cx, 0.05f, 0.42f * sz, sd * 0.1f * cx, 0.2f, 0.1f * sz, a, c, a);
                Solid.triangle(0, 0.0f, 0, sd * 0.38f * cx, -0.12f, 0.38f * sz, sd * 0.25f * cx, -0.34f, 0.25f * sz, a, c, c);
            }
        } else if (st.equals("jetpack")) {
            for (int sd = -1; sd <= 1; sd += 2) {
                cyl(sd * 0.1f, -0.25f, -0.08f, 0.075f, 0.5f, 12, c, Solid.shade(c, 1.15f));
                cone(sd * 0.1f, -0.25f, -0.08f, 0.075f, -0.07f, 0, 0, 12, Solid.shade(c, 0.6f), Solid.shade(c, 0.4f));
            }
            Solid.aabb(-0.12f, -0.05f, -0.06f, 0.12f, 0.12f, 0.0f, Solid.shade(c, 0.7f));
        } else if (st.equals("backpack")) {
            Solid.aabb(-0.2f, -0.3f, -0.14f, 0.2f, 0.22f, 0.0f, c);
            Solid.aabb(-0.205f, 0.06f, -0.145f, 0.205f, 0.225f, -0.06f, a);
            Solid.aabb(-0.12f, -0.22f, -0.16f, 0.12f, -0.04f, -0.13f, Solid.shade(c, 0.8f));
        } else {
            // Tail: segments curving out behind and down, swaying side to side.
            float px = 0, py = -0.28f, pz = 0;
            float sway = (float) Math.sin(t * (p.fast ? 6 : 2.5f)) * 0.12f;
            int segs = 7;
            for (int i = 1; i <= segs; i++) {
                float k = i / (float) segs;
                float nx = sway * k * k, ny = -0.28f - 0.12f * k + 0.18f * k * k, nz = -0.42f * k;
                float w0 = 0.15f * (float) Math.sin((k - 1f / segs) * Math.PI * 0.9f + 0.4f), w1 = 0.15f * (float) Math.sin(k * Math.PI * 0.9f + 0.4f);
                Solid.beam(px, py, pz, nx, ny, nz, Math.max(0.03f, w0), Math.max(0.02f, w1), k > 0.75f ? a : c, k > 0.7f ? a : c);
                px = nx;
                py = ny;
                pz = nz;
            }
        }
        Solid.end();
        GlStateManager.popMatrix();
        Solid.begin(GL11.GL_TRIANGLES);
    }

    private static void jetFlame(Pose p, Item it, float level, float drop) {
        if (!p.fast && p.scene == Scene.WORLD) return;
        float s = it.size.f(), t = p.time;
        Sprites.beginGlow();
        for (int sd = -1; sd <= 1; sd += 2) {
            for (int i = 0; i < 6; i++) {
                float ph = (t * 3 + i / 6f) % 1;
                float x = sd * 0.1f * s, y = 1.05f - drop - (0.32f + ph * 0.35f) * s, z = -0.25f * s - 0.17f;
                Sprites.add(bodyX(p, x, z), y, bodyZ(p, x, z), (0.08f - ph * 0.05f) * s, Colors.alpha(Colors.mix(0xFFFFFFFF, it.accent.argb(), ph), (1 - ph) * level));
            }
        }
        Sprites.end();
    }

    // ---------- trails ----------

    private static int trailColour(Item it, int i) {
        return it.style.equals("ribbon") ? Colors.mix(it.colour.argb(), it.accent.argb(), (i % 10) / 10f) : Colors.mix(it.colour.argb(), it.accent.argb(), hash(i, 11));
    }

    /** In the preview there is no path to follow: show the trail streaming back from the feet. */
    private static void previewTrail(Pose p, Item it, float level) {
        Sprites.beginGlow();
        for (int i = 0; i < 14; i++) {
            float ph = (p.time * 0.6f + i / 14f) % 1;
            float x = (hash(i, 1) - 0.5f) * 0.5f, z = -0.15f - ph * 1.2f;
            float y = trailY(it.style, ph, i);
            Sprites.add(bodyX(p, x, z), y, bodyZ(p, x, z), 0.06f * it.size.f() * (1 - ph * 0.5f), Colors.alpha(trailColour(it, i), (1 - ph) * level));
        }
        Sprites.end();
    }

    private static float trailY(String st, float age, int i) {
        if (st.equals("bubbles") || st.equals("hearts")) return 0.1f + age * 0.9f;
        if (st.equals("petals") || st.equals("snow")) return 0.6f - age * 0.55f + 0.05f * (float) Math.sin(age * 9 + i);
        if (st.equals("flame")) return 0.08f + age * 0.35f;
        if (st.equals("ribbon")) return 0.12f;
        return 0.1f + 0.2f * hash(i, 12);
    }

    /** Records where you walked and draws the trails in the world. */
    public static final class Trails {
        private static final int MAX = 60;
        private final double[] xs = new double[MAX], ys = new double[MAX], zs = new double[MAX];
        private final float[] at = new float[MAX];
        private int head;
        private double lx, ly, lz;

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            EntityPlayerSP pl = Minecraft.getMinecraft().thePlayer;
            if (e.phase != TickEvent.Phase.END || pl == null) return;
            if (Math.abs(pl.posX - lx) + Math.abs(pl.posY - ly) + Math.abs(pl.posZ - lz) < 0.12) return;
            lx = pl.posX;
            ly = pl.posY;
            lz = pl.posZ;
            xs[head] = pl.posX;
            ys[head] = pl.posY;
            zs[head] = pl.posZ;
            at[head] = dev.vesper.client.render.Ease.seconds();
            head = (head + 1) % MAX;
        }

        @SubscribeEvent
        public void onWorld(RenderWorldLastEvent e) {
            if (!General.fx()) return;
            boolean any = false;
            for (Item it : ITEMS) if (it.kind.equals("trail") && it.enabled()) any = true;
            if (!any) return;
            RenderManager rm = Minecraft.getMinecraft().getRenderManager();
            float now = dev.vesper.client.render.Ease.seconds();
            Gl.beginGlow();
            View.capture();
            FxDraw.sprites(Textures.glow());
            for (Item it : ITEMS) {
                if (!it.kind.equals("trail") || !it.enabled()) continue;
                float life = 1.6f * it.size.f();
                for (int k = 0; k < MAX; k++) {
                    float age = (now - at[k]) / life;
                    if (at[k] == 0 || age >= 1 || age < 0) continue;
                    double x = xs[k] - rm.viewerPosX + (hash(k, 1) - 0.5) * 0.4, z = zs[k] - rm.viewerPosZ + (hash(k, 2) - 0.5) * 0.4;
                    double y = ys[k] - rm.viewerPosY + trailY(it.style, age, k);
                    FxDraw.sprite(x, y, z, 0.13 * it.size.f() * (1 - age * 0.5), Colors.alpha(trailColour(it, k), (1 - age) * 0.85f));
                }
            }
            FxDraw.flush();
            Gl.endGlow();
        }
    }
}
