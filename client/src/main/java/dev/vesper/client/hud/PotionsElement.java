package dev.vesper.client.hud;

import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import net.minecraft.client.resources.I18n;
import net.minecraft.potion.Potion;
import net.minecraft.potion.PotionEffect;
import net.minecraft.util.ResourceLocation;

/** Active potion effects with their level and the time they have left. */
final class PotionsElement extends HudElement {
    private static final ResourceLocation INVENTORY = new ResourceLocation("textures/gui/container/inventory.png");
    private static final String[] ROMAN = {"", " II", " III", " IV", " V", " VI", " VII", " VIII", " IX", " X"};
    private static final float ROW = 13, PAD = 4, SIZE = 7.5f, ICON = 9;

    private final List<String> names = new ArrayList<String>(), times = new ArrayList<String>();
    private final List<Integer> icons = new ArrayList<Integer>(), colors = new ArrayList<Integer>();
    private float nameW;

    PotionsElement() {
        super("potions", "Potion effects", "Your active potion effects and the time each has left.", true, true, 0.99f, 0.35f, 1f);
    }

    @Override
    public boolean hasContent() {
        return !names.isEmpty();
    }

    @Override
    public void refresh(boolean sample) {
        if (!due(250, sample)) return;
        names.clear();
        times.clear();
        icons.clear();
        colors.clear();
        if (sample) {
            add(Potion.moveSpeed, "1:24", 1);
            add(Potion.regeneration, "0:12", 0);
        } else if (mc.thePlayer != null) {
            Collection<PotionEffect> effects = mc.thePlayer.getActivePotionEffects();
            for (PotionEffect e : effects) {
                int id = e.getPotionID();
                if (id < 0 || id >= Potion.potionTypes.length || Potion.potionTypes[id] == null) continue;
                add(Potion.potionTypes[id], e.getIsPotionDurationMax() ? "∞" : Potion.getDurationString(e), e.getAmplifier());
            }
        }
        nameW = 0;
        float timeW = 0;
        for (int i = 0; i < names.size(); i++) {
            nameW = Math.max(nameW, textW(names.get(i), SIZE, false));
            timeW = Math.max(timeW, textW(times.get(i), SIZE, true));
        }
        w = PAD * 2 + ICON + 4 + nameW + 6 + timeW;
        h = PAD * 2 + ROW * Math.max(1, names.size()) - 2;
    }

    private void add(Potion p, String time, int amp) {
        names.add(I18n.format(p.getName()) + (amp > 0 && amp < ROMAN.length ? ROMAN[amp] : amp > 0 ? " " + (amp + 1) : ""));
        times.add(time);
        icons.add(p.hasStatusIcon() ? p.getStatusIconIndex() : -1);
        colors.add(0xFF000000 | p.getLiquidColor());
    }

    @Override
    protected void paint(boolean sample) {
        panel(0, 0, w, h);
        for (int i = 0; i < names.size(); i++) {
            float y = PAD + i * ROW, cy = y + (ROW - 2) / 2f;
            int idx = icons.get(i);
            if (idx >= 0) {
                float u = (idx % 8) * 18 / 256f, v = (198 + (idx / 8) * 18) / 256f;
                sprite(INVENTORY, PAD, cy - ICON / 2f, ICON, ICON, u, v, u + 18 / 256f, v + 18 / 256f, 0xFFFFFFFF);
            } else {
                circle(PAD + ICON / 2f, cy, 3f, colors.get(i));
            }
            float tx = PAD + ICON + 4;
            textMid(names.get(i), tx, cy, SIZE, false, TEXT);
            textMid(times.get(i), tx + nameW + 6, cy, SIZE, true, TEXT_MUTED);
        }
    }
}
