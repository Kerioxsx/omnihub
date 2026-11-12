package dev.vesper.client.fx;

import dev.vesper.client.render.Textures;
import java.awt.image.BufferedImage;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.util.ResourceLocation;

/** Sprite lookup for effects: the shared glow/twinkle plus our own soft square (embers). */
public final class FxTextures {
    private static ResourceLocation square;

    private FxTextures() {
    }

    public static ResourceLocation get(int tex) {
        switch (tex) {
            case Particles.TEX_TWINKLE:
                return Textures.twinkle();
            case Particles.TEX_SQUARE:
                return square();
            default:
                return Textures.glow();
        }
    }

    /** 32x32 white square with a soft edge (`vesper:softsquare`). */
    public static ResourceLocation square() {
        if (square == null) {
            int n = 32;
            BufferedImage img = new BufferedImage(n, n, BufferedImage.TYPE_INT_ARGB);
            for (int y = 0; y < n; y++) {
                for (int x = 0; x < n; x++) {
                    double ex = Math.min(x + 0.5, n - x - 0.5) / 5.0, ey = Math.min(y + 0.5, n - y - 0.5) / 5.0;
                    double a = Math.min(1, Math.min(ex, ey));
                    a = a * a * (3 - 2 * a);
                    img.setRGB(x, y, ((int) Math.round(a * 255) << 24) | 0xFFFFFF);
                }
            }
            square = Minecraft.getMinecraft().getTextureManager().getDynamicTextureLocation("vesper_softsquare", new DynamicTexture(img));
        }
        return square;
    }
}
