package dev.vesper.client.preview;

import dev.vesper.client.cosmetic.Cosmetics;
import dev.vesper.client.cosmetic.Scene;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.OpenGlHelper;
import net.minecraft.client.renderer.RenderHelper;

/**
 * Draws the user's own skin on a player model, with the equipped
 * cosmetics, inside a GUI. No world needed.
 */
public final class PlayerPreview {
    private PlayerPreview() {
    }

    /**
     * @param feetX GUI x of the feet
     * @param feetY GUI y of the feet
     * @param height model height in GUI units
     * @param yaw body yaw in degrees (0 = facing the viewer)
     * @param alpha 0..1 fade
     * @param time seconds, for animation
     */
    public static void draw(float feetX, float feetY, float height, float yaw, float alpha, float time) {
        if (alpha <= 0.01f) return;
        float block = height / 1.875f;
        GlStateManager.pushMatrix();
        GlStateManager.enableDepth();
        GlStateManager.depthMask(true);
        GlStateManager.translate(feetX, feetY, 300);

        // The model (y down like the GUI); mirrored on x so it reads like a person facing you.
        GlStateManager.pushMatrix();
        GlStateManager.scale(-block * 0.9375f, block * 0.9375f, block * 0.9375f);
        GlStateManager.rotate(180 + yaw, 0, 1, 0);
        GlStateManager.translate(0, -1.5f, 0);
        GlStateManager.enableRescaleNormal();
        GlStateManager.enableAlpha();
        GlStateManager.enableBlend();
        GlStateManager.disableCull();
        RenderHelper.enableStandardItemLighting();
        Avatar.drawModel(time, alpha, -yaw * 0.4f, false);
        RenderHelper.disableStandardItemLighting();
        GlStateManager.disableRescaleNormal();
        GlStateManager.popMatrix();

        // Cosmetics: their space is y up, facing +z (toward the viewer).
        GlStateManager.pushMatrix();
        GlStateManager.scale(block, -block, block);
        Cosmetics.drawFor(Scene.PREVIEW, yaw, alpha);
        GlStateManager.popMatrix();

        GlStateManager.disableDepth();
        GlStateManager.setActiveTexture(OpenGlHelper.lightmapTexUnit);
        GlStateManager.disableTexture2D();
        GlStateManager.setActiveTexture(OpenGlHelper.defaultTexUnit);
        GlStateManager.enableTexture2D();
        GlStateManager.popMatrix();
    }
}
