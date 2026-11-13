package dev.vesper.client.avatar;

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
import dev.vesper.client.render.ObjModel;
import dev.vesper.client.render.Textures;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraftforge.client.event.RenderPlayerEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.opengl.GL11;

/**
 * Wraith head (an original take on a headless look): your head disappears,
 * a ghostly flame burns where it was, and two curved horns with spiked,
 * glowing tips rise from either side. Your own player only.
 */
public final class WraithHead extends Module {
    public static WraithHead INSTANCE;

    public final BoolSetting hideHead = bool("hideHead", "Hide my head", true);
    public final ColorSetting flame = color("flame", "Flame colour", 0xFF5CFFE0);
    public final NumberSetting flameSize = number("flameSize", "Flame size", 1.4, 0.4, 2.5, 0.05, "×");
    public final dev.vesper.client.module.setting.ChoiceSetting flameStyle = choice("flameStyle", "Flame style", "smoke", new String[] {"smoke", "crystal"}, new String[] {"Soft smoke", "Crystal shards"});
    public final BoolSetting horns = bool("horns", "Horns", true);
    public final ColorSetting hornColor = color("hornColor", "Horn colour", 0xFF2C2F38);
    public final ColorSetting tipColor = color("tipColor", "Horn tip colour", 0xFF3D7BFF);
    public final NumberSetting hornSize = number("hornSize", "Horn size", 0.75, 0.4, 1.4, 0.05, "×");

    public WraithHead() {
        super("wraith", "Wraith head", "Your head becomes a ghostly flame with spiked horns (only you see it).", Category.COSMETICS, false, true);
        INSTANCE = this;
    }

    /** Width of the flame at height 0..1: round at the bottom, widest low, pointed at the top. */
    private static float teardrop(float h) {
        return (float) (Math.sin(Math.min(1, h * 1.6) * Math.PI * 0.5) * Math.pow(1 - h, 0.85));
    }

    boolean hidesHead() {
        return enabled() && hideHead.on() && General.fx();
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase == TickEvent.Phase.END && enabled() && Minecraft.getMinecraft().thePlayer != null) VesperPlayerModel.install();
    }

    @SubscribeEvent
    public void onRenderPlayer(RenderPlayerEvent.Post e) {
        Minecraft mc = Minecraft.getMinecraft();
        EntityPlayer pl = e.entityPlayer;
        if (!enabled() || pl != mc.thePlayer || pl.isInvisible() || !General.fx()) return;
        float pt = e.partialRenderTick;
        float bodyYaw = pl.prevRenderYawOffset + (pl.renderYawOffset - pl.prevRenderYawOffset) * pt;
        float headYaw = pl.prevRotationYawHead + (pl.rotationYawHead - pl.prevRotationYawHead) * pt;
        float pitch = pl.prevRotationPitch + (pl.rotationPitch - pl.prevRotationPitch) * pt;
        float neck = 1.406f - (pl.isSneaking() ? 0.19f : 0);
        float t = Ease.seconds();
        GlStateManager.pushMatrix();
        GlStateManager.translate(e.x, e.y, e.z);
        GlStateManager.rotate(-bodyYaw, 0, 1, 0);
        GlStateManager.translate(0, neck, 0);
        GlStateManager.rotate(-(headYaw - bodyYaw), 0, 1, 0);
        GlStateManager.rotate(pitch, 1, 0, 0);
        GlStateManager.disableLighting();
        GlStateManager.disableCull();
        GlStateManager.enableBlend();
        GlStateManager.disableAlpha();
        GlStateManager.shadeModel(GL11.GL_SMOOTH);
        GlStateManager.disableTexture2D();
        GlStateManager.enableDepth();
        if (horns.on()) {
            ObjModel m = ObjModel.get("horns.obj");
            if (m.ok) {
                GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
                GlStateManager.pushMatrix();
                GlStateManager.translate(0, 0.1, 0);
                float s = hornSize.f();
                GlStateManager.scale(s, s, s);
                int base = hornColor.argb(), tip = tipColor.argb();
                m.draw("horn_base", base);
                m.draw("bands", Colors.mix(base, 0xFF000000, 0.5f));
                m.draw("horn_mid", Colors.mix(base, tip, 0.55f));
                m.draw("horn_tip", tip);
                m.draw("spikes", Colors.mix(base, 0xFF000000, 0.35f));
                m.draw("spikes_tip", Colors.mix(tip, 0xFFFFFFFF, 0.2f));
                GlStateManager.popMatrix();
            }
        }
        // The flame: glow sprites streaming up from the neck, wide at the base, licking upward.
        GlStateManager.depthMask(false);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        View.capture();
        int base = flame.argb();
        float fs = flameSize.f();
        if (flameStyle.is("crystal")) {
            // Shards tumbling inside a teardrop, bright at the core.
            FxDraw.geo();
            int n = General.detail() <= 0 ? 24 : 54;
            for (int i = 0; i < n; i++) {
                float ph = (t * 0.45f + i / (float) n) % 1f;
                float h = (0.05f + ph * 0.95f) * fs;
                float w = teardrop(ph) * 0.24f * fs;
                float ang = i * 2.399f + t * 0.9f;
                double cx = Math.cos(ang) * w * (0.4 + 0.6 * ((i * 7) % 10) / 10.0), cz = Math.sin(ang) * w * 0.6;
                float sz = (0.075f + 0.05f * ((i * 13) % 7) / 7f) * fs;
                float rot = t * 2 + i;
                double ux = Math.cos(rot) * sz, uy = Math.sin(rot) * sz;
                int c = Colors.alpha(Colors.mix(0xFFFFFFFF, base, 0.2f + 0.6f * ph), 0.95f * (1 - ph * ph));
                double ax = cx + View.rx * ux + View.ux * uy, ay = h + View.ry * ux + View.uy * uy, az = cz + View.rz * ux + View.uz * uy;
                double bx = cx - View.rx * uy + View.ux * ux * 0.5, by = h - View.ry * uy + View.uy * ux * 0.5, bz = cz - View.rz * uy + View.uz * ux * 0.5;
                double qx = cx - View.rx * ux - View.ux * uy, qy = h - View.ry * ux - View.uy * uy, qz = cz - View.rz * ux - View.uz * uy;
                FxDraw.quad(cx, h, cz, ax, ay, az, bx, by, bz, qx, qy, qz, c, c, Colors.alpha(c, 0.4f), c);
            }
            FxDraw.flush();
        }
        FxDraw.sprites(Textures.glow());
        int n = General.detail() <= 0 ? 24 : 52;
        float soft = flameStyle.is("crystal") ? 0.6f : 1f;
        for (int i = 0; i < n; i++) {
            float ph = (t * 1.25f + i / (float) n) % 1f;
            float h = (0.04f + ph * 0.95f) * fs;
            float spread = teardrop(ph) * 0.21f * fs * (0.75f + 0.25f * (float) Math.sin(i * 2.3f));
            float ang = i * 2.399f + t * 1.7f;
            double x = Math.cos(ang) * spread + Math.sin(t * 5 + i) * 0.025 * ph;
            double z = Math.sin(ang) * spread * 0.7;
            float size = (0.13f + 0.17f * teardrop(ph)) * fs;
            int c = Colors.mix(Colors.mix(base, 0xFFFFFFFF, 0.6f * (1 - ph) * (1 - ph)), base, ph);
            FxDraw.sprite(x, h, z, size, Colors.alpha(c, 0.4f * soft * (1 - ph * ph)));
        }
        FxDraw.sprite(0, 0.38 * fs, 0, 0.5 * fs, Colors.alpha(base, (0.22f + 0.05f * (float) Math.sin(t * 6)) * soft));
        FxDraw.sprite(0, 0.26 * fs, 0, 0.2 * fs, Colors.alpha(0xFFFFFFFF, 0.35f * soft));
        FxDraw.flush();
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
}
