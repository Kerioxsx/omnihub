package dev.vesper.client.hud;

import dev.vesper.client.VesperClient;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import net.minecraft.client.gui.GuiScreen;
import org.lwjgl.input.Mouse;

/**
 * Drag HUD elements into place, scroll over one to scale it, right-click
 * to hide or show it. Elements snap to the screen edges and centre (hold
 * Shift to place freely). Elements without live data show sample content.
 * Positions are saved as fractions of the screen, so they survive any
 * resolution or GUI scale.
 */
public class HudEditorScreen extends GuiScreen {
    private final GuiScreen parent;
    private HudElement dragging, hover;
    private float grabX, grabY;
    private float snapX = -1, snapY = -1;

    public HudEditorScreen(GuiScreen parent) {
        this.parent = parent;
    }

    private static final float SNAP = 4;

    private float[] rect(HudElement el) {
        HudLayout.Place p = HudLayout.get(el.id);
        el.prepare(p.scale);
        el.refresh(!el.hasContent());
        el.refresh(!el.hasContent());
        float w = el.width() * p.scale, h = el.height() * p.scale;
        return new float[] {Hud.screenX(p, w, width), Hud.screenY(p, h, height), w, h};
    }

    private HudElement at(int mx, int my) {
        for (int i = Hud.elements().size() - 1; i >= 0; i--) {
            HudElement el = Hud.elements().get(i);
            if (!el.enabled()) continue;
            float[] r = rect(el);
            if (mx >= r[0] && mx < r[0] + r[2] && my >= r[1] && my < r[1] + r[3]) return el;
        }
        return null;
    }

    @Override
    protected void keyTyped(char c, int key) {
        if (key == 1) {
            VesperClient.config().save();
            mc.displayGuiScreen(parent);
        }
    }

    private boolean onButton(int mx, int my, float x) {
        float y = height - 26, w = 70, h = 16;
        return mx >= x && mx < x + w && my >= y && my < y + h;
    }

    @Override
    protected void mouseClicked(int mx, int my, int button) {
        float bx = width / 2f - 75;
        if (button == 0 && onButton(mx, my, bx)) {
            for (HudElement el : Hud.elements()) HudLayout.reset(el.id);
            VesperClient.config().markDirty();
            return;
        }
        if (button == 0 && onButton(mx, my, bx + 80)) {
            VesperClient.config().save();
            mc.displayGuiScreen(parent);
            return;
        }
        HudElement el = at(mx, my);
        if (el == null) return;
        if (button == 1) {
            HudLayout.Place p = HudLayout.get(el.id);
            p.shown = !p.shown;
            VesperClient.config().markDirty();
            return;
        }
        if (button == 0) {
            float[] r = rect(el);
            dragging = el;
            grabX = mx - r[0];
            grabY = my - r[1];
        }
    }

    @Override
    protected void mouseClickMove(int mx, int my, int button, long held) {
        if (dragging == null) return;
        float[] r = rect(dragging);
        float x = mx - grabX, y = my - grabY, w = r[2], h = r[3];
        snapX = -1;
        snapY = -1;
        if (!isShiftKeyDown()) {
            if (Math.abs(x) < SNAP) x = snapX = 0;
            else if (Math.abs(x + w - width) < SNAP) {
                x = width - w;
                snapX = width;
            } else if (Math.abs(x + w / 2 - width / 2f) < SNAP) {
                x = width / 2f - w / 2;
                snapX = width / 2f;
            }
            if (Math.abs(y) < SNAP) y = snapY = 0;
            else if (Math.abs(y + h - height) < SNAP) {
                y = height - h;
                snapY = height;
            } else if (Math.abs(y + h / 2 - height / 2f) < SNAP) {
                y = height / 2f - h / 2;
                snapY = height / 2f;
            }
        }
        HudLayout.Place p = HudLayout.get(dragging.id);
        p.x = width - w <= 0 ? 0 : Math.max(0, Math.min(1, x / (width - w)));
        p.y = height - h <= 0 ? 0 : Math.max(0, Math.min(1, y / (height - h)));
        VesperClient.config().markDirty();
    }

    @Override
    protected void mouseReleased(int mx, int my, int state) {
        dragging = null;
        snapX = snapY = -1;
    }

    @Override
    public void handleMouseInput() throws java.io.IOException {
        super.handleMouseInput();
        int d = Mouse.getEventDWheel();
        if (d == 0 || hover == null) return;
        HudLayout.Place p = HudLayout.get(hover.id);
        p.scale = Math.max(0.5f, Math.min(3f, Math.round((p.scale + Math.signum(d) * 0.1f) * 10) / 10f));
        VesperClient.config().markDirty();
    }

    @Override
    public void drawScreen(int mx, int my, float pt) {
        hover = dragging != null ? dragging : at(mx, my);
        Gl.begin2D();
        // Dim the game a little and show the centre lines faintly.
        Draw.rect(0, 0, width, height, mc.theWorld == null ? 0xFF15161B : 0x55000000);
        Draw.rect(width / 2f - 0.5f, 0, width / 2f + 0.5f, height, 0x14FFFFFF);
        Draw.rect(0, height / 2f - 0.5f, width, height / 2f + 0.5f, 0x14FFFFFF);
        if (snapX >= 0) Draw.rect(snapX - 0.5f, 0, snapX + 0.5f, height, 0x806FB7FF);
        if (snapY >= 0) Draw.rect(0, snapY - 0.5f, width, snapY + 0.5f, 0x806FB7FF);
        for (HudElement el : Hud.elements()) {
            if (!el.enabled()) continue;
            HudLayout.Place p = HudLayout.get(el.id);
            float[] r = rect(el);
            boolean sample = !el.hasContent();
            el.renderAt(r[0], r[1], p.scale, sample, p.shown ? 1f : 0.3f);
            int edge = el == hover ? 0xFF6FB7FF : 0x40FFFFFF;
            Draw.roundOutline(r[0] - 2, r[1] - 2, r[2] + 4, r[3] + 4, 3, 1, edge);
            if (el == hover) {
                String tip = el.name + " · " + Math.round(p.scale * 100) + "%" + (p.shown ? "" : " · hidden");
                float ty = r[1] + r[3] + 4 > height - 40 ? r[1] - 12 : r[1] + r[3] + 4;
                GlyphFont.draw(tip, r[0], ty, 7, false, 0xFFE4E5EA);
            }
        }
        // Buttons and help.
        float bx = width / 2f - 75;
        button(bx, "Reset layout", mx, my);
        button(bx + 80, "Done", mx, my);
        String help = "Drag to move · scroll to resize · right-click to hide · Shift to place freely · Esc to finish";
        GlyphFont.drawCentered(help, width / 2f, height - 40, 6.5f, false, 0xFF9A9DA6);
        if (Hud.elements().isEmpty() || allOff()) GlyphFont.drawCentered("Turn HUD elements on in Client → HUD first.", width / 2f, height / 2f - 20, 8, false, 0xFFE4E5EA);
        Gl.end2D();
    }

    private boolean allOff() {
        for (HudElement el : Hud.elements()) if (el.enabled()) return false;
        return true;
    }

    private void button(float x, String label, int mx, int my) {
        boolean hov = onButton(mx, my, x);
        Draw.roundRect(x, height - 26, 70, 16, 8, hov ? Colors.withAlpha(0x6FB7FF, 0.45f) : 0x33FFFFFF);
        GlyphFont.drawCentered(label, x + 35, height - 26 + (16 - 7 * 1.25f) / 2, 7, false, 0xFFFFFFFF);
    }

    @Override
    public boolean doesGuiPauseGame() {
        return false;
    }
}
