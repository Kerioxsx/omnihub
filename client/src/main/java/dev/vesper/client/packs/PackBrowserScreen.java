package dev.vesper.client.packs;

import dev.vesper.client.module.General;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiButton;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.gui.GuiScreenResourcePacks;
import net.minecraft.client.gui.GuiTextField;
import net.minecraftforge.client.event.GuiScreenEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import org.lwjgl.input.Keyboard;
import org.lwjgl.input.Mouse;

/**
 * Browse and download resource packs for 1.8.9 from Modrinth inside the
 * game. Downloaded packs land in the resourcepacks folder and show up in
 * Minecraft's Resource Packs screen right away (turn them on there).
 */
public final class PackBrowserScreen extends GuiScreen {
    private final GuiScreen parent;
    private GuiTextField search;
    private volatile List<Modrinth.Pack> hits = new ArrayList<Modrinth.Pack>();
    private volatile String status = "Loading popular packs…";
    private final Set<String> busy = new HashSet<String>(), done = new HashSet<String>();
    private float scroll;
    private long typedAt;
    private String lastQuery = null;

    public PackBrowserScreen(GuiScreen parent) {
        this.parent = parent;
    }

    /** Adds a "Browse online" button to Minecraft's Resource Packs screen. */
    public static void hook() {
        MinecraftForge.EVENT_BUS.register(new Object() {
            @SubscribeEvent
            public void onInit(GuiScreenEvent.InitGuiEvent.Post e) {
                if (e.gui instanceof GuiScreenResourcePacks) e.buttonList.add(new GuiButton(7731, e.gui.width - 128, 6, 120, 20, "Browse online (Vesper)"));
            }

            @SubscribeEvent
            public void onAction(GuiScreenEvent.ActionPerformedEvent.Pre e) {
                if (e.gui instanceof GuiScreenResourcePacks && e.button.id == 7731) Minecraft.getMinecraft().displayGuiScreen(new PackBrowserScreen(e.gui));
            }
        });
    }

    @Override
    public void initGui() {
        Keyboard.enableRepeatEvents(true);
        search = new GuiTextField(1, fontRendererObj, width / 2 - 150, 34, 300, 18);
        search.setMaxStringLength(60);
        search.setFocused(true);
        buttonList.clear();
        buttonList.add(new GuiButton(0, width / 2 - 100, height - 28, 200, 20, "Done"));
        if (lastQuery == null) run("");
    }

    @Override
    public void onGuiClosed() {
        Keyboard.enableRepeatEvents(false);
    }

    private void run(final String q) {
        lastQuery = q;
        status = "Searching Modrinth…";
        Thread t = new Thread(new Runnable() {
            public void run() {
                try {
                    List<Modrinth.Pack> r = Modrinth.search(q);
                    if (!q.equals(lastQuery)) return;
                    hits = r;
                    scroll = 0;
                    status = r.isEmpty() ? "No 1.8.9 packs found for that." : r.size() + " packs for 1.8.9 on Modrinth";
                } catch (Exception e) {
                    status = "Couldn't reach Modrinth: " + e.getMessage();
                }
            }
        }, "Vesper pack search");
        t.setDaemon(true);
        t.start();
    }

    private void download(final Modrinth.Pack p) {
        if (busy.contains(p.id)) return;
        busy.add(p.id);
        final File dir = mc.getResourcePackRepository().getDirResourcepacks();
        Thread t = new Thread(new Runnable() {
            public void run() {
                try {
                    final String name = Modrinth.install(p.id, dir);
                    done.add(p.id);
                    status = "Downloaded " + name + ". Turn it on in Resource Packs.";
                    mc.addScheduledTask(new Runnable() {
                        public void run() {
                            mc.getResourcePackRepository().updateRepositoryEntriesAll();
                        }
                    });
                } catch (Exception e) {
                    status = "Couldn't download " + p.title + ": " + e.getMessage();
                } finally {
                    busy.remove(p.id);
                }
            }
        }, "Vesper pack download");
        t.setDaemon(true);
        t.start();
    }

    @Override
    public void updateScreen() {
        search.updateCursorCounter();
        String q = search.getText();
        if (!q.equals(lastQuery) && System.currentTimeMillis() - typedAt > 400) run(q);
    }

    @Override
    protected void keyTyped(char c, int key) throws IOException {
        if (key == Keyboard.KEY_ESCAPE) {
            mc.displayGuiScreen(parent);
            return;
        }
        if (search.textboxKeyTyped(c, key)) typedAt = System.currentTimeMillis();
    }

    @Override
    protected void actionPerformed(GuiButton b) {
        if (b.id == 0) mc.displayGuiScreen(parent);
    }

    @Override
    public void handleMouseInput() throws IOException {
        super.handleMouseInput();
        int w = Mouse.getEventDWheel();
        if (w != 0) scroll = Math.max(0, Math.min(Math.max(0, hits.size() * 34 - (height - 110)), scroll - Math.signum(w) * 34));
    }

    private int rowX() {
        return width / 2 - 190;
    }

    @Override
    protected void mouseClicked(int mx, int my, int button) throws IOException {
        super.mouseClicked(mx, my, button);
        search.mouseClicked(mx, my, button);
        List<Modrinth.Pack> list = hits;
        int y0 = 62 - (int) scroll;
        for (int i = 0; i < list.size(); i++) {
            int y = y0 + i * 34;
            if (y < 56 || y + 30 > height - 52) continue;
            int bx = rowX() + 380 - 76;
            if (mx >= bx && mx <= bx + 70 && my >= y + 6 && my <= y + 24) download(list.get(i));
        }
    }

    @Override
    public void drawScreen(int mx, int my, float pt) {
        drawDefaultBackground();
        int accent = General.accent();
        GlyphFont.drawCentered("Resource packs for 1.8.9", width / 2f, 10, 12, true, 0xFFFFFFFF);
        search.drawTextBox();
        if (search.getText().isEmpty() && !search.isFocused()) drawString(fontRendererObj, "Search…", width / 2 - 146, 39, 0xFF808080);
        List<Modrinth.Pack> list = hits;
        int y0 = 62 - (int) scroll;
        int x = rowX();
        for (int i = 0; i < list.size(); i++) {
            int y = y0 + i * 34;
            if (y < 56 || y + 30 > height - 52) continue;
            Modrinth.Pack p = list.get(i);
            Gl.begin2D();
            Draw.roundRect(x, y, 380, 30, 5, 0xAA14141C);
            int bx = x + 380 - 76;
            boolean hover = mx >= bx && mx <= bx + 70 && my >= y + 6 && my <= y + 24;
            int bc = done.contains(p.id) ? 0xFF2E8B57 : hover ? accent : Colors.alpha(accent, 0.6f);
            Draw.roundRect(bx, y + 6, 70, 18, 4, bc);
            Gl.end2D();
            String title = p.title.length() > 40 ? p.title.substring(0, 39) + "…" : p.title;
            GlyphFont.draw(title, x + 8, y + 3, 9, true, 0xFFFFFFFF);
            String desc = p.description.length() > 62 ? p.description.substring(0, 61) + "…" : p.description;
            GlyphFont.draw(desc, x + 8, y + 16, 7, false, 0xFFA8ACB8);
            String label = busy.contains(p.id) ? "…" : done.contains(p.id) ? "Added" : "Download";
            GlyphFont.drawCentered(label, bx + 35, y + 10, 8, true, 0xFF0B0B10);
        }
        GlyphFont.drawCentered(status, width / 2f, height - 44, 8, false, 0xFFC8CCD6);
        super.drawScreen(mx, my, pt);
    }

    @Override
    public boolean doesGuiPauseGame() {
        return true;
    }
}
