package dev.vesper.client.visual;

import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.fx.FxDraw;
import dev.vesper.client.fx.FxText;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.qol.Utilities;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import dev.vesper.client.render.Textures;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraft.entity.Entity;
import net.minecraft.entity.item.EntityItem;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.client.event.RenderWorldLastEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import org.lwjgl.opengl.GL11;

/** Looks for the world and the screen: filters, ambience, markers, a crosshair and kill moments. */
public final class WorldVisuals {
    private WorldVisuals() {
    }

    public static void init() {
        Module[] ms = {new ScreenFilter(), new Vignette(), new Ambient("fireflies", "Fireflies", "Fireflies drifting around you at night.", 0xFFD8FF6A),
            new Ambient("leaves", "Falling leaves", "Leaves drifting down around you.", 0xFFE89A3A), new Ambient("snow", "Snowfall", "Soft snow falling around you, any biome.", 0xFFFFFFFF),
            new Ambient("embers", "Embers", "Glowing embers floating up around you.", 0xFFFF6A1A), new ItemGlow(), new KillStreak(), new DeathMarker(), new ChunkBorders(),
            new Crosshair(), new CinemaBars(), new KillFlash()};
        for (Module m : ms) {
            ModuleRegistry.register(m);
            MinecraftForge.EVENT_BUS.register(m);
            FMLCommonHandler.instance().bus().register(m);
        }
    }

    static Minecraft mc() {
        return Minecraft.getMinecraft();
    }

    static float hash(int a, int b) {
        return dev.vesper.client.cosmetic.Sprites.hash(a, b);
    }

    /** A colour grade over the game: warm, cool, sepia, dusk and more. */
    public static final class ScreenFilter extends Module {
        static final String[] IDS = {"warm", "cool", "sepia", "dusk", "mint", "rose", "night", "faded"};
        static final int[] TINT = {0xFFFF9A3A, 0xFF3A8AFF, 0xFFB08A50, 0xFF8A3AFF, 0xFF3AFFB0, 0xFFFF6AA8, 0xFF3AFF5A, 0xFFB8B8C0};
        public final ChoiceSetting look = choice("look", "Look", "warm", IDS, new String[] {"Warm", "Cool", "Sepia", "Dusk", "Mint", "Rose", "Night vision", "Faded"});
        public final NumberSetting strength = number("strength", "Strength", 0.18, 0.03, 0.5, 0.01, "");
        public final BoolSetting menus = bool("menus", "Keep it in menus too", false);

        public ScreenFilter() {
            super("screenfilter", "Screen filter", "A colour grade over the game: warm, cool, sepia, dusk, mint, rose, night vision or faded.", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Pre e) {
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.ALL || (mc().currentScreen != null && !menus.on())) return;
            int i = java.util.Arrays.asList(IDS).indexOf(look.get());
            Gl.begin2D();
            Draw.rect(0, 0, e.resolution.getScaledWidth(), e.resolution.getScaledHeight(), Colors.alpha(TINT[Math.max(0, i)], strength.f()));
            Gl.end2D();
        }
    }

    /** Darker screen corners that draw the eye to the middle. */
    public static final class Vignette extends Module {
        public final NumberSetting strength = number("strength", "Strength", 0.45, 0.1, 0.9, 0.05, "");
        public final ColorSetting colour = color("colour", "Colour", 0xFF000000);

        public Vignette() {
            super("vignette", "Vignette", "Darker screen edges that draw your eye to the middle.", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Pre e) {
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.ALL) return;
            int w = e.resolution.getScaledWidth(), h = e.resolution.getScaledHeight();
            int c = colour.argb();
            float a = strength.f(), edge = Math.min(w, h) * 0.3f;
            Gl.begin2D();
            Draw.gradientV(0, 0, w, edge, Colors.alpha(c, a), Colors.alpha(c, 0));
            Draw.gradientV(0, h - edge, w, h, Colors.alpha(c, 0), Colors.alpha(c, a));
            Draw.gradientH(0, 0, edge, h, Colors.alpha(c, a), Colors.alpha(c, 0));
            Draw.gradientH(w - edge, 0, w, h, Colors.alpha(c, 0), Colors.alpha(c, a));
            Gl.end2D();
        }
    }

    /** Little lights or flakes filling the air around you (only on your screen). */
    public static final class Ambient extends Module {
        final String kind;
        public final ColorSetting colour;
        public final NumberSetting amount = number("amount", "Amount", 40, 10, 120, 5, "");
        public final NumberSetting radius = number("radius", "Area", 10, 4, 24, 1, " blocks");

        Ambient(String kind, String name, String desc, int c) {
            super("amb_" + kind, name, desc, Category.VISUALS, false, true);
            this.kind = kind;
            colour = color("colour", "Colour", c);
        }

        @SubscribeEvent
        public void onWorld(RenderWorldLastEvent e) {
            Entity v = mc().getRenderViewEntity();
            if (!enabled() || v == null || !General.fx()) return;
            if (kind.equals("fireflies") && mc().theWorld.isDaytime()) return;
            RenderManager rm = mc().getRenderManager();
            float t = Ease.seconds(), r = radius.f();
            int n = Math.round(amount.f() * (General.detail() == 0 ? 0.5f : 1));
            Gl.beginGlow();
            View.capture();
            FxDraw.sprites(Textures.glow());
            for (int i = 0; i < n; i++) {
                // Each one lives in a cell that moves with you, so the air is always full.
                float fall = kind.equals("snow") ? 0.8f : kind.equals("leaves") ? 0.6f : kind.equals("embers") ? -0.7f : 0;
                double cx = Math.floor(rm.viewerPosX / (r * 2)) * r * 2 + hash(i, 1) * r * 2, cz = Math.floor(rm.viewerPosZ / (r * 2)) * r * 2 + hash(i, 2) * r * 2;
                double x = wrap(cx + Math.sin(t * 0.4 + i) * 0.6 - rm.viewerPosX, r), z = wrap(cz + Math.cos(t * 0.35 + i * 1.3) * 0.6 - rm.viewerPosZ, r);
                double y = wrap(hash(i, 3) * r * 2 - fall * t * (0.6 + hash(i, 4)) - (rm.viewerPosY % (r * 2)), r) + 1;
                float a = kind.equals("fireflies") ? Math.max(0, (float) Math.sin(t * 2 + i * 1.7f)) : 0.8f;
                float edge = 1 - (float) Math.min(1, Math.sqrt(x * x + z * z) / r);
                float size = kind.equals("snow") ? 0.07f : kind.equals("leaves") ? 0.09f : kind.equals("embers") ? 0.06f : 0.08f;
                int c = kind.equals("leaves") ? Colors.mix(colour.argb(), 0xFFB04A1A, hash(i, 5)) : colour.argb();
                FxDraw.sprite(x, y, z, size, Colors.alpha(c, a * edge));
            }
            FxDraw.flush();
            Gl.endGlow();
        }

        private static double wrap(double v, double r) {
            double m = ((v + r) % (r * 2) + r * 2) % (r * 2);
            return m - r;
        }
    }

    /** A soft light under dropped items so you spot loot. */
    public static final class ItemGlow extends Module {
        public final ColorSetting colour = color("colour", "Colour", 0xFFFFE07A);

        public ItemGlow() {
            super("itemglow", "Item glow", "A soft light and a slow ring under dropped items so loot is easy to spot.", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onWorld(RenderWorldLastEvent e) {
            if (!enabled() || mc().theWorld == null || !General.fx()) return;
            RenderManager rm = mc().getRenderManager();
            float t = Ease.seconds();
            Gl.beginGlow();
            View.capture();
            FxDraw.geo();
            int c = colour.argb();
            for (Entity o : mc().theWorld.loadedEntityList) {
                if (!(o instanceof EntityItem) || o.getDistanceSqToEntity(mc().thePlayer) > 32 * 32) continue;
                double x = o.posX - rm.viewerPosX, y = o.posY - rm.viewerPosY + 0.02, z = o.posZ - rm.viewerPosZ;
                FxDraw.glowRingH(x, y, z, 0.28 + 0.04 * Math.sin(t * 3 + o.getEntityId()), 0.05, Colors.alpha(c, 0.7f), 24);
            }
            FxDraw.flush();
            FxDraw.sprites(Textures.glow());
            for (Entity o : mc().theWorld.loadedEntityList) {
                if (!(o instanceof EntityItem) || o.getDistanceSqToEntity(mc().thePlayer) > 32 * 32) continue;
                FxDraw.sprite(o.posX - rm.viewerPosX, o.posY - rm.viewerPosY + 0.25, o.posZ - rm.viewerPosZ, 0.45, Colors.alpha(c, 0.25f));
            }
            FxDraw.flush();
            Gl.endGlow();
        }
    }

    /** "Double kill", "Triple kill"… slides in when kills come close together. */
    public static final class KillStreak extends Module {
        static final String[] NAMES = {"", "Kill", "Double kill", "Triple kill", "Quad kill", "Rampage", "Unstoppable", "Godlike"};
        public final NumberSetting window = number("window", "Time between kills", 6, 2, 15, 1, "s");
        public final BoolSetting single = bool("single", "Show single kills too", false);
        private int streak;
        private float lastKill = -100, shownAt = -100;
        private String text = "";

        public KillStreak() {
            super("killstreak", "Kill streak banner", "\"Double kill\", \"Triple kill\" and more slide in when kills come close together.", Category.EFFECTS, false, true);
            CombatBus.listen(new CombatBus.Listener() {
                public void on(CombatVisualEvent e) {
                    if (e.type != CombatVisualEvent.Type.PLAYER_KILL_CONFIRMED) return;
                    float now = Ease.seconds();
                    streak = now - lastKill < window.f() ? streak + 1 : 1;
                    lastKill = now;
                    if (streak > 1 || single.on()) {
                        text = NAMES[Math.min(streak, NAMES.length - 1)];
                        shownAt = now;
                    }
                }
            });
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Post e) {
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.ALL) return;
            float t = Ease.seconds() - shownAt;
            if (t > 2.2f) return;
            float in = Ease.outBack(Ease.clamp01(t / 0.3f)), out = 1 - Ease.span(t, 1.7f, 2.2f);
            int w = e.resolution.getScaledWidth();
            float size = 22, cx = w / 2f, y = e.resolution.getScaledHeight() * 0.22f;
            int acc = 0xFF000000 | General.accent();
            Gl.begin2D();
            GlStateManager.pushMatrix();
            GlStateManager.translate(cx, y, 0);
            float s = 0.6f + 0.4f * in;
            GlStateManager.scale(s, s, 1);
            float tw = GlyphFont.width(text, size, true);
            Draw.glow(0, size * 0.6f, tw * 0.7f, Colors.alpha(acc, 0.35f * out));
            GlyphFont.drawCentered(text, 1.5f, 1.5f, size, true, Colors.alpha(0xFF000000, 0.6f * out));
            GlyphFont.drawCentered(text, 0, 0, size, true, Colors.alpha(0xFFFFFFFF, out));
            Draw.rect(-tw / 2 * in, size * 1.35f, tw / 2 * in, size * 1.35f + 2, Colors.alpha(acc, out));
            GlStateManager.popMatrix();
            Gl.end2D();
        }
    }

    /** A beam of light where you last died, with the distance to it. */
    public static final class DeathMarker extends Module {
        public final ColorSetting colour = color("colour", "Colour", 0xFFFF3A5A);

        public DeathMarker() {
            super("deathmarker", "Death marker", "A beam of light where you last died, with the distance, until you get back there.", Category.VISUALS, true, true);
        }

        @SubscribeEvent
        public void onWorld(RenderWorldLastEvent e) {
            net.minecraft.util.BlockPos d = Utilities.DeathCoords.last;
            if (!enabled() || d == null || mc().thePlayer == null || mc().thePlayer.dimension != Utilities.DeathCoords.lastDim || mc().thePlayer.getHealth() <= 0) return;
            double dist = mc().thePlayer.getDistance(d.getX() + 0.5, d.getY(), d.getZ() + 0.5);
            if (dist < 3) {
                Utilities.DeathCoords.last = null;
                return;
            }
            RenderManager rm = mc().getRenderManager();
            double x = d.getX() + 0.5 - rm.viewerPosX, y = d.getY() - rm.viewerPosY, z = d.getZ() + 0.5 - rm.viewerPosZ;
            int c = colour.argb();
            Gl.beginGlow();
            GlStateManager.disableDepth();
            View.capture();
            FxDraw.geo();
            FxDraw.glowLine(x, y, z, x, y + 40, z, 0.3, Colors.alpha(c, 0.55f));
            FxDraw.glowRingH(x, y + 0.05, z, 0.6 + 0.1 * Math.sin(Ease.seconds() * 3), 0.08, Colors.alpha(c, 0.8f), 32);
            FxDraw.flush();
            double k = Math.min(dist, 12) / dist;
            GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
            FxText.text("Died here · " + Math.round(dist) + " m", x * k, (y + 2.4) * k, z * k, 0.35 * k * (dist > 12 ? dist / 12 : 1) * Math.min(1, 12 / Math.max(1, dist)) + 0.12, 0, 0xFFFFFFFF, 0xC0000000);
            GlStateManager.enableDepth();
            Gl.endGlow();
        }
    }

    /** The edges of the chunk you are in (and its neighbours), drawn on the ground and up. */
    public static final class ChunkBorders extends Module {
        public final ColorSetting colour = color("colour", "Colour", 0xFFFFD23A);

        public ChunkBorders() {
            super("chunkborders", "Chunk borders", "Lines on the edges of the chunk you stand in, so builds and farms line up.", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onWorld(RenderWorldLastEvent e) {
            if (!enabled() || mc().thePlayer == null) return;
            RenderManager rm = mc().getRenderManager();
            int cx = ((int) Math.floor(mc().thePlayer.posX)) >> 4 << 4, cz = ((int) Math.floor(mc().thePlayer.posZ)) >> 4 << 4;
            double y0 = Math.floor(mc().thePlayer.posY) - 4 - rm.viewerPosY, y1 = y0 + 12;
            int c = colour.argb();
            Gl.beginGlow();
            View.capture();
            FxDraw.geo();
            for (int i = 0; i <= 16; i += 2) {
                for (int side = 0; side < 4; side++) {
                    double x = cx + (side == 0 ? 0 : side == 1 ? 16 : i) - rm.viewerPosX, z = cz + (side == 2 ? 0 : side == 3 ? 16 : i) - rm.viewerPosZ;
                    boolean corner = (i == 0 || i == 16);
                    FxDraw.line(x, y0, z, x, y1, z, corner ? 0.05 : 0.02, Colors.alpha(c, corner ? 0.9f : 0.35f));
                }
            }
            for (double y = y0; y <= y1; y += 2) {
                double ax = cx - rm.viewerPosX, az = cz - rm.viewerPosZ;
                FxDraw.line(ax, y, az, ax + 16, y, az, 0.02, Colors.alpha(c, 0.35f));
                FxDraw.line(ax, y, az + 16, ax + 16, y, az + 16, 0.02, Colors.alpha(c, 0.35f));
                FxDraw.line(ax, y, az, ax, y, az + 16, 0.02, Colors.alpha(c, 0.35f));
                FxDraw.line(ax + 16, y, az, ax + 16, y, az + 16, 0.02, Colors.alpha(c, 0.35f));
            }
            FxDraw.flush();
            Gl.endGlow();
        }
    }

    /** Your own crosshair: shape, size, gap, colour and outline, in place of Minecraft's. */
    public static final class Crosshair extends Module {
        public final ChoiceSetting shape = choice("shape", "Shape", "cross", new String[] {"cross", "dot", "circle", "crossdot", "chevron", "square"}, new String[] {"Cross", "Dot", "Circle", "Cross and dot", "Chevron", "Square"});
        public final ColorSetting colour = color("colour", "Colour", 0xFFFFFFFF);
        public final BoolSetting world = bool("world", "Use my world colour", false);
        public final NumberSetting size = number("size", "Size", 5, 2, 12, 0.5, "");
        public final NumberSetting gap = number("gap", "Gap", 2, 0, 6, 0.5, "");
        public final NumberSetting thick = number("thick", "Thickness", 1, 0.5, 3, 0.25, "");
        public final BoolSetting outline = bool("outline", "Dark outline", true);
        public final BoolSetting target = bool("target", "Turn red over a player or mob", true);

        public Crosshair() {
            super("crosshair", "Custom crosshair", "Your own crosshair: six shapes, size, gap, thickness, colour and outline; red over a target.", Category.HUD, false, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Pre e) {
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.CROSSHAIRS) return;
            e.setCanceled(true);
            if (mc().gameSettings.thirdPersonView != 0) return;
            float cx = e.resolution.getScaledWidth() / 2f, cy = e.resolution.getScaledHeight() / 2f;
            boolean onTarget = target.on() && mc().objectMouseOver != null && mc().objectMouseOver.entityHit != null;
            int c = onTarget ? 0xFFFF3A3A : world.on() ? 0xFF000000 | General.accent() : colour.argb();
            Gl.begin2D();
            if (outline.on()) draw(cx, cy, 0xC0000000, thick.f() + 1.2f);
            draw(cx, cy, c, thick.f());
            Gl.end2D();
        }

        private void draw(float cx, float cy, int c, float t) {
            float s = size.f(), g = gap.f();
            String sh = shape.get();
            if (sh.equals("cross") || sh.equals("crossdot")) {
                Draw.line(cx - g - s, cy, cx - g, cy, t, c);
                Draw.line(cx + g, cy, cx + g + s, cy, t, c);
                Draw.line(cx, cy - g - s, cx, cy - g, t, c);
                Draw.line(cx, cy + g, cx, cy + g + s, t, c);
            }
            if (sh.equals("dot") || sh.equals("crossdot")) Draw.circle(cx, cy, Math.max(0.8f, t * 0.9f), c);
            if (sh.equals("circle")) Draw.ring(cx, cy, s * 0.7f, s * 0.7f + t, c, c);
            if (sh.equals("chevron")) {
                Draw.line(cx - s, cy + s * 0.6f, cx, cy, t, c);
                Draw.line(cx, cy, cx + s, cy + s * 0.6f, t, c);
            }
            if (sh.equals("square")) {
                float h = s * 0.6f;
                Draw.line(cx - h, cy - h, cx + h, cy - h, t, c);
                Draw.line(cx + h, cy - h, cx + h, cy + h, t, c);
                Draw.line(cx + h, cy + h, cx - h, cy + h, t, c);
                Draw.line(cx - h, cy + h, cx - h, cy - h, t, c);
            }
        }
    }

    /** Black bars at the top and bottom for a film look (great for recording). */
    public static final class CinemaBars extends Module {
        public final NumberSetting height = number("height", "Bar height", 10, 4, 20, 1, "%");
        private float shown;
        private long last;

        public CinemaBars() {
            super("cinemabars", "Cinema bars", "Black bars slide in at the top and bottom for a film look (nice for recording).", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Pre e) {
            if (e.type != RenderGameOverlayEvent.ElementType.ALL) return;
            long now = System.nanoTime();
            float dt = last == 0 ? 0.016f : Math.min(0.1f, (now - last) / 1e9f);
            last = now;
            shown = Ease.approach(shown, enabled() ? 1 : 0, 6, dt);
            if (shown < 0.01f) return;
            int w = e.resolution.getScaledWidth(), h = e.resolution.getScaledHeight();
            float bh = h * height.f() / 100f * Ease.outCubic(shown);
            Gl.begin2D();
            Draw.rect(0, 0, w, bh, 0xFF000000);
            Draw.rect(0, h - bh, w, h, 0xFF000000);
            Gl.end2D();
        }
    }

    /** A quick flash of light across the screen on a kill. */
    public static final class KillFlash extends Module {
        public final BoolSetting world = bool("world", "In my world colour", true);
        private float at = -10;

        public KillFlash() {
            super("killflash", "Kill flash", "A quick flash of light across the screen when you get a kill.", Category.EFFECTS, false, true);
            CombatBus.listen(new CombatBus.Listener() {
                public void on(CombatVisualEvent e) {
                    if (e.type == CombatVisualEvent.Type.PLAYER_KILL_CONFIRMED) at = Ease.seconds();
                }
            });
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Post e) {
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.ALL) return;
            float t = Ease.seconds() - at;
            if (t > 0.45f) return;
            float a = (1 - Ease.outCubic(t / 0.45f)) * 0.35f;
            int c = world.on() ? 0xFF000000 | General.accent() : 0xFFFFFFFF;
            Gl.begin2D();
            Draw.rect(0, 0, e.resolution.getScaledWidth(), e.resolution.getScaledHeight(), Colors.alpha(c, a));
            Gl.end2D();
        }
    }
}
