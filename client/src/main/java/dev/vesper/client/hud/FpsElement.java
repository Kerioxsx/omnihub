package dev.vesper.client.hud;

import net.minecraft.client.Minecraft;

/** Frames per second as Minecraft counts them (Minecraft.getDebugFPS, updated once a second). */
final class FpsElement extends LineElement {
    FpsElement() {
        super("fps", "FPS", "Frames per second, from the game's own counter.", false, 0.01f, 0.10f);
    }

    @Override
    public boolean hasContent() {
        return true;
    }

    @Override
    protected void compute(boolean sample) {
        int fps = sample ? 144 : Minecraft.getDebugFPS();
        label = "FPS";
        value = String.valueOf(fps);
        valueColor = fps >= 120 ? GOOD : fps >= 60 ? TEXT : BAD;
    }
}
