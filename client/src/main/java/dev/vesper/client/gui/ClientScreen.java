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
import org.lwjgl.input.Keyboard;
import org.lwjgl.input.Mouse;

/**
 * The client's settings: categories, search, every module with its real
 * options (toggles, sliders, colours, choices, keys), profiles and reset,
 * plus the HUD editor and the showroom. Changes save a second later.
 */
public class ClientScreen extends GuiScreen {
    private final GuiScreen parent;
    private Category category; // null = all
    private GuiTextField search;
    private float scroll, scrollTarget, maxScroll;
    private final Map<String, Float> expand = new HashMap<String, Float>();
    private final Map<String, Float> knob = new HashMap<String, Float>();
    private String open; // expanded module id
    private NumberSetting dragging;
    private float dragX, dragW;
    private KeySetting binding;
    private Module bindingModule;
    private ColorSetting picking;
    private float pickX, pickY, pickW, pickH, hueX, hueW;
    private int pickDrag; // 0 none, 1 square, 2 hue
    private float lastFrame, openedAt;

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

    // ---------- layout ----------

    private float u() {
        return height / 100f; // 1% of the height
    }

    private float panelX() {
        return Math.max(8, width * 0.08f);
    }

    private float panelY() {
        return 8 * u();
    }

    private float panelW() {
        return width - panelX() * 2;
    }

    private float panelH() {
        return height - panelY() * 2;
    }

    private float sideW() {
        return Math.max(90, panelW() * 0.2f);
    }

    private float fs(float pct) {
        return Math.max(6f, pct * u());
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
            mc.displayGuiScreen(parent);
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
        if (d != 0) scrollTarget = Math.max(0, Math.min(maxScroll, scrollTarget - Math.signum(d) * 8 * u()));
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
        float now = Ease.seconds();
        float dt = Math.min(0.1f, now - lastFrame);
        lastFrame = now;
        scroll = Ease.approach(scroll, scrollTarget, 18, dt);
        hits.clear();
        float in = Ease.outCubic(Ease.span(now, openedAt, openedAt + 0.22f));

        Gl.begin2D();
        Draw.rect(0, 0, width, height, 0xFF000000);
        MenuBackdrop.base(width, height, 1);
        MenuBackdrop.curtains(width, height, now, 0.35f);
        MenuBackdrop.stars(width, height, now, 0.5f);

        float px = panelX(), py = panelY() + (1 - in) * 2 * u(), pw = panelW(), ph = panelH();
        Draw.roundRect(px, py, pw, ph, 1.6f * u(), Colors.withAlpha(0x0E0F12, 0.92f * in));
        Draw.roundOutline(px, py, pw, ph, 1.6f * u(), 1, Colors.withAlpha(0xFFFFFF, 0.06f * in));

        // Side bar: logo, categories, links.
        float sx = px + 2.4f * u(), sy = py + 3 * u();
        Sigil.draw(sx, sy, 4 * u(), Colors.withAlpha(0xF2F3F5, in), 1);
        GlyphFont.draw("Vesper", sx + 5.2f * u(), sy + 0.5f * u(), fs(3f), true, Colors.withAlpha(0xF2F3F5, in));
        float cy = sy + 8 * u();
        cy = sideItem("All", category == null, sx, cy, mx, my, new Runnable() {
            public void run() {
                category = null;
                scrollTarget = 0;
            }
        });
        for (final Category c : Category.values()) {
            if (ModuleRegistry.find(c, null).isEmpty()) continue;
            cy = sideItem(c.label, category == c, sx, cy, mx, my, new Runnable() {
                public void run() {
                    category = c;
                    scrollTarget = 0;
                }
            });
        }
        cy += 2 * u();
        cy = sideItem("Edit HUD layout", false, sx, cy, mx, my, new Runnable() {
            public void run() {
                mc.displayGuiScreen(new HudEditorScreen(ClientScreen.this));
            }
        });
        cy = sideItem("Showroom", false, sx, cy, mx, my, new Runnable() {
            public void run() {
                mc.displayGuiScreen(new ShowroomScreen(ClientScreen.this));
            }
        });
        cy = sideItem("Resource packs", false, sx, cy, mx, my, new Runnable() {
            public void run() {
                mc.displayGuiScreen(new dev.vesper.client.packs.PackBrowserScreen(ClientScreen.this));
            }
        });

        // Profiles, bottom of the side bar.
        float by = py + ph - 13 * u();
        GlyphFont.draw("PROFILE", sx, by, fs(1.7f), true, Colors.withAlpha(0x7C808A, in));
        final List<String> profiles = VesperClient.config().profiles();
        final String active = VesperClient.config().activeProfile();
        float bw = sideW() - 4.8f * u();
        pill(sx, by + 2.4f * u(), bw, 3.6f * u(), active + "  ›", mx, my, new Runnable() {
            public void run() {
                int i = profiles.indexOf(active);
                VesperClient.config().switchProfile(profiles.get((i + 1) % profiles.size()));
            }
        });
        float half = (bw - u()) / 2;
        pill(sx, by + 6.8f * u(), half, 3.2f * u(), "New", mx, my, new Runnable() {
            public void run() {
                int n = profiles.size() + 1;
                while (profiles.contains("Profile " + n)) n++;
                VesperClient.config().switchProfile("Profile " + n);
            }
        });
        pill(sx + half + u(), by + 6.8f * u(), half, 3.2f * u(), "Reset", mx, my, new Runnable() {
            public void run() {
                VesperClient.config().resetProfile();
            }
        });

        // Content: header, search, module list.
        float cx = px + sideW(), cw = pw - sideW() - 2.4f * u();
        Draw.rect(cx - 1, py + 2 * u(), cx, py + ph - 2 * u(), Colors.withAlpha(0xFFFFFF, 0.05f));
        float hx = cx + 2.4f * u();
        GlyphFont.draw(category == null ? "All modules" : category.label, hx, py + 3 * u(), fs(3.4f), true, Colors.withAlpha(0xF2F3F5, in));
        // Search box.
        float sw = Math.min(32 * u(), cw * 0.4f), sbx = cx + cw - sw, sby = py + 2.6f * u(), sbh = 3.8f * u();
        Draw.roundRect(sbx, sby, sw, sbh, sbh / 2, search.isFocused() ? 0x26FFFFFF : 0x14FFFFFF);
        String q = search.getText();
        GlyphFont.draw(q.isEmpty() && !search.isFocused() ? "Search (Ctrl+F)" : q + (search.isFocused() && (now * 2) % 2 < 1 ? "|" : ""), sbx + 1.4f * u(), sby + (sbh - fs(1.9f) * 1.25f) / 2, fs(1.9f), false, q.isEmpty() && !search.isFocused() ? 0xFF7C808A : 0xFFF2F3F5);
        search.xPosition = (int) sbx;
        search.yPosition = (int) sby;
        search.width = (int) sw;
        search.height = (int) sbh;
        hits.add(new Hit(sbx, sby, sw, sbh, new Runnable() {
            public void run() {
                search.setFocused(true);
            }
        }));
        String err = VesperClient.config().lastError();
        float listTop = py + 9 * u();
        if (err != null) {
            GlyphFont.draw(err, hx, py + 7 * u(), fs(1.6f), false, 0xFFFFB86B);
            listTop += 2 * u();
        }

        List<Module> mods = ModuleRegistry.find(category, q.trim());
        float listBottom = py + ph - 4.5f * u();
        scissor(cx, listTop, cw + 2.4f * u(), listBottom - listTop);
        float y = listTop - scroll;
        for (Module m : mods) y = card(m, hx, y, cw - 2.4f * u(), mx, my, listTop, listBottom, dt) + 1.2f * u();
        if (mods.isEmpty()) GlyphFont.draw("Nothing matches “" + q + "”.", hx, listTop + 2 * u(), fs(2f), false, 0xFF7C808A);
        maxScroll = Math.max(0, y + scroll - listBottom + 2 * u());
        if (scrollTarget > maxScroll) scrollTarget = maxScroll;
        endScissor();

        // Colour picker popup (drawn last, on top).
        if (picking != null) drawPicker();

        GlyphFont.draw("Esc to go back · settings save automatically", hx, py + ph - 1.6f * u() - fs(1.5f), fs(1.5f), false, 0xFF5C606A);
        Gl.end2D();
    }

    private float sideItem(String label, boolean on, float x, float y, int mx, int my, Runnable r) {
        float w = sideW() - 4.8f * u(), h = 3.6f * u();
        boolean hov = mx >= x && mx < x + w && my >= y && my < y + h;
        if (on || hov) Draw.roundRect(x - u(), y, w + u(), h, u(), on ? Colors.withAlpha(TitleScreen.accent(), 0.16f) : 0x0FFFFFFF);
        if (on) Draw.rect(x - u(), y + h * 0.25f, x - u() + 2, y + h * 0.75f, 0xFF000000 | TitleScreen.accent());
        GlyphFont.draw(label, x + 0.6f * u(), y + (h - fs(2f) * 1.25f) / 2, fs(2f), on, on ? 0xFFFFFFFF : hov ? 0xFFE3E5EA : 0xFFA9ADB6);
        hits.add(new Hit(x - u(), y, w + u(), h, r));
        return y + h + 0.6f * u();
    }

    private void pill(float x, float y, float w, float h, String label, int mx, int my, Runnable r) {
        boolean hov = mx >= x && mx < x + w && my >= y && my < y + h;
        Draw.roundRect(x, y, w, h, h / 2, hov ? 0x33FFFFFF : 0x1FFFFFFF);
        GlyphFont.drawCentered(label, x + w / 2, y + (h - fs(1.8f) * 1.25f) / 2, fs(1.8f), false, 0xFFE3E5EA);
        hits.add(new Hit(x, y, w, h, r));
    }

    /** One module card; returns the y below it. */
    private float card(final Module m, float x, float y, float w, int mx, int my, float top, float bottom, float dt) {
        float head = 7 * u();
        float ex = Ease.approach(expand.containsKey(m.id) ? expand.get(m.id) : 0, m.id.equals(open) ? 1 : 0, 14, dt);
        expand.put(m.id, ex);
        float bodyH = ex > 0.001f ? settingsHeight(m) * Ease.outCubic(ex) : 0;
        float hgt = head + bodyH;
        boolean visible = y + hgt > top && y < bottom;
        if (visible) {
            boolean hov = mx >= x && mx < x + w && my >= y && my < y + head && my >= top && my < bottom;
            Draw.roundRect(x, y, w, hgt, 1.2f * u(), hov ? 0xFF1A1B20 : 0xFF15161A);
            GlyphFont.draw(m.name, x + 2 * u(), y + 1.3f * u(), fs(2.2f), true, 0xFFF2F3F5);
            String desc = m.description;
            float maxW = w - 16 * u();
            while (desc.length() > 4 && GlyphFont.width(desc, fs(1.7f), false) > maxW) desc = desc.substring(0, desc.length() - 2);
            if (!desc.equals(m.description)) desc = desc.trim() + "…";
            GlyphFont.draw(desc, x + 2 * u(), y + 4 * u(), fs(1.7f), false, 0xFF8C909A);
            if (!m.publicSafe) GlyphFont.draw("opt-in", x + 2.6f * u() + GlyphFont.width(m.name, fs(2.2f), true), y + 1.6f * u(), fs(1.5f), false, 0xFFFFB86B);
            // Toggle switch.
            float tw = 5.2f * u(), th = 2.8f * u(), tx = x + w - tw - 2 * u(), ty = y + (head - th) / 2;
            switchBox(m.id, tx, ty, tw, th, m.enabled(), dt);
            if (my >= top && my < bottom) {
                hits.add(new Hit(x, y, w - tw - 4 * u(), head, new Runnable() {
                    public void run() {
                        UiSound.hover();
                        open = m.id.equals(open) ? null : m.id;
                    }
                }));
                hits.add(new Hit(tx - u(), y, tw + 3 * u(), head, new Runnable() {
                    public void run() {
                        UiSound.press();
                        m.toggle();
                        changed();
                    }
                }));
            }
            if (bodyH > 0.5f) {
                scissor(x, Math.max(top, y + head), w, Math.min(bottom, y + hgt) - Math.max(top, y + head));
                drawSettings(m, x + 2 * u(), y + head, w - 4 * u(), mx, my, top, bottom);
                endScissor();
                scissor(panelX() + sideW(), top, panelW() - sideW(), bottom - top);
            }
        }
        return y + hgt;
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
        float rw = 12 * u();
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
