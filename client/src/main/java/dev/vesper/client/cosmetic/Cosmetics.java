package dev.vesper.client.cosmetic;

import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Ease;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraftforge.client.event.RenderPlayerEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;

/**
 * Cosmetics: wings, comet plume with thorn crown, orbiting glyph tokens,
 * halo, floor ring and glints. Drawn on your own player only (third person
 * and the inventory), the title-screen preview and the showroom. Nobody
 * else sees them.
 */
public final class Cosmetics {
    private Cosmetics() {
    }

    public static final class Wings extends Module {
        public final ChoiceSetting colourway = choice("colourway", "Colours", "palette", Palette.WING_IDS, Palette.WING_NAMES);
        public final ChoiceSetting palette = choice("palette", "Palette", "shared", Palette.OVERRIDE_IDS, Palette.OVERRIDE_NAMES);
        public final NumberSetting size = number("size", "Size", 0.85, 0.5, 1.3, 0.05, "×");
        public final NumberSetting flap = number("flap", "Flap", 1.0, 0, 2, 0.1, "×");
        public final NumberSetting glow = number("glow", "Glow", 1.0, 0, 2, 0.1, "×");

        Wings() {
            super("wings", "Celestial wings", "Membrane wings with glowing tips that flap faster when you sprint or fly.", Category.COSMETICS, true, true);
        }
    }

    public static final class Comet extends Module {
        public final ChoiceSetting variant = choice("variant", "Style", "comet", new String[] {"comet", "orb", "wisp"}, new String[] {"Comet", "Orb", "Wisp"});
        public final ChoiceSetting palette = choice("palette", "Palette", "shared", Palette.OVERRIDE_IDS, Palette.OVERRIDE_NAMES);
        public final NumberSetting length = number("length", "Trail length", 1.4, 0.4, 2.5, 0.1, " blocks");
        public final NumberSetting glow = number("glow", "Glow", 1.0, 0, 2, 0.1, "×");
        public final NumberSetting sparks = number("sparks", "Sparks", 10, 0, 30, 1, "");
        public final BoolSetting crown = bool("crown", "Thorn crown", true);

        Comet() {
            super("comet", "Comet plume", "A comet burning above your head, with a thorn crown under it.", Category.COSMETICS, false, true);
        }
    }

    public static final class Orbit extends Module {
        public final ChoiceSetting palette = choice("palette", "Palette", "shared", Palette.OVERRIDE_IDS, Palette.OVERRIDE_NAMES);
        public final NumberSetting count = number("count", "Tokens", 3, 2, 6, 1, "");
        public final NumberSetting radius = number("radius", "Radius", 0.85, 0.5, 1.4, 0.05, " blocks");
        public final NumberSetting speed = number("speed", "Speed", 1.0, 0.2, 3, 0.1, "×");
        public final NumberSetting size = number("size", "Size", 1.0, 0.6, 1.6, 0.05, "×");

        Orbit() {
            super("orbit", "Orbiting glyphs", "Glowing tokens with original glyphs circling you.", Category.COSMETICS, false, true);
        }
    }

    public static final class Halo extends Module {
        public final ChoiceSetting palette = choice("palette", "Palette", "shared", Palette.OVERRIDE_IDS, Palette.OVERRIDE_NAMES);

        Halo() {
            super("halo", "Halo", "A thin glowing ring floating over your head.", Category.COSMETICS, false, true);
        }
    }

    public static final class FloorRing extends Module {
        public final ChoiceSetting palette = choice("palette", "Palette", "shared", Palette.OVERRIDE_IDS, Palette.OVERRIDE_NAMES);
        public final NumberSetting radius = number("radius", "Radius", 0.6, 0.3, 1.2, 0.05, " blocks");

        FloorRing() {
            super("floorring", "Floor ring", "A slowly turning ring of light at your feet.", Category.COSMETICS, false, true);
        }
    }

    public static final class Glints extends Module {
        public final NumberSetting amount = number("amount", "Amount", 8, 2, 24, 1, "");

        Glints() {
            super("glints", "Glints", "Tiny sparkles that come and go around you.", Category.COSMETICS, true, true);
        }
    }

    /** All cosmetic modules. */
    public static final class Set {
        public final Wings wings = new Wings();
        public final Comet comet = new Comet();
        public final Orbit orbit = new Orbit();
        public final Halo halo = new Halo();
        public final FloorRing floor = new FloorRing();
        public final Glints glints = new Glints();
    }

    private static Set set;
    private static final Pose POSE = new Pose();

    public static Set set() {
        return set;
    }

    public static void init() {
        ModuleRegistry.register(new CosmeticStyle());
        set = new Set();
        ModuleRegistry.register(set.wings);
        ModuleRegistry.register(set.comet);
        ModuleRegistry.register(set.orbit);
        ModuleRegistry.register(set.halo);
        ModuleRegistry.register(set.floor);
        ModuleRegistry.register(set.glints);
        MinecraftForge.EVENT_BUS.register(new Cosmetics.Handler());
    }

    /** Draws the cosmetics for a GUI/stage figure (origin at the feet, matrix already placed). */
    public static void drawFor(Scene scene, float bodyYaw, float alpha) {
        POSE.idle(bodyYaw, alpha);
        scene.advance(POSE, Ease.seconds());
        CosmeticRenderer.draw(POSE);
    }

    public static final class Handler {
        @SubscribeEvent
        public void onRenderPlayer(RenderPlayerEvent.Post e) {
            Minecraft mc = Minecraft.getMinecraft();
            EntityPlayer pl = e.entityPlayer;
            if (pl != mc.thePlayer || pl.isInvisible() || !General.fx()) return;
            boolean inventory = mc.currentScreen != null;
            if (inventory && CosmeticStyle.INSTANCE != null && !CosmeticStyle.INSTANCE.inventory.on()) return;
            float pt = e.partialRenderTick;
            Pose p = POSE;
            p.partialTicks = pt;
            p.bodyYaw = pl.prevRenderYawOffset + (pl.renderYawOffset - pl.prevRenderYawOffset) * pt;
            p.headYaw = 0;
            p.sneaking = pl.isSneaking();
            p.fast = pl.isSprinting() || pl.capabilities.isFlying || !pl.onGround;
            p.limbSwing = pl.limbSwing;
            p.limbSwingAmount = pl.limbSwingAmount;
            p.alpha = 1;
            p.ground = 0;
            Scene.WORLD.advance(p, Ease.seconds());
            GlStateManager.pushMatrix();
            GlStateManager.translate(e.x, e.y, e.z);
            CosmeticRenderer.draw(p);
            GlStateManager.popMatrix();
        }
    }
}
