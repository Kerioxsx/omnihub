package dev.vesper.client.hud;

import net.minecraft.util.MathHelper;

/** Block coordinates and the direction you face. Off by default. */
final class CoordsElement extends HudElement {
    private static final String[] FACING = {"Facing south (+Z)", "Facing west (−X)", "Facing north (−Z)", "Facing east (+X)"};
    private static final float SIZE = 7.5f, PAD = 5;
    private String l1 = "", l2 = "";

    CoordsElement() {
        super("coords", "Coordinates", "Your block position and the direction you face.", false, true, 0.01f, 0.32f, 1f);
    }

    @Override
    public boolean hasContent() {
        return mc.thePlayer != null;
    }

    @Override
    public void refresh(boolean sample) {
        if (!due(100, sample)) return;
        if (sample || mc.thePlayer == null) {
            l1 = "XYZ 128 64 -256";
            l2 = FACING[2];
        } else {
            l1 = "XYZ " + MathHelper.floor_double(mc.thePlayer.posX) + " " + MathHelper.floor_double(mc.thePlayer.getEntityBoundingBox().minY) + " "
                + MathHelper.floor_double(mc.thePlayer.posZ);
            l2 = FACING[MathHelper.floor_double(mc.thePlayer.rotationYaw * 4.0F / 360.0F + 0.5D) & 3];
        }
        w = PAD * 2 + Math.max(textW(l1, SIZE, false), textW(l2, SIZE, false));
        h = 8 + lineH(SIZE) * 2;
    }

    @Override
    protected void paint(boolean sample) {
        panel(0, 0, w, h);
        text(l1, PAD, 4, SIZE, false, TEXT);
        text(l2, PAD, 4 + lineH(SIZE), SIZE, false, TEXT_MUTED);
    }
}
