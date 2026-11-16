package dev.vesper.client.render;

import java.awt.image.BufferedImage;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.util.ResourceLocation;

/**
 * Runtime-generated sprites (nothing copied): a soft round glow and a
 * four-point twinkle, both white so vertex colour tints them.
 */
public final class Textures {
    private static ResourceLocation glow, twinkle;

    private Textures() {
    }

    public static ResourceLocation glow() {
        if (glow == null) glow = make("vesper_glow", false);
        return glow;
    }

    public static ResourceLocation twinkle() {
        if (twinkle == null) twinkle = make("vesper_twinkle", true);
        return twinkle;
    }

    private static ResourceLocation make(String name, boolean star) {
        int n = 64;
        BufferedImage img = new BufferedImage(n, n, BufferedImage.TYPE_INT_ARGB);
        for (int y = 0; y < n; y++) {
            for (int x = 0; x < n; x++) {
                double fx = (x + 0.5) / (n / 2.0) - 1, fy = (y + 0.5) / (n / 2.0) - 1;
                double a;
                if (star) {
                    a = Math.max(Math.exp(-Math.abs(fx) * 18) * (1 - Math.abs(fy)), Math.exp(-Math.abs(fy) * 18) * (1 - Math.abs(fx)));
                    a = Math.max(a, Math.pow(Math.max(0, 1 - Math.sqrt(fx * fx + fy * fy) * 3.2), 2));
                } else {
                    double r = Math.min(1, Math.sqrt(fx * fx + fy * fy));
                    a = Math.pow(1 - r, 2.2);
                }
                int al = (int) Math.round(Math.max(0, Math.min(1, a)) * 255);
                img.setRGB(x, y, (al << 24) | 0xFFFFFF);
            }
        }
        return Minecraft.getMinecraft().getTextureManager().getDynamicTextureLocation(name, new DynamicTexture(img));
    }

    /** Forget the textures (after a resource reload they are made again). */
    public static void reset() {
        glow = null;
        twinkle = null;
    }
}
