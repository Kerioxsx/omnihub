package dev.vesper.client.hud;

import dev.vesper.client.module.setting.BoolSetting;
import net.minecraft.init.Items;
import net.minecraft.item.ItemStack;

/** Worn armour (and optionally the held item) with remaining durability. */
final class ArmorElement extends HudElement {
    private static final float ROW = 17, PAD = 3, SIZE = 7.5f;

    final BoolSetting held = bool("held", "Include held item", true);
    final BoolSetting percent = bool("percent", "Show as percentage", false);

    private final ItemStack[] rows = new ItemStack[5];
    private final String[] text = new String[5];
    private final int[] color = new int[5];
    private int n;
    private ItemStack[] samples;

    ArmorElement() {
        super("armor", "Armour status", "Your armour pieces and held item with the durability they have left.", true, true, 0.99f, 0.80f, 1f);
    }

    @Override
    public boolean hasContent() {
        return n > 0;
    }

    @Override
    public void refresh(boolean sample) {
        if (!due(200, sample)) return;
        n = 0;
        if (sample) {
            if (samples == null) samples = new ItemStack[] {new ItemStack(Items.diamond_helmet), new ItemStack(Items.diamond_chestplate), new ItemStack(Items.iron_leggings), new ItemStack(Items.diamond_boots), new ItemStack(Items.diamond_sword)};
            samples[1].setItemDamage(200);
            samples[2].setItemDamage(180);
            for (ItemStack s : samples) row(s);
        } else if (mc.thePlayer != null) {
            ItemStack[] armor = mc.thePlayer.inventory.armorInventory;
            for (int i = 3; i >= 0; i--) if (armor[i] != null) row(armor[i]);
            if (held.on()) {
                ItemStack h = mc.thePlayer.getHeldItem();
                if (h != null) row(h);
            }
        }
        float tw = 0;
        for (int i = 0; i < n; i++) tw = Math.max(tw, textW(text[i], SIZE, true));
        w = PAD * 2 + 16 + (tw > 0 ? 4 + tw : 0);
        h = PAD * 2 + ROW * Math.max(1, n) - 1;
    }

    private void row(ItemStack s) {
        rows[n] = s;
        if (s.isItemStackDamageable()) {
            int max = s.getMaxDamage(), left = max - s.getItemDamage();
            float f = max <= 0 ? 1 : left / (float) max;
            text[n] = percent.on() ? Math.round(f * 100) + "%" : String.valueOf(left);
            color[n] = f > 0.6f ? GOOD : f > 0.3f ? WARN : BAD;
        } else {
            text[n] = s.stackSize > 1 ? String.valueOf(s.stackSize) : "";
            color[n] = TEXT;
        }
        n++;
    }

    @Override
    protected void paint(boolean sample) {
        panel(0, 0, w, h);
        for (int i = 0; i < n; i++) {
            float y = PAD + i * ROW;
            item(rows[i], PAD, y);
            if (!text[i].isEmpty()) textMid(text[i], PAD + 20, y + 8, SIZE, true, color[i]);
        }
    }
}
