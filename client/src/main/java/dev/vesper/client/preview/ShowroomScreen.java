package dev.vesper.client.preview;

import net.minecraft.client.gui.GuiScreen;

/** The local showroom: avatar turntable, cosmetics, skies, kill-effect test buttons (owner: cosmetics-animator). */
public class ShowroomScreen extends GuiScreen {
    private final GuiScreen parent;

    public ShowroomScreen(GuiScreen parent) {
        this.parent = parent;
    }

    @Override
    protected void keyTyped(char c, int key) {
        if (key == 1) mc.displayGuiScreen(parent);
    }
}
