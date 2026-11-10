package dev.vesper.client.gui;

import dev.vesper.client.module.General;
import net.minecraft.client.Minecraft;
import net.minecraft.client.audio.PositionedSoundRecord;
import net.minecraft.util.ResourceLocation;

/** Menu sounds, using Minecraft's own sound events (nothing bundled). */
public final class UiSound {
    private UiSound() {
    }

    public static void press() {
        play("gui.button.press", 1.0f);
    }

    public static void hover() {
        play("random.click", 1.9f);
    }

    private static void play(String id, float pitch) {
        if (General.INSTANCE != null && !General.INSTANCE.uiSounds.on()) return;
        Minecraft.getMinecraft().getSoundHandler().playSound(PositionedSoundRecord.create(new ResourceLocation(id), pitch));
    }
}
