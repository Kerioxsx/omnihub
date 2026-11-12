package dev.vesper.client.hud;

import dev.vesper.client.module.General;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;
import net.minecraft.client.settings.GameSettings;
import net.minecraft.client.settings.KeyBinding;
import org.lwjgl.input.Keyboard;

/** Movement keys, mouse buttons and jump as you press them (with a short press animation). */
final class KeystrokesElement extends HudElement {
    private static final float K = 18, G = 2, MOUSE_H = 16, SPACE_H = 9;
    private static final int UP = 0xB80E0F13, DOWN = 0xEEF2F3F5;

    final BoolSetting showCps = bool("cps", "CPS under mouse buttons", true);

    private final float[] press = new float[7];
    private final String[] labels = {"W", "A", "S", "D", "LMB", "RMB", ""};
    private String leftCps = "", rightCps = "";
    private float lastT;

    KeystrokesElement() {
        super("keystrokes", "Keystrokes", "Shows W, A, S, D, mouse buttons and jump as you press them.", false, true, 0.01f, 0.55f, 1f);
        w = K * 3 + G * 2;
        h = K + G + K + G + MOUSE_H + G + SPACE_H;
    }

    @Override
    public boolean hasContent() {
        return mc.thePlayer != null;
    }

    private KeyBinding bind(int i) {
        GameSettings gs = mc.gameSettings;
        switch (i) {
            case 0: return gs.keyBindForward;
            case 1: return gs.keyBindLeft;
            case 2: return gs.keyBindBack;
            case 3: return gs.keyBindRight;
            case 4: return gs.keyBindAttack;
            case 5: return gs.keyBindUseItem;
            default: return gs.keyBindJump;
        }
    }

    @Override
    public void refresh(boolean sample) {
        float now = Ease.seconds();
        float dt = Math.min(0.1f, Math.max(0f, now - lastT));
        lastT = now;
        boolean calm = General.calm();
        for (int i = 0; i < press.length; i++) {
            boolean down = !sample && bind(i).isKeyDown();
            if (sample) down = i == 0 || i == 4; // sample: W and LMB held
            press[i] = calm ? (down ? 1 : 0) : Ease.approach(press[i], down ? 1 : 0, 22f, dt);
        }
        if (due(100, sample)) {
            for (int i = 0; i < 4; i++) {
                int code = bind(i).getKeyCode();
                String n = code > 0 ? Keyboard.getKeyName(code) : null;
                labels[i] = n == null ? labels[i] : n.length() > 3 ? n.substring(0, 3) : n;
            }
            leftCps = (sample ? 7 : Hud.cps(0)) + " CPS";
            rightCps = (sample ? 2 : Hud.cps(1)) + " CPS";
        }
    }

    @Override
    protected void paint(boolean sample) {
        key(0, K + G, 0, K, K, 8f);
        key(1, 0, K + G, K, K, 8f);
        key(2, K + G, K + G, K, K, 8f);
        key(3, (K + G) * 2, K + G, K, K, 8f);
        float my = (K + G) * 2, mw = (w - G) / 2f;
        key(4, 0, my, mw, MOUSE_H, 7f);
        key(5, mw + G, my, mw, MOUSE_H, 7f);
        key(6, 0, my + MOUSE_H + G, w, SPACE_H, 7f);
    }

    private void key(int i, float x, float y, float kw, float kh, float size) {
        float p = press[i];
        float sh = 0.6f * p; // pressed keys sink slightly
        round(x, y + sh, kw, kh, 2.5f, Colors.mix(UP, DOWN, p));
        outline(x, y + sh, kw, kh, 2.5f, 0.5f, PANEL_EDGE);
        int tc = Colors.mix(0xFFE4E5EA, 0xFF0E0F13, p);
        if (i == 6) {
            float bw = kw * 0.4f;
            rect(x + (kw - bw) / 2f, y + sh + kh / 2f - 0.5f, bw, 1f, tc);
            return;
        }
        boolean cps = i >= 4 && showCps.on();
        float tw = textW(labels[i], size, true);
        if (!cps) {
            textMid(labels[i], x + (kw - tw) / 2f, y + sh + kh / 2f, size, true, tc);
            return;
        }
        String c = i == 4 ? leftCps : rightCps;
        float cw = textW(c, 5.5f, false);
        text(labels[i], x + (kw - tw) / 2f, y + sh + 1.5f, size, true, tc);
        text(c, x + (kw - cw) / 2f, y + sh + kh - lineH(5.5f) - 0.5f, 5.5f, false, Colors.alpha(tc, 0.75f));
    }
}
