package dev.vesper.client.gui;

import dev.vesper.client.VesperClient;
import dev.vesper.client.hud.HudEditorScreen;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.KeySetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.module.setting.Setting;
import dev.vesper.client.preview.PlayerPreview;
import dev.vesper.client.preview.ShowroomScreen;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.gui.GuiTextField;
import net.minecraft.client.renderer.GlStateManager;
import org.lwjgl.input.Keyboard;
import org.lwjgl.input.Mouse;

/**
 * The client's settings, laid out like a launcher: you, live in 3D with
 * your cosmetics, beside the categories; every module as a tile you can
 * switch on and off; a drawer that slides in with a module's options
 * (toggles, sliders, colours, choices, keys); profiles, the HUD editor,
 * the showroom and resource packs. Motion is quick and eases out: the
 * panel opens and closes, tiles come in one after another, the category
 * marker slides, tiles answer hover and clicks, and switching something
 * shows a short note. Changes save a second later.
 */
public class ClientScreen extends GuiScreen {
    private final GuiScreen parent;
    private Category category; // null = all
    private GuiTextField search;
    private float scroll, scrollTarget, maxScroll;
    private float drawerScroll, drawerScrollTarget, drawerMax;
    private final Map<String, Float> knob = new HashMap<String, Float>();
    private final Map<String, Float> hover = new HashMap<String, Float>();
    private final Map<String, Float> pressedAt = new HashMap<String, Float>();
    private String open; // module in the drawer
    private String closingDrawer;
    private float drawerAt, drawerClosedAt;
    private NumberSetting dragging;
    private float dragX, dragW;
    private KeySetting binding;
    private Module bindingModule;
    private ColorSetting picking;
    private float pickX, pickY, pickW, pickH, hueX, hueW;
    private int pickDrag; // 0 none, 1 square, 2 hue
    private float lastFrame, openedAt, closingAt = -1;
    private String gridKey = "";
    private float gridAt;
    private float markerY = -1, markerH;
    private String toast;
    private boolean toastOn;
    private float toastAt = -10;
    private int mouseX, mouseY;

    /** Clickable regions from the last frame. */
    private static final class Hit {
        final float x, y, w, h;
        final Runnable action;

        Hit(float x, float y, float w, float h, Runnable action) {
            this.x = x;
            this.y = y;
            this.w = w;
            this.h = h;
            this.action = action;
        }

        boolean in(int mx, int my) {
            return mx >= x && mx < x + w && my >= y && my < y + h;
        }
    }

    private final List<Hit> hits = new ArrayList<Hit>();

    public ClientScreen(GuiScreen parent) {
        this.parent = parent;
    }

    @Override
    public void initGui() {
        Keyboard.enableRepeatEvents(true);
        String q = search == null ? "" : search.getText();
        search = new GuiTextField(0, fontRendererObj, 0, 0, 10, 10);
        search.setEnableBackgroundDrawing(false);
        search.setMaxStringLength(40);
        search.setText(q);
        lastFrame = Ease.seconds();
        if (openedAt == 0) openedAt = lastFrame;
    }

    @Override
    public void onGuiClosed() {
        Keyboard.enableRepeatEvents(false);
        VesperClient.config().save();
    }

    private void changed() {
        VesperClient.config().markDirty();
    }

    private void close() {
        if (closingAt < 0) closingAt = Ease.seconds();
    }

    private void openDrawer(String id) {
        if (id.equals(open)) return;
        if (open != null) {
            closingDrawer = open;
            drawerClosedAt = Ease.seconds();
        }
        open = id;
        drawerAt = Ease.seconds();
        drawerScroll = drawerScrollTarget = 0;
        picking = null;
    }

    private void closeDrawer() {
        if (open == null) return;
        closingDrawer = open;
        drawerClosedAt = Ease.seconds();
        open = null;
        picking = null;
    }

    private void note(String text, boolean on) {
        toast = text;
        toastOn = on;
        toastAt = Ease.seconds();
    }

    // ---------- layout ----------

    private float u() {
        return height / 100f; // 1% of the height
    }

    private float panelX() {
        return Math.max(8, width * 0.06f);
    }

    private float panelY() {
        return 7 * u();
    }

    private float panelW() {
        return width - panelX() * 2;
    }

    private float panelH() {
        return height - panelY() * 2;
    }

    private float sideW() {
        return Math.max(100, panelW() * 0.22f);
    }

    private float fs(float pct) {
        return Math.max(6f, pct * u());
    }

    private float drawerW() {
        return Math.min(Math.max(150, panelW() * 0.42f), panelW() - sideW() - 6 * u());
    }

    // ---------- input ----------

    @Override
    protected void keyTyped(char c, int key) {
        if (binding != null) {
            if (key == Keyboard.KEY_ESCAPE) {
                // cancel
            } else if (key == Keyboard.KEY_BACK || key == Keyboard.KEY_DELETE) {
                binding.set(0);
                changed();
            } else {
                binding.set(key);
                changed();
            }
            binding = null;
            bindingModule = null;
            return;
        }
        if (key == Keyboard.KEY_ESCAPE) {
            if (picking != null) {
                picking = null;
                return;
            }
            UiSound.press();
            if (open != null) {
                closeDrawer();
                return;
            }
            close();
            return;
        }
        if (search.textboxKeyTyped(c, key)) {
            scrollTarget = 0;
            return;
        }
        if (key == Keyboard.KEY_F && isCtrlKeyDown()) search.setFocused(true);
    }

    @Override
    protected void mouseClicked(int mx, int my, int button) {
        search.mouseClicked(mx, my, button);
        if (button != 0) return;
        if (picking != null) {
            if (mx >= pickX && mx < pickX + pickW && my >= pickY && my < pickY + pickH) {
                pickDrag = 1;
                pick(mx, my);
                return;
            }
            if (mx >= hueX && mx < hueX + hueW && my >= pickY && my < pickY + pickH) {
                pickDrag = 2;
                pick(mx, my);
                return;
            }
        }
        for (int i = hits.size() - 1; i >= 0; i--) {
            Hit h = hits.get(i);
            if (h.in(mx, my)) {
                h.action.run();
                return;
            }
        }
    }

    @Override
    protected void mouseClickMove(int mx, int my, int button, long held) {
        if (dragging != null) {
            dragging.set(dragging.min + (dragging.max - dragging.min) * Ease.clamp01((mx - dragX) / dragW));
            changed();
        }
        if (pickDrag != 0) pick(mx, my);
    }

    @Override
    protected void mouseReleased(int mx, int my, int state) {
        dragging = null;
        pickDrag = 0;
    }

    @Override
    public void handleMouseInput() throws java.io.IOException {
        super.handleMouseInput();
        int d = Mouse.getEventDWheel();
        if (d == 0) return;
        float step = Math.signum(d) * 8 * u();
        if (open != null && mouseX >= panelX() + panelW() - drawerW()) drawerScrollTarget = Math.max(0, Math.min(drawerMax, drawerScrollTarget - step));
        else scrollTarget = Math.max(0, Math.min(maxScroll, scrollTarget - step));
    }

    private void pick(int mx, int my) {
        if (picking == null) return;
        float[] hsv = hsv(picking.argb());
        if (pickDrag == 1) {
            hsv[1] = Ease.clamp01((mx - pickX) / pickW);
            hsv[2] = 1 - Ease.clamp01((my - pickY) / pickH);
        } else {
            hsv[0] = Ease.clamp01((my - pickY) / pickH);
        }
        int rgb = java.awt.Color.HSBtoRGB(hsv[0], hsv[1], hsv[2]) & 0xFFFFFF;
        picking.set((picking.argb() & 0xFF000000) | rgb);
        changed();
    }

    private static float[] hsv(int argb) {
        return java.awt.Color.RGBtoHSB((argb >> 16) & 255, (argb >> 8) & 255, argb & 255, null);
    }

    // ---------- drawing ----------

    @Override
    public void drawScreen(int mx, int my, float pt) {
        mouseX = mx;
        mouseY = my;
        float now = Ease.seconds();
        float dt = Math.min(0.1f, now - lastFrame);
        lastFrame = now;
        scroll = Ease.approach(scroll, scrollTarget, 18, dt);
        drawerScroll = Ease.approach(drawerScroll, drawerScrollTarget, 18, dt);
        hits.clear();
        // Opening: 280 ms, eased out. Closing: 160 ms, then back to the menu.
        float in = Ease.outQuint(Ease.span(now, openedAt, openedAt + 0.28f));
        if (closingAt >= 0) {
            float k = Ease.span(now, closingAt, closingAt + 0.16f);
            in = Math.min(in, 1 - Ease.outCubic(k));
            if (k >= 1) {
                mc.displayGuiScreen(parent);
                return;
            }
        }
        int accent = TitleScreen.accent();

        Gl.begin2D();
        Draw.rect(0, 0, width, height, 0xFF000000);
        MenuBackdrop.base(width, height, 1);
        MenuBackdrop.curtains(width, height, now, 0.35f);
        MenuBackdrop.stars(width, height, now, 0.5f);
        Draw.rect(0, 0, width, height, Colors.withAlpha(0x000000, 0.25f * in));

        float px = panelX(), py = panelY(), pw = panelW(), ph = panelH();
        // The panel grows from 97 % and fades in around its centre.
        GlStateManager.pushMatrix();
        float sc = 0.97f + 0.03f * in;
        GlStateManager.translate(px + pw / 2, py + ph / 2, 0);
        GlStateManager.scale(sc, sc, 1);
        GlStateManager.translate(-(px + pw / 2), -(py + ph / 2) + (1 - in) * 1.5f * u(), 0);
        // Soft shadow under the panel, then the panel.
        for (int i = 1; i <= 4; i++) Draw.roundRect(px - i * 0.6f * u(), py - i * 0.3f * u() + 1.2f * u(), pw + i * 1.2f * u(), ph + i * 0.9f * u(), 2.2f * u(), Colors.withAlpha(0x000000, 0.07f * in));
        Draw.roundRect(px, py, pw, ph, 1.8f * u(), Colors.withAlpha(0x0D0E11, 0.94f * in));
        Draw.roundOutline(px, py, pw, ph, 1.8f * u(), 1, Colors.withAlpha(0xFFFFFF, 0.07f * in));
        // Side bar.
        float sw = sideW();
        Draw.roundRect(px, py, sw, ph, 1.8f * u(), Colors.withAlpha(0x121318, 0.95f * in));
        drawSide(px, py, sw, ph, mx, my, now, dt, in, accent);

        // Content: title, count, search, tiles.
        float cx = px + sw + 2.6f * u(), cw = pw - sw - 5.2f * u();
        String title = category == null ? "All modules" : category.label;
        GlyphFont.draw(title, cx, py + 3 * u(), fs(3.6f), true, Colors.withAlpha(0xF2F3F5, in));
        String q = search.getText();
        List<Module> mods = ModuleRegistry.find(category, q.trim());
        int on = 0;
        for (Module m : mods) if (m.enabled()) on++;
        GlyphFont.draw(mods.size() + " modules · " + on + " on", cx, py + 7.4f * u(), fs(1.8f), false, Colors.withAlpha(0x8C909A, in));
        float sbw = Math.min(34 * u(), cw * 0.42f), sbx = cx + cw - sbw, sby = py + 3 * u(), sbh = 4.2f * u();
        float focus = Ease.approach(hover.containsKey("#search") ? hover.get("#search") : 0, search.isFocused() ? 1 : 0, 16, dt);
        hover.put("#search", focus);
        Draw.roundRect(sbx, sby, sbw, sbh, sbh / 2, Colors.mix(0x14FFFFFF, 0x24FFFFFF, focus));
        Draw.roundOutline(sbx, sby, sbw, sbh, sbh / 2, 1, Colors.withAlpha(accent, 0.55f * focus));
        Glyphs.search(sbx + 1.8f * u(), sby + sbh / 2, 0.9f * u(), Colors.mix(0xFF7C808A, 0xFFF2F3F5, focus));
        boolean empty = q.isEmpty() && !search.isFocused();
        GlyphFont.draw(empty ? "Search  (Ctrl+F)" : q + (search.isFocused() && (now * 2) % 2 < 1 ? "|" : ""), sbx + 3.4f * u(), sby + (sbh - fs(1.9f) * 1.25f) / 2, fs(1.9f), false, empty ? 0xFF7C808A : 0xFFF2F3F5);
        search.xPosition = (int) sbx;
        search.yPosition = (int) sby;
        search.width = (int) sbw;
        search.height = (int) sbh;
        hits.add(new Hit(sbx, sby, sbw, sbh, new Runnable() {
            public void run() {
                search.setFocused(true);
            }
        }));
        String err = VesperClient.config().lastError();
        float top = py + 11 * u();
        if (err != null) {
            GlyphFont.draw(err, cx, py + 9.4f * u(), fs(1.6f), false, 0xFFFFB86B);
            top += 2 * u();
        }
        float bottom = py + ph - 4.5f * u();

        // New filter: the tiles come in again, one after another.
        String key = (category == null ? "all" : category.name()) + "|" + q.trim().toLowerCase();
        if (!key.equals(gridKey)) {
            gridKey = key;
            gridAt = now;
        }
        int cols = Math.max(1, Math.min(3, (int) (cw / (40 * u()))));
        float gap = 1.4f * u(), tw = (cw - gap * (cols - 1)) / cols, th = 10.5f * u();
        scissor(cx - u(), top, cw + 2 * u(), bottom - top);
        float y0 = top - scroll;
        boolean gridLive = open == null && picking == null;
        for (int i = 0; i < mods.size(); i++) {
            int c = i % cols, r = i / cols;
            float tx = cx + c * (tw + gap), ty = y0 + r * (th + gap);
            if (ty + th < top || ty > bottom) continue;
            float appear = Ease.outCubic(Ease.span(now, gridAt + Math.min(i, 18) * 0.03f, gridAt + Math.min(i, 18) * 0.03f + 0.3f));
            tile(mods.get(i), tx, ty + (1 - appear) * 1.6f * u(), tw, th, mx, my, top, bottom, now, dt, appear * in, gridLive, accent);
        }
        int rows = (mods.size() + cols - 1) / cols;
        maxScroll = Math.max(0, rows * (th + gap) - (bottom - top) + u());
        if (scrollTarget > maxScroll) scrollTarget = maxScroll;
        endScissor();
        if (mods.isEmpty()) GlyphFont.draw("Nothing matches “" + q + "”.", cx, top + 2 * u(), fs(2f), false, 0xFF7C808A);
        // A fade at the bottom edge when there is more below.
        if (scroll < maxScroll - 1) Draw.gradientV(cx - u(), bottom - 4 * u(), cx + cw + u(), bottom, 0x000D0E11, Colors.withAlpha(0x0D0E11, in));
        GlyphFont.draw("Esc to go back · click a tile for its options · saved automatically", cx, py + ph - 1.6f * u() - fs(1.5f), fs(1.5f), false, Colors.withAlpha(0x5C606A, in));

        drawDrawer(px, py, pw, ph, mx, my, now, in, accent);
        if (picking != null) drawPicker();
        drawToast(px + sw + (pw - sw) / 2, py + ph, now, in, accent);
        GlStateManager.popMatrix();
        Gl.end2D();
    }

    private void drawSide(float px, float py, float sw, float ph, int mx, int my, float now, float dt, float in, int accent) {
        float sx = px + 2.4f * u(), sy = py + 2.8f * u(), iw = sw - 4.8f * u();
        Sigil.draw(sx, sy, 3.6f * u(), Colors.withAlpha(0xF2F3F5, in), 1);
        GlyphFont.draw("Vesper", sx + 4.8f * u(), sy + 0.3f * u(), fs(2.8f), true, Colors.withAlpha(0xF2F3F5, in));
        GlyphFont.draw(VesperClient.VERSION, sx + 4.8f * u() + GlyphFont.width("Vesper", fs(2.8f), true) + u(), sy + 1.1f * u(), fs(1.5f), false, Colors.withAlpha(0x7C808A, in));

        // You, live, with your cosmetics: turns slowly and leans toward the mouse.
        float cardY = sy + 5.5f * u(), cardH = Math.min(26 * u(), ph * 0.3f);
        Draw.roundRect(sx - u(), cardY, iw + 2 * u(), cardH, 1.4f * u(), Colors.withAlpha(0x0B0C0F, in));
        Draw.glow(sx + iw / 2, cardY + cardH * 0.6f, cardH * 0.7f, Colors.withAlpha(accent, (0.34f + 0.06f * (float) Math.sin(now * 1.6f)) * in));
        Draw.roundOutline(sx - u(), cardY, iw + 2 * u(), cardH, 1.4f * u(), 1, Colors.withAlpha(0xFFFFFF, 0.05f * in));
        float lean = Math.max(-1, Math.min(1, (mx - (sx + iw / 2)) / (width * 0.4f)));
        float yaw = 22 * (float) Math.sin(now * 0.45f) + lean * 30;
        int total = ModuleRegistry.all().size(), on = 0;
        for (Module m : ModuleRegistry.all()) if (m.enabled()) on++;
        GlyphFont.draw(mc.getSession().getUsername(), sx, cardY + cardH + 0.8f * u(), fs(2f), true, Colors.withAlpha(0xF2F3F5, in));
        GlyphFont.draw(on + " of " + total + " modules on", sx, cardY + cardH + 3.6f * u(), fs(1.6f), false, Colors.withAlpha(0x8C909A, in));

        // Categories, with a marker that slides to the chosen one.
        float cy = cardY + cardH + 7.6f * u();
        float itemH = 3.4f * u();
        float targetY = cy;
        List<Object> items = new ArrayList<Object>();
        items.add(null);
        for (Category c : Category.values()) if (!ModuleRegistry.find(c, null).isEmpty()) items.add(c);
        for (int i = 0; i < items.size(); i++) if (items.get(i) == category) targetY = cy + i * (itemH + 0.5f * u());
        markerY = markerY < 0 ? targetY : Ease.approach(markerY, targetY, 16, dt);
        Draw.roundRect(sx - u(), markerY, iw + u(), itemH, u(), Colors.withAlpha(accent, 0.16f * in));
        Draw.roundRect(sx - u(), markerY + itemH * 0.22f, 2, itemH * 0.56f, 1, Colors.withAlpha(accent, in));
        for (Object o : items) {
            final Category c = (Category) o;
            String label = c == null ? "All" : c.label;
            int count = ModuleRegistry.find(c, null).size();
            cy = sideItem(label, String.valueOf(count), category == c, sx, cy, iw, itemH, mx, my, dt, in, new Runnable() {
                public void run() {
                    UiSound.hover();
                    category = c;
                    scrollTarget = 0;
                    closeDrawer();
                }
            });
        }
        cy += 1.4f * u();
        Draw.rect(sx, cy, sx + iw, cy + 1, Colors.withAlpha(0xFFFFFF, 0.05f * in));
        cy += 1.4f * u();
        cy = sideItem("Edit HUD layout", "›", false, sx, cy, iw, itemH, mx, my, dt, in, new Runnable() {
            public void run() {
                mc.displayGuiScreen(new HudEditorScreen(ClientScreen.this));
            }
        });
        cy = sideItem("Showroom", "›", false, sx, cy, iw, itemH, mx, my, dt, in, new Runnable() {
            public void run() {
                mc.displayGuiScreen(new ShowroomScreen(ClientScreen.this));
            }
        });
        cy = sideItem("Resource packs", "›", false, sx, cy, iw, itemH, mx, my, dt, in, new Runnable() {
            public void run() {
                mc.displayGuiScreen(new dev.vesper.client.packs.PackBrowserScreen(ClientScreen.this));
            }
        });

        // Profiles at the bottom.
        float by = py + ph - 11.5f * u();
        if (by > cy + u()) {
            GlyphFont.draw("PROFILE", sx, by, fs(1.5f), true, Colors.withAlpha(0x7C808A, in));
            final List<String> profiles = VesperClient.config().profiles();
            final String active = VesperClient.config().activeProfile();
            pill(sx, by + 2.2f * u(), iw, 3.4f * u(), active + "  ›", mx, my, new Runnable() {
                public void run() {
                    int i = profiles.indexOf(active);
                    VesperClient.config().switchProfile(profiles.get((i + 1) % profiles.size()));
                    note("Profile: " + VesperClient.config().activeProfile(), true);
                }
            });
            float half = (iw - u()) / 2;
            pill(sx, by + 6.4f * u(), half, 3f * u(), "New", mx, my, new Runnable() {
                public void run() {
                    int n = profiles.size() + 1;
                    while (profiles.contains("Profile " + n)) n++;
                    VesperClient.config().switchProfile("Profile " + n);
                    note("New profile: Profile " + n, true);
                }
            });
            pill(sx + half + u(), by + 6.4f * u(), half, 3f * u(), "Reset", mx, my, new Runnable() {
                public void run() {
                    VesperClient.config().resetProfile();
                    note("Profile reset", false);
                }
            });
        }

        // The 3D preview last in the side bar (it needs depth; the rest is flat).
        Gl.end2D();
        scissor(sx - u(), cardY, iw + 2 * u(), cardH);
        PlayerPreview.draw(sx + iw / 2, cardY + cardH - 1.6f * u(), cardH * 0.74f, yaw, in, now);
        endScissor();
        Gl.begin2D();
    }

    private float sideItem(String label, String right, boolean on, float x, float y, float w, float h, int mx, int my, float dt, float in, Runnable r) {
        boolean hov = mx >= x - u() && mx < x + w && my >= y && my < y + h && open == null;
        String k = "#side" + label;
        float t = Ease.approach(hover.containsKey(k) ? hover.get(k) : 0, hov ? 1 : 0, 18, dt);
        hover.put(k, t);
        if (!on && t > 0.01f) Draw.roundRect(x - u(), y, w + u(), h, u(), Colors.withAlpha(0xFFFFFF, 0.05f * t * in));
        float shift = on ? 0.5f * u() : t * 0.3f * u();
        int col = on ? 0xFFFFFFFF : Colors.mix(0xFFA9ADB6, 0xFFE3E5EA, t);
        GlyphFont.draw(label, x + 0.6f * u() + shift, y + (h - fs(1.9f) * 1.25f) / 2, fs(1.9f), on, Colors.alpha(col, in));
        GlyphFont.draw(right, x + w - GlyphFont.width(right, fs(1.6f), false) - 0.6f * u(), y + (h - fs(1.6f) * 1.25f) / 2, fs(1.6f), false, Colors.withAlpha(0x6C707A, in));
        hits.add(new Hit(x - u(), y, w + u(), h, r));
        return y + h + 0.5f * u();
    }

    private void pill(float x, float y, float w, float h, String label, int mx, int my, Runnable r) {
        boolean hov = mx >= x && mx < x + w && my >= y && my < y + h;
        Draw.roundRect(x, y, w, h, h / 2, hov ? 0x33FFFFFF : 0x1FFFFFFF);
        GlyphFont.drawCentered(label, x + w / 2, y + (h - fs(1.8f) * 1.25f) / 2, fs(1.8f), false, 0xFFE3E5EA);
        hits.add(new Hit(x, y, w, h, r));
    }

    /** One module tile: name, what it does, its switch; hover lifts it, a click presses it. */
    private void tile(final Module m, float x, float y, float w, float h, int mx, int my, float top, float bottom, float now, float dt, float alpha, boolean live, int accent) {
        boolean hov = live && mx >= x && mx < x + w && my >= y && my < y + h && my >= top && my < bottom;
        float t = Ease.approach(hover.containsKey(m.id) ? hover.get(m.id) : 0, hov ? 1 : 0, 16, dt);
        hover.put(m.id, t);
        float press = pressedAt.containsKey(m.id) ? Ease.bell(Ease.span(now, pressedAt.get(m.id), pressedAt.get(m.id) + 0.18f)) : 0;
        float sel = m.id.equals(open) ? 1 : 0;
        float k = knob.containsKey(m.id) ? knob.get(m.id) : m.enabled() ? 1 : 0;
        GlStateManager.pushMatrix();
        float s = 1 - 0.025f * press;
        GlStateManager.translate(x + w / 2, y + h / 2, 0);
        GlStateManager.scale(s, s, 1);
        GlStateManager.translate(-(x + w / 2), -(y + h / 2) - t * 0.25f * u(), 0);
        // Lift shadow on hover.
        if (t > 0.01f) Draw.roundRect(x + 0.3f * u(), y + 0.8f * u(), w - 0.6f * u(), h, 1.4f * u(), Colors.withAlpha(0x000000, 0.35f * t * alpha));
        Draw.roundRect(x, y, w, h, 1.4f * u(), Colors.alpha(Colors.mix(0xFF15161B, 0xFF1C1D23, t), alpha));
        // On: a wash of the accent from the top-left.
        if (k > 0.01f) Draw.gradientV(x + 1, y + 1, x + w - 1, y + h * 0.6f, Colors.withAlpha(accent, 0.10f * k * alpha), Colors.withAlpha(accent, 0));
        Draw.roundOutline(x, y, w, h, 1.4f * u(), 1, Colors.alpha(Colors.mix(0x10FFFFFF, 0xFF000000 | accent, Math.max(sel, 0.45f * t)), alpha * Math.max(0.6f, Math.max(sel, t))));
        // Status dot.
        float dx = x + 2 * u(), dy = y + 2.5f * u();
        if (k > 0.01f) Draw.glow(dx, dy, 1.6f * u(), Colors.withAlpha(accent, 0.5f * k * alpha));
        Draw.circle(dx, dy, 0.5f * u(), Colors.alpha(Colors.mix(0xFF4A4D57, 0xFF000000 | accent, k), alpha));
        GlyphFont.draw(m.name, x + 3.4f * u(), y + 1.3f * u(), fs(2.1f), true, Colors.alpha(0xFFF2F3F5, alpha));
        // Description: up to two lines.
        List<String> lines = wrap(m.description, w - 4 * u(), fs(1.6f), 2);
        for (int i = 0; i < lines.size(); i++) GlyphFont.draw(lines.get(i), x + 2 * u(), y + 4.6f * u() + i * 2.2f * u(), fs(1.6f), false, Colors.alpha(0xFF8C909A, alpha));
        if (!m.publicSafe) GlyphFont.draw("opt-in", x + 3.8f * u() + GlyphFont.width(m.name, fs(2.1f), true), y + 1.6f * u(), fs(1.4f), false, Colors.alpha(0xFFFFB86B, alpha));
        float swW = 4.8f * u(), swH = 2.6f * u(), swX = x + w - swW - 1.6f * u(), swY = y + 1.2f * u();
        switchBox(m.id, swX, swY, swW, swH, m.enabled(), dt);
        GlStateManager.popMatrix();
        if (live && my >= top && my < bottom && alpha > 0.5f) {
            hits.add(new Hit(x, y, w, h, new Runnable() {
                public void run() {
                    UiSound.hover();
                    pressedAt.put(m.id, Ease.seconds());
                    openDrawer(m.id);
                }
            }));
            hits.add(new Hit(swX - u(), y, swW + 2.6f * u(), swY + swH + u() - y, new Runnable() {
                public void run() {
                    UiSound.press();
                    pressedAt.put(m.id, Ease.seconds());
                    m.toggle();
                    changed();
                    note(m.name + (m.enabled() ? " on" : " off"), m.enabled());
                }
            }));
        }
    }

    /** The options drawer: slides in from the right over a dimmed grid. */
    private void drawDrawer(float px, float py, float pw, float ph, int mx, int my, float now, float in, int accent) {
        String id = open != null ? open : closingDrawer;
        if (id == null) return;
        float k = open != null ? Ease.outQuint(Ease.span(now, drawerAt, drawerAt + 0.26f)) : 1 - Ease.outCubic(Ease.span(now, drawerClosedAt, drawerClosedAt + 0.18f));
        if (open == null && k <= 0) {
            closingDrawer = null;
            return;
        }
        final Module m = ModuleRegistry.get(id);
        if (m == null) return;
        float sw = sideW();
        Draw.roundRect(px + sw, py, pw - sw, ph, 1.8f * u(), Colors.withAlpha(0x000000, 0.45f * k * in));
        if (open != null) hits.add(new Hit(px + sw, py, pw - sw - drawerW(), ph, new Runnable() {
            public void run() {
                closeDrawer();
            }
        }));
        float dw = drawerW(), dx = px + pw - dw * k, dy = py;
        Draw.rect(dx - 2 * u(), dy + 1.8f * u(), dx, dy + ph - 1.8f * u(), Colors.withAlpha(0x000000, 0.25f * k));
        Draw.roundRect(dx, dy, dw, ph, 1.8f * u(), Colors.withAlpha(0x111217, in));
        Draw.rect(dx, dy + 1.8f * u(), dx + 1, dy + ph - 1.8f * u(), Colors.withAlpha(0xFFFFFF, 0.07f * in));
        float ix = dx + 2.6f * u(), iw = dw - 5.2f * u();
        float hy = dy + 2.8f * u();
        GlyphFont.draw(m.category.label.toUpperCase(), ix, hy, fs(1.5f), true, Colors.withAlpha(accent, in));
        GlyphFont.draw(m.name, ix, hy + 2.4f * u(), fs(3.2f), true, Colors.withAlpha(0xF2F3F5, in));
        // Close button.
        float cbs = 3.4f * u(), cbx = dx + dw - cbs - 1.8f * u();
        boolean ch = open != null && mx >= cbx && mx < cbx + cbs && my >= hy && my < hy + cbs;
        Draw.roundRect(cbx, hy, cbs, cbs, cbs / 2, ch ? 0x33FFFFFF : 0x14FFFFFF);
        Draw.line(cbx + cbs * 0.33f, hy + cbs * 0.33f, cbx + cbs * 0.67f, hy + cbs * 0.67f, 1.2f, 0xFFE3E5EA);
        Draw.line(cbx + cbs * 0.67f, hy + cbs * 0.33f, cbx + cbs * 0.33f, hy + cbs * 0.67f, 1.2f, 0xFFE3E5EA);
        if (open != null) hits.add(new Hit(cbx, hy, cbs, cbs, new Runnable() {
            public void run() {
                UiSound.press();
                closeDrawer();
            }
        }));
        List<String> lines = wrap(m.description, iw, fs(1.8f), 3);
        float ly = hy + 7.2f * u();
        for (String l : lines) {
            GlyphFont.draw(l, ix, ly, fs(1.8f), false, Colors.withAlpha(0xA9ADB6, in));
            ly += 2.5f * u();
        }
        // The big switch.
        float rowY = ly + u();
        Draw.roundRect(ix, rowY, iw, 5.4f * u(), 1.2f * u(), 0x0CFFFFFF);
        GlyphFont.draw(m.enabled() ? "On" : "Off", ix + 1.6f * u(), rowY + (5.4f * u() - fs(2.1f) * 1.25f) / 2, fs(2.1f), true, m.enabled() ? 0xFFFFFFFF : 0xFFA9ADB6);
        switchBox(m.id, ix + iw - 6.2f * u() - 1.4f * u(), rowY + 1.3f * u(), 6.2f * u(), 2.8f * u(), m.enabled(), 0.016f);
        if (open != null) hits.add(new Hit(ix, rowY, iw, 5.4f * u(), new Runnable() {
            public void run() {
                UiSound.press();
                m.toggle();
                changed();
                note(m.name + (m.enabled() ? " on" : " off"), m.enabled());
            }
        }));
        float top = rowY + 7 * u(), bottom = dy + ph - 2 * u();
        scissor(dx, top, dw, bottom - top);
        float body = settingsHeight(m);
        drawerMax = Math.max(0, body - (bottom - top) + u());
        if (drawerScrollTarget > drawerMax) drawerScrollTarget = drawerMax;
        int smx = open != null ? mx : -9999;
        drawSettings(m, ix, top - drawerScroll, iw, smx, my, top, bottom);
        endScissor();
        if (drawerScroll < drawerMax - 1) Draw.gradientV(dx, bottom - 4 * u(), dx + dw, bottom, 0x00111217, 0xFF111217);
    }

    private void drawToast(float cx, float bottomY, float now, float in, int accent) {
        if (toast == null) return;
        float t = now - toastAt;
        if (t > 1.9f) return;
        float a = Ease.outCubic(Ease.span(now, toastAt, toastAt + 0.22f)) * (1 - Ease.span(now, toastAt + 1.5f, toastAt + 1.9f));
        float w = GlyphFont.width(toast, fs(1.9f), true) + 6 * u(), h = 4.2f * u();
        float x = cx - w / 2, y = bottomY - 8 * u() + (1 - a) * 1.6f * u();
        Draw.roundRect(x, y + 0.6f * u(), w, h, h / 2, Colors.withAlpha(0x000000, 0.4f * a * in));
        Draw.roundRect(x, y, w, h, h / 2, Colors.withAlpha(0x1C1D23, a * in));
        Draw.roundOutline(x, y, w, h, h / 2, 1, Colors.withAlpha(0xFFFFFF, 0.08f * a * in));
        Draw.circle(x + 2 * u(), y + h / 2, 0.55f * u(), toastOn ? Colors.withAlpha(accent, a * in) : Colors.withAlpha(0x6C707A, a * in));
        GlyphFont.draw(toast, x + 3.6f * u(), y + (h - fs(1.9f) * 1.25f) / 2, fs(1.9f), true, Colors.withAlpha(0xF2F3F5, a * in));
    }

    private static List<String> wrap(String text, float maxW, float size, int maxLines) {
        List<String> out = new ArrayList<String>();
        String[] words = text.split(" ");
        StringBuilder line = new StringBuilder();
        for (int i = 0; i < words.length; i++) {
            String tryLine = line.length() == 0 ? words[i] : line + " " + words[i];
            if (GlyphFont.width(tryLine, size, false) <= maxW) {
                line = new StringBuilder(tryLine);
                continue;
            }
            if (line.length() > 0) out.add(line.toString());
            line = new StringBuilder(words[i]);
            if (out.size() == maxLines) break;
        }
        if (out.size() < maxLines && line.length() > 0) out.add(line.toString());
        if (out.size() > maxLines) out = out.subList(0, maxLines);
        // Mark a cut with an ellipsis.
        String joined = String.join(" ", out);
        if (joined.length() < text.length() && !out.isEmpty()) {
            String last = out.get(out.size() - 1);
            while (last.length() > 3 && GlyphFont.width(last + "…", size, false) > maxW) last = last.substring(0, last.length() - 1);
            out.set(out.size() - 1, last.trim() + "…");
        }
        return out;
    }

    private void switchBox(String key, float x, float y, float w, float h, boolean on, float dt) {
        float k = Ease.approach(knob.containsKey(key) ? knob.get(key) : on ? 1 : 0, on ? 1 : 0, 20, dt);
        knob.put(key, k);
        Draw.roundRect(x, y, w, h, h / 2, Colors.mix(0xFF2A2C33, 0xFF000000 | TitleScreen.accent(), k));
        float r = h / 2 - 1.5f;
        Draw.circle(x + h / 2 + (w - h) * k, y + h / 2, r, 0xFFF2F3F5);
    }

    private float rowH() {
        return 5 * u();
    }

    private float settingsHeight(Module m) {
        float h = 1.2f * u();
        for (Setting<?> s : m.settings()) if (s.visible()) h += rowH();
        h += rowH(); // toggle key
        h += rowH(); // reset
        return h;
    }

    private void drawSettings(final Module m, float x, float y, float w, int mx, int my, float top, float bottom) {
        Draw.rect(x, y, x + w, y + 1, 0x10FFFFFF);
        y += 1.2f * u();
        boolean clickable = true;
        for (final Setting<?> s : m.settings()) {
            if (!s.visible()) continue;
            boolean ok = clickable && y + rowH() > top && y < bottom;
            GlyphFont.draw(s.name, x, y + (rowH() - fs(1.9f) * 1.25f) / 2, fs(1.9f), false, 0xFFC9CCD3);
            float cx = x + w * 0.5f, cw = w * 0.5f;
            float ry = y + (rowH() - 2.8f * u()) / 2;
            if (s instanceof BoolSetting) {
                final BoolSetting b = (BoolSetting) s;
                float tw = 5.2f * u(), th = 2.8f * u();
                switchBox(m.id + "." + s.id, x + w - tw, ry, tw, th, b.on(), 0.016f);
                if (ok) hits.add(new Hit(x, y, w, rowH(), new Runnable() {
                    public void run() {
                        UiSound.press();
                        b.toggle();
                        changed();
                    }
                }));
            } else if (s instanceof NumberSetting) {
                final NumberSetting n = (NumberSetting) s;
                final float tx = cx, tw = cw - 8 * u();
                float f = (float) ((n.get() - n.min) / (n.max - n.min));
                float ty = y + rowH() / 2;
                Draw.roundRect(tx, ty - 0.35f * u(), tw, 0.7f * u(), 0.35f * u(), 0xFF2A2C33);
                Draw.roundRect(tx, ty - 0.35f * u(), tw * f, 0.7f * u(), 0.35f * u(), 0xFF000000 | TitleScreen.accent());
                Draw.circle(tx + tw * f, ty, 1.1f * u(), 0xFFF2F3F5);
                String v = n.display();
                GlyphFont.draw(v, x + w - GlyphFont.width(v, fs(1.8f), false), y + (rowH() - fs(1.8f) * 1.25f) / 2, fs(1.8f), false, 0xFFF2F3F5);
                if (ok) hits.add(new Hit(tx - u(), y, tw + 2 * u(), rowH(), new Runnable() {
                    public void run() {
                        dragging = n;
                        dragX = tx;
                        dragW = tw;
                        n.set(n.min + (n.max - n.min) * Ease.clamp01((Mouse.getX() * width / (float) mc.displayWidth - tx) / tw));
                        changed();
                    }
                }));
            } else if (s instanceof ColorSetting) {
                final ColorSetting c = (ColorSetting) s;
                final float sw = 6 * u(), sh = 2.8f * u(), swx = x + w - sw;
                Draw.roundRect(swx, ry, sw, sh, u(), c.argb() | 0xFF000000);
                Draw.roundOutline(swx, ry, sw, sh, u(), 1, 0x30FFFFFF);
                final float fy = ry;
                if (ok) hits.add(new Hit(x, y, w, rowH(), new Runnable() {
                    public void run() {
                        UiSound.press();
                        picking = picking == c ? null : c;
                        pickW = 24 * u();
                        pickH = 16 * u();
                        pickX = Math.min(swx + sw - pickW - 5 * u(), width - pickW - 6 * u());
                        pickY = Math.min(fy + sh + u(), height - pickH - 4 * u());
                        hueX = pickX + pickW + 1.5f * u();
                        hueW = 2.6f * u();
                    }
                }));
            } else if (s instanceof ChoiceSetting) {
                final ChoiceSetting ch = (ChoiceSetting) s;
                String lab = "‹  " + ch.label() + "  ›";
                float lw = GlyphFont.width(lab, fs(1.8f), false) + 3 * u();
                float lx = x + w - lw;
                Draw.roundRect(lx, ry, lw, 2.8f * u(), 1.4f * u(), 0x1FFFFFFF);
                GlyphFont.drawCentered(lab, lx + lw / 2, ry + (2.8f * u() - fs(1.8f) * 1.25f) / 2, fs(1.8f), false, 0xFFF2F3F5);
                final float mid = lx + lw / 2;
                if (ok) hits.add(new Hit(lx, y, lw, rowH(), new Runnable() {
                    public void run() {
                        UiSound.press();
                        int mxNow = Mouse.getX() * width / mc.displayWidth;
                        ch.cycle(mxNow < mid ? -1 : 1);
                        changed();
                    }
                }));
            } else if (s instanceof KeySetting) {
                keyRow((KeySetting) s, m, x, y, w, ry, ok);
            }
            y += rowH();
        }
        GlyphFont.draw("Toggle key", x, y + (rowH() - fs(1.9f) * 1.25f) / 2, fs(1.9f), false, 0xFFC9CCD3);
        keyRow(m.toggleKey, m, x, y, w, y + (rowH() - 2.8f * u()) / 2, y + rowH() > top && y < bottom);
        y += rowH();
        float rw = GlyphFont.width("Reset module", fs(1.8f), false) + 4 * u();
        pill(x + w - rw, y + (rowH() - 2.8f * u()) / 2, rw, 2.8f * u(), "Reset module", mx, my, new Runnable() {
            public void run() {
                m.resetAll();
                changed();
            }
        });
    }

    private void keyRow(final KeySetting k, final Module m, float x, float y, float w, float ry, boolean ok) {
        String lab = binding == k ? "Press a key…" : k.keyName();
        float lw = Math.max(8 * u(), GlyphFont.width(lab, fs(1.8f), false) + 3 * u());
        float lx = x + w - lw;
        Draw.roundRect(lx, ry, lw, 2.8f * u(), 1.4f * u(), binding == k ? Colors.withAlpha(TitleScreen.accent(), 0.3f) : 0x1FFFFFFF);
        GlyphFont.drawCentered(lab, lx + lw / 2, ry + (2.8f * u() - fs(1.8f) * 1.25f) / 2, fs(1.8f), false, 0xFFF2F3F5);
        if (ok) hits.add(new Hit(lx, y, lw, rowH(), new Runnable() {
            public void run() {
                UiSound.press();
                binding = k;
                bindingModule = m;
            }
        }));
    }

    private void drawPicker() {
        float[] hsv = hsv(picking.argb());
        float pad = 1.2f * u();
        Draw.roundRect(pickX - pad, pickY - pad, pickW + hueW + 1.5f * u() + pad * 2, pickH + pad * 2 + 3 * u(), u(), 0xF0101114);
        Draw.roundOutline(pickX - pad, pickY - pad, pickW + hueW + 1.5f * u() + pad * 2, pickH + pad * 2 + 3 * u(), u(), 1, 0x20FFFFFF);
        int pure = java.awt.Color.HSBtoRGB(hsv[0], 1, 1) | 0xFF000000;
        Draw.gradientH(pickX, pickY, pickX + pickW, pickY + pickH, 0xFFFFFFFF, pure);
        Draw.gradientV(pickX, pickY, pickX + pickW, pickY + pickH, 0x00000000, 0xFF000000);
        int steps = 12;
        for (int i = 0; i < steps; i++) {
            int a = java.awt.Color.HSBtoRGB(i / (float) steps, 1, 1) | 0xFF000000, b = java.awt.Color.HSBtoRGB((i + 1) / (float) steps, 1, 1) | 0xFF000000;
            Draw.gradientV(hueX, pickY + pickH * i / steps, hueX + hueW, pickY + pickH * (i + 1) / steps, a, b);
        }
        Draw.ring(pickX + hsv[1] * pickW, pickY + (1 - hsv[2]) * pickH, 0.9f * u(), 1.3f * u(), 0xFFFFFFFF, 0xFFFFFFFF);
        Draw.rect(hueX - 1, pickY + hsv[0] * pickH - 1, hueX + hueW + 1, pickY + hsv[0] * pickH + 1, 0xFFFFFFFF);
        GlyphFont.draw(String.format("#%06X", picking.rgb()), pickX, pickY + pickH + 0.8f * u(), fs(1.7f), false, 0xFFC9CCD3);
    }

    private void scissor(float x, float y, float w, float h) {
        if (w <= 0 || h <= 0) {
            w = 0;
            h = 0;
        }
        float k = mc.displayWidth / (float) width;
        org.lwjgl.opengl.GL11.glEnable(org.lwjgl.opengl.GL11.GL_SCISSOR_TEST);
        org.lwjgl.opengl.GL11.glScissor((int) (x * k), (int) (mc.displayHeight - (y + h) * k), (int) Math.max(0, w * k), (int) Math.max(0, h * k));
    }

    private void endScissor() {
        org.lwjgl.opengl.GL11.glDisable(org.lwjgl.opengl.GL11.GL_SCISSOR_TEST);
    }

    @Override
    public boolean doesGuiPauseGame() {
        return true;
    }
}
