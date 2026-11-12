package dev.vesper.client.preview;

import com.mojang.authlib.GameProfile;
import com.mojang.authlib.minecraft.MinecraftProfileTexture;
import dev.vesper.client.VesperClient;
import net.minecraft.client.Minecraft;
import net.minecraft.client.model.ModelBase;
import net.minecraft.client.model.ModelPlayer;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.resources.DefaultPlayerSkin;
import net.minecraft.client.resources.SkinManager;
import net.minecraft.util.ResourceLocation;

/**
 * A player model with the user's own skin, drawn without a world (menus,
 * showroom). The skin is looked up once in the background through
 * Minecraft's own session and skin services; until then (or offline) the
 * default skin is used.
 */
public final class Avatar {
    private static ResourceLocation skin;
    private static boolean slim, requested;
    private static ModelPlayer wide, thin;
    private static ModelPlayer npcModel;

    private Avatar() {
    }

    private static void request() {
        if (requested) return;
        requested = true;
        final Minecraft mc = Minecraft.getMinecraft();
        final GameProfile base = mc.getSession().getProfile();
        Thread t = new Thread(new Runnable() {
            public void run() {
                try {
                    final GameProfile full = mc.getSessionService().fillProfileProperties(base, false);
                    mc.addScheduledTask(new Runnable() {
                        public void run() {
                            mc.getSkinManager().loadProfileTextures(full, new SkinManager.SkinAvailableCallback() {
                                public void skinAvailable(MinecraftProfileTexture.Type type, ResourceLocation loc, MinecraftProfileTexture tex) {
                                    if (type == MinecraftProfileTexture.Type.SKIN) {
                                        skin = loc;
                                        slim = "slim".equals(tex.getMetadata("model"));
                                    }
                                }
                            }, false);
                        }
                    });
                } catch (Throwable e) {
                    VesperClient.LOG.info("Skin lookup unavailable ({}); using the default skin", e.toString());
                }
            }
        }, "Vesper skin lookup");
        t.setDaemon(true);
        t.start();
    }

    private static ResourceLocation skin() {
        request();
        if (skin != null) return skin;
        return DefaultPlayerSkin.getDefaultSkin(Minecraft.getMinecraft().getSession().getProfile().getId());
    }

    private static boolean slimArms() {
        if (skin != null) return slim;
        return "slim".equals(DefaultPlayerSkin.getSkinType(Minecraft.getMinecraft().getSession().getProfile().getId()));
    }

    private static ModelPlayer model() {
        if (slimArms()) {
            if (thin == null) thin = new ModelPlayer(0f, true);
            return thin;
        }
        if (wide == null) wide = new ModelPlayer(0f, false);
        return wide;
    }

    /**
     * Draws the model in model space (y down, 1 unit = 16 px, feet at y=1.5,
     * facing -z), with an idle animation at `time` seconds. Texture and
     * colour are set here; the caller sets up the matrix and lighting.
     */
    public static void drawModel(float time, float alpha, float headYaw, boolean npc) {
        ModelPlayer m;
        if (npc) {
            if (npcModel == null) npcModel = new ModelPlayer(0f, false);
            m = npcModel;
            Minecraft.getMinecraft().getTextureManager().bindTexture(DefaultPlayerSkin.getDefaultSkinLegacy());
        } else {
            m = model();
            Minecraft.getMinecraft().getTextureManager().bindTexture(skin());
        }
        float breathe = (float) Math.sin(time * 1.8f);
        m.bipedHead.rotateAngleY = (float) Math.toRadians(headYaw);
        m.bipedHead.rotateAngleX = 0.04f * breathe;
        m.bipedRightArm.rotateAngleZ = 0.05f + 0.04f * (float) Math.cos(time * 1.8f);
        m.bipedLeftArm.rotateAngleZ = -0.05f - 0.04f * (float) Math.cos(time * 1.8f);
        m.bipedRightArm.rotateAngleX = 0.03f * breathe;
        m.bipedLeftArm.rotateAngleX = -0.03f * breathe;
        m.bipedRightLeg.rotateAngleX = 0;
        m.bipedLeftLeg.rotateAngleX = 0;
        ModelBase.copyModelAngles(m.bipedHead, m.bipedHeadwear);
        ModelBase.copyModelAngles(m.bipedRightArm, m.bipedRightArmwear);
        ModelBase.copyModelAngles(m.bipedLeftArm, m.bipedLeftArmwear);
        ModelBase.copyModelAngles(m.bipedRightLeg, m.bipedRightLegwear);
        ModelBase.copyModelAngles(m.bipedLeftLeg, m.bipedLeftLegwear);
        ModelBase.copyModelAngles(m.bipedBody, m.bipedBodyWear);
        GlStateManager.enableTexture2D();
        GlStateManager.color(1, 1, 1, alpha);
        float s = 0.0625f;
        m.bipedHead.render(s);
        m.bipedBody.render(s);
        m.bipedRightArm.render(s);
        m.bipedLeftArm.render(s);
        m.bipedRightLeg.render(s);
        m.bipedLeftLeg.render(s);
        GlStateManager.enableBlend();
        GlStateManager.blendFunc(770, 771);
        m.bipedHeadwear.render(s);
        m.bipedBodyWear.render(s);
        m.bipedRightArmwear.render(s);
        m.bipedLeftArmwear.render(s);
        m.bipedRightLegwear.render(s);
        m.bipedLeftLegwear.render(s);
        GlStateManager.color(1, 1, 1, 1);
    }
}
