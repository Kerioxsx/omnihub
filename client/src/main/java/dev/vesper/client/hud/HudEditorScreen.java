package dev.vesper.client.hud;

import net.minecraft.client.gui.GuiScreen;

/** Drag/scale/hide HUD elements (owner: hud-pvp). */
public class HudEditorScreen extends GuiScreen {
    private final GuiScreen parent;

    public HudEditorScreen(GuiScreen parent) {
        this.parent = parent;
    }

    @Override
    protected void keyTyped(char c, int key) {
        if (key == 1) mc.displayGuiScreen(parent);
    }
}
