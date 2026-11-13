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
    public final BoolSetting horns = bool("horns", "Horns", true);
    public final ColorSetting hornColor = color("hornColor", "Horn colour", 0xFF2C2F38);
    public final ColorSetting tipColor = color("tipColor", "Horn tip colour", 0xFF3D7BFF);
    public final NumberSetting hornSize = number("hornSize", "Horn size", 1.0, 0.5, 1.6, 0.05, "×");

    public WraithHead() {
        super("wraith", "Wraith head", "Your head becomes a ghostly flame with spiked horns (only you see it).", Category.COSMETICS, false, true);
        INSTANCE = this;
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
                GlStateManager.translate(0, 0.22, 0);
                float s = hornSize.f();
                GlStateManager.scale(s, s, s);
                m.draw("horn_base", hornColor.argb());
                m.draw("horn_tip", tipColor.argb());
                m.draw("spikes", Colors.mix(tipColor.argb(), 0xFF0A0F2A, 0.35f));
                GlStateManager.popMatrix();
            }
        }
        // The flame: glow sprites streaming up from the neck, wide at the base, licking upward.
        GlStateManager.depthMask(false);
        GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE);
        View.capture();
        FxDraw.sprites(Textures.glow());
        int base = flame.argb();
        float fs = flameSize.f();
        int n = General.detail() <= 0 ? 22 : 46;
        for (int i = 0; i < n; i++) {
            float ph = (t * 1.3f + i / (float) n) % 1f;
            float h = ph * 1.0f * fs;
            float spread = 0.2f * fs * (float) Math.pow(1 - ph, 0.6) * (0.7f + 0.3f * (float) Math.sin(i * 2.3f));
            float ang = i * 2.399f + t * 1.7f;
            double x = Math.cos(ang) * spread + Math.sin(t * 5 + i) * 0.03 * ph;
            double z = Math.sin(ang) * spread;
            float size = (0.28f - 0.18f * ph) * fs;
            int c = Colors.mix(Colors.mix(base, 0xFFFFFFFF, 0.55f * (1 - ph) * (1 - ph)), base, ph);
            FxDraw.sprite(x, 0.05 + h, z, size, Colors.alpha(c, 0.42f * (1 - ph * ph)));
        }
        FxDraw.sprite(0, 0.25 * fs, 0, 0.55 * fs, Colors.alpha(base, 0.25f + 0.05f * (float) Math.sin(t * 6)));
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
