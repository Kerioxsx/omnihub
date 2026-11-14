package dev.vesper.client.avatar;

import dev.vesper.client.VesperClient;
import java.lang.reflect.Field;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.model.ModelBase;
import net.minecraft.client.model.ModelPlayer;
import net.minecraft.client.renderer.entity.RenderPlayer;
import net.minecraft.client.renderer.entity.RendererLivingEntity;
import net.minecraft.entity.Entity;

/**
 * Minecraft's player model, plus Vesper's emote poses and the Wraith head
 * (head hidden) for your own player only. Every other player is drawn
 * exactly as Minecraft draws them. Installed in place of the game's model
 * by reflection, like the sword animations (ADR-8).
 */
public final class VesperPlayerModel extends ModelPlayer {
    private static boolean installed, failed;

    public VesperPlayerModel(float size, boolean smallArms) {
        super(size, smallArms);
    }

    @Override
    public void setRotationAngles(float limbSwing, float limbSwingAmount, float ageInTicks, float netHeadYaw, float headPitch, float scale, Entity entity) {
        super.setRotationAngles(limbSwing, limbSwingAmount, ageInTicks, netHeadYaw, headPitch, scale, entity);
        boolean self = entity != null && entity == Minecraft.getMinecraft().thePlayer;
        boolean headless = self && WraithHead.INSTANCE != null && WraithHead.INSTANCE.hidesHead();
        bipedHead.showModel = !headless;
        bipedHeadwear.showModel = !headless;
        if (self && Emotes.INSTANCE != null && Emotes.INSTANCE.active()) {
            Emotes.INSTANCE.pose(this);
            copyModelAngles(bipedHead, bipedHeadwear);
            copyModelAngles(bipedRightArm, bipedRightArmwear);
            copyModelAngles(bipedLeftArm, bipedLeftArmwear);
            copyModelAngles(bipedRightLeg, bipedRightLegwear);
            copyModelAngles(bipedLeftLeg, bipedLeftLegwear);
            copyModelAngles(bipedBody, bipedBodyWear);
        }
    }

    /** Swaps the model in every player renderer (default and slim arms). */
    public static void install() {
        if (installed || failed) return;
        try {
            Map<String, RenderPlayer> skins = Minecraft.getMinecraft().getRenderManager().getSkinMap();
            Field main = null;
            for (Field f : RendererLivingEntity.class.getDeclaredFields()) if (f.getType() == ModelBase.class) main = f;
            Field slim = null;
            for (Field f : ModelPlayer.class.getDeclaredFields()) if (f.getType() == boolean.class) slim = f;
            if (main == null || slim == null) throw new IllegalStateException("model fields not found");
            main.setAccessible(true);
            slim.setAccessible(true);
            int n = 0;
            for (RenderPlayer r : skins.values()) {
                Object m = main.get(r);
                if (m instanceof VesperPlayerModel) continue;
                boolean small = m instanceof ModelPlayer && slim.getBoolean(m);
                main.set(r, new VesperPlayerModel(0f, small));
                n++;
            }
            installed = n > 0;
        } catch (Throwable t) {
            failed = true;
            VesperClient.LOG.warn("Emotes and the Wraith head are unavailable: {}", t.toString());
        }
    }
}
