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
import net.minecraft.client.resources.ResourcePackRepository;
import net.minecraftforge.client.event.GuiScreenEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import org.lwjgl.input.Keyboard;
import org.lwjgl.input.Mouse;

/**
 * Browse and install resource packs for 1.8.9 from Modrinth inside the
 * game. Install downloads the pack into the resourcepacks folder and turns
 * it on at the top of your packs; clicking it again turns it off.
 */
public final class PackBrowserScreen extends GuiScreen {
    private final GuiScreen parent;
    private GuiTextField search;
    private volatile List<Modrinth.Pack> hits = new ArrayList<Modrinth.Pack>();
    private volatile String status = "Loading popular packs…";
    private final Set<String> busy = new HashSet<String>(), done = new HashSet<String>();
    private final java.util.Map<String, String> names = new java.util.HashMap<String, String>();
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
                if (e.gui instanceof GuiScreenResourcePacks) e.buttonList.add(new GuiButton(7731, e.gui.width - 128, 6, 120, 20, "Get packs (Vesper)"));
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

    /**
     * Turns the pack in the resourcepacks folder called {@code file} on (at
     * the top, so it wins over the others) or off, saves that in options.txt
     * and reloads the game's resources. Main thread only.
     */
    static boolean setEnabled(Minecraft mc, String file, boolean on) {
        ResourcePackRepository repo = mc.getResourcePackRepository();
        repo.updateRepositoryEntriesAll();
        List<ResourcePackRepository.Entry> active = new ArrayList<ResourcePackRepository.Entry>(repo.getRepositoryEntries());
        ResourcePackRepository.Entry pick = null;
        for (ResourcePackRepository.Entry e : repo.getRepositoryEntriesAll()) if (e.getResourcePackName().equals(file)) pick = e;
        if (pick == null) return false;
        for (ResourcePackRepository.Entry e : new ArrayList<ResourcePackRepository.Entry>(active)) if (e.getResourcePackName().equals(file)) active.remove(e);
        // The last entry is the one on top in the Resource Packs screen.
        if (on) active.add(pick);
        repo.setRepositories(active);
        mc.gameSettings.resourcePacks.clear();
        mc.gameSettings.incompatibleResourcePacks.clear();
        for (ResourcePackRepository.Entry e : active) {
            mc.gameSettings.resourcePacks.add(e.getResourcePackName());
            if (e.func_183027_f() != 1) mc.gameSettings.incompatibleResourcePacks.add(e.getResourcePackName());
        }
        mc.gameSettings.saveOptions();
        mc.refreshResources();
        return true;
    }

    private void download(final Modrinth.Pack p) {
        if (busy.contains(p.id)) return;
        if (done.contains(p.id) && names.containsKey(p.id)) {
            // Installed and on: a second click turns it off again.
            if (setEnabled(mc, names.get(p.id), false)) {
                done.remove(p.id);
                status = "Turned off " + p.title + ". Click Install to turn it back on.";
            }
            return;
        }
        busy.add(p.id);
        final File dir = mc.getResourcePackRepository().getDirResourcepacks();
        Thread t = new Thread(new Runnable() {
            public void run() {
                try {
                    final String name = Modrinth.install(p.id, dir);
                    names.put(p.id, name);
                    status = "Turning on " + name + "…";
                    mc.addScheduledTask(new Runnable() {
                        public void run() {
                            if (setEnabled(mc, name, true)) {
                                done.add(p.id);
                                status = "Installed and turned on " + p.title + ".";
                            } else {
                                status = "Downloaded " + name + ", but it couldn't be turned on: pick it in Resource Packs.";
                            }
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
            // Emoji aren't in the game's font: leave them out rather than show "??".
            String desc = p.description.replaceAll("[\\uD800-\\uDFFF\\u2600-\\u27BF\\uFE0F]", "").replaceAll("\\s+", " ").trim();
            desc = desc.length() > 62 ? desc.substring(0, 61) + "…" : desc;
            GlyphFont.draw(desc, x + 8, y + 16, 7, false, 0xFFA8ACB8);
            String label = busy.contains(p.id) ? "…" : done.contains(p.id) ? (hover ? "Turn off" : "On") : "Install";
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
