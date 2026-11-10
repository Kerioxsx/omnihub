package dev.vesper.client.gui;

import net.minecraft.client.gui.GuiButton;
import net.minecraft.client.gui.GuiMultiplayer;
import net.minecraft.client.gui.GuiOptions;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.gui.GuiSelectWorld;

/** Placeholder title used for the Gate 0 launch check (replaced in Gate 1). */
public class TitleScreen extends GuiScreen {
    public static boolean enabled() {
        return true;
    }

    @Override
    public void initGui() {
        buttonList.clear();
        int y = height / 2 - 40;
        buttonList.add(new GuiButton(0, width / 2 - 100, y, "Singleplayer"));
        buttonList.add(new GuiButton(1, width / 2 - 100, y + 24, "Multiplayer"));
        buttonList.add(new GuiButton(2, width / 2 - 100, y + 48, "Minecraft Settings"));
        buttonList.add(new GuiButton(3, width / 2 - 100, y + 72, "Quit Game"));
    }

    @Override
    protected void actionPerformed(GuiButton b) {
        if (b.id == 0) mc.displayGuiScreen(new GuiSelectWorld(this));
        if (b.id == 1) mc.displayGuiScreen(new GuiMultiplayer(this));
        if (b.id == 2) mc.displayGuiScreen(new GuiOptions(this, mc.gameSettings));
        if (b.id == 3) mc.shutdown();
    }

    @Override
    public void drawScreen(int mx, int my, float pt) {
        drawRect(0, 0, width, height, 0xFF0B0B0E);
        drawCenteredString(fontRendererObj, "Vesper Client", width / 2, height / 2 - 70, 0xFFFFFF);
        super.drawScreen(mx, my, pt);
    }
}
