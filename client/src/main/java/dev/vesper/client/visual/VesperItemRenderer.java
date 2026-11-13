package dev.vesper.client.visual;

import dev.vesper.client.render.Ease;
import java.lang.reflect.Field;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.AbstractClientPlayer;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.ItemRenderer;
import net.minecraft.client.renderer.OpenGlHelper;
import net.minecraft.client.renderer.RenderHelper;
import net.minecraft.client.renderer.block.model.ItemCameraTransforms;
import net.minecraft.item.EnumAction;
import net.minecraft.item.ItemMap;
import net.minecraft.item.ItemStack;
import net.minecraft.util.BlockPos;
import net.minecraft.util.MathHelper;

/**
 * Minecraft's first-person item renderer with Vesper's swing styles. With
 * the module off, or for maps and the empty hand, it is Minecraft's own
 * code (super). The transforms below follow the game's so the item sits
 * exactly where players expect before any style is applied.
 */
final class VesperItemRenderer extends ItemRenderer {
    private final Minecraft mc;
    private static Field itemF, equipF, prevF;

    VesperItemRenderer(Minecraft mc) throws Exception {
        super(mc);
        this.mc = mc;
        if (itemF == null) {
            // By name (development and SRG names), else by declaration order:
            // itemToRender, equippedProgress, prevEquippedProgress.
            itemF = named("itemToRender", "field_78453_b");
            equipF = named("equippedProgress", "field_78454_c");
            prevF = named("prevEquippedProgress", "field_78451_d");
            if (itemF == null || equipF == null || prevF == null) {
                Field stack = null;
                Field[] floats = new Field[2];
                int nf = 0;
                for (Field f : ItemRenderer.class.getDeclaredFields()) {
                    if (f.getType() == ItemStack.class && stack == null) stack = f;
                    else if (f.getType() == float.class && nf < 2) floats[nf++] = f;
                }
                if (stack == null || nf < 2) throw new IllegalStateException("item renderer fields not found");
                itemF = stack;
                equipF = floats[0];
                prevF = floats[1];
            }
            itemF.setAccessible(true);
            equipF.setAccessible(true);
            prevF.setAccessible(true);
        }
    }

    private static Field named(String... names) {
        for (String n : names) {
            try {
                return ItemRenderer.class.getDeclaredField(n);
            } catch (NoSuchFieldException ignored) {
                // try the next name
            }
        }
        return null;
    }

    @Override
    public void renderItemInFirstPerson(float pt) {
        SwingModule m = SwingModule.INSTANCE;
        ItemStack item;
        float equip, prev;
        try {
            item = (ItemStack) itemF.get(this);
            equip = equipF.getFloat(this);
            prev = prevF.getFloat(this);
        } catch (Exception e) {
            super.renderItemInFirstPerson(pt);
            return;
        }
        if (m == null || !m.enabled() || item == null || item.getItem() instanceof ItemMap || !(mc.thePlayer instanceof EntityPlayerSP)) {
            super.renderItemInFirstPerson(pt);
            return;
        }
        EntityPlayerSP p = (EntityPlayerSP) mc.thePlayer;
        float f = m.noLowering.on() ? 0 : 1.0F - (prev + (equip - prev) * pt);
        float swing = m.progress(p, pt);
        float pitch = p.prevRotationPitch + (p.rotationPitch - p.prevRotationPitch) * pt;
        float yaw = p.prevRotationYaw + (p.rotationYaw - p.prevRotationYaw) * pt;
        rotateAround(pitch, yaw);
        lightFrom(p);
        armSway(p, pt);
        GlStateManager.enableRescaleNormal();
        GlStateManager.pushMatrix();
        GlStateManager.translate(m.x.f(), m.y.f(), -m.z.f());
        boolean using = p.getItemInUseCount() > 0;
        EnumAction act = using ? item.getItemUseAction() : EnumAction.NONE;
        if (using && act == EnumAction.BLOCK) {
            block(m, f, swing);
        } else if (using && (act == EnumAction.EAT || act == EnumAction.DRINK)) {
            drinking(p, item, pt);
            transform(f, 0, m.size.f());
        } else if (using && act == EnumAction.BOW) {
            transform(f, 0, m.size.f());
            bow(p, item, pt);
        } else if (using) {
            transform(f, 0, m.size.f());
        } else {
            swing(m, f, swing);
        }
        renderItem(p, item, ItemCameraTransforms.TransformType.FIRST_PERSON);
        GlStateManager.popMatrix();
        GlStateManager.disableRescaleNormal();
        RenderHelper.disableStandardItemLighting();
    }

    // ------------------------------------------------------------ styles

    private static float bump(float s) {
        return MathHelper.sin(s * (float) Math.PI);
    }

    private void swing(SwingModule m, float f, float s) {
        float k = m.size.f();
        String st = m.style.get();
        if ("vanilla".equals(st)) {
            used(s);
            transform(f, s, k);
        } else if ("classic".equals(st)) {
            transform(f, s, k);
        } else if ("smooth".equals(st)) {
            float e = Ease.inOutCubic(s);
            used(e * 0.85f);
            transform(f, e, k);
        } else if ("spin".equals(st)) {
            transform(f, 0, k);
            GlStateManager.rotate(-360 * Ease.outCubic(s), 0, 0, 1);
        } else if ("stab".equals(st)) {
            GlStateManager.translate(0, 0, -0.45 * bump(s));
            transform(f, 0, k);
            GlStateManager.rotate(-20 * bump(s), 1, 0, 0);
        } else if ("chop".equals(st)) {
            transform(f, 0, k);
            GlStateManager.rotate(-75 * bump(Ease.outCubic(s)), 1, 0, 0);
            GlStateManager.rotate(10 * bump(s), 0, 0, 1);
        } else if ("swirl".equals(st)) {
            transform(f, s * 0.5f, k);
            GlStateManager.rotate(55 * bump(s), 0, 1, 0);
            GlStateManager.rotate(-35 * bump(s), 0, 0, 1);
        } else { // flick
            float b = bump(Ease.outCubic(s));
            GlStateManager.translate(-0.15 * b, 0.1 * b, 0);
            transform(f, 0, k);
            GlStateManager.rotate(-50 * b, 0, 0, 1);
        }
    }

    private void block(SwingModule m, float f, float s) {
        float k = m.size.f();
        String b = m.block.get();
        if ("swing".equals(b)) {
            transform(f, s, k);
            blockPose();
        } else if ("push".equals(b)) {
            GlStateManager.translate(0, 0, -0.3 * bump(s));
            transform(f, 0, k);
            blockPose();
        } else if ("tilt".equals(b)) {
            transform(f * 0.5f, s * 0.6f, k);
            blockPose();
            GlStateManager.rotate(-25 * bump(s), 1, 0, 0);
        } else {
            transform(f, 0, k);
            blockPose();
        }
    }

    // ------------------------------------------------------------ Minecraft's transforms

    private void rotateAround(float angle, float angleY) {
        GlStateManager.pushMatrix();
        GlStateManager.rotate(angle, 1.0F, 0.0F, 0.0F);
        GlStateManager.rotate(angleY, 0.0F, 1.0F, 0.0F);
        RenderHelper.enableStandardItemLighting();
        GlStateManager.popMatrix();
    }

    private void lightFrom(AbstractClientPlayer p) {
        int i = mc.theWorld.getCombinedLight(new BlockPos(p.posX, p.posY + p.getEyeHeight(), p.posZ), 0);
        OpenGlHelper.setLightmapTextureCoords(OpenGlHelper.lightmapTexUnit, i & 65535, i >> 16);
    }

    private void armSway(EntityPlayerSP p, float pt) {
        float f = p.prevRenderArmPitch + (p.renderArmPitch - p.prevRenderArmPitch) * pt;
        float f1 = p.prevRenderArmYaw + (p.renderArmYaw - p.prevRenderArmYaw) * pt;
        GlStateManager.rotate((p.rotationPitch - f) * 0.1F, 1.0F, 0.0F, 0.0F);
        GlStateManager.rotate((p.rotationYaw - f1) * 0.1F, 0.0F, 1.0F, 0.0F);
    }

    private void transform(float equip, float swing, float size) {
        GlStateManager.translate(0.56F, -0.52F, -0.71999997F);
        GlStateManager.translate(0.0F, equip * -0.6F, 0.0F);
        GlStateManager.rotate(45.0F, 0.0F, 1.0F, 0.0F);
        float f = MathHelper.sin(swing * swing * (float) Math.PI);
        float f1 = MathHelper.sin(MathHelper.sqrt_float(swing) * (float) Math.PI);
        GlStateManager.rotate(f * -20.0F, 0.0F, 1.0F, 0.0F);
        GlStateManager.rotate(f1 * -20.0F, 0.0F, 0.0F, 1.0F);
        GlStateManager.rotate(f1 * -80.0F, 1.0F, 0.0F, 0.0F);
        GlStateManager.scale(0.4F * size, 0.4F * size, 0.4F * size);
    }

    private void used(float swing) {
        float f = -0.4F * MathHelper.sin(MathHelper.sqrt_float(swing) * (float) Math.PI);
        float f1 = 0.2F * MathHelper.sin(MathHelper.sqrt_float(swing) * (float) Math.PI * 2.0F);
        float f2 = -0.2F * MathHelper.sin(swing * (float) Math.PI);
        GlStateManager.translate(f, f1, f2);
    }

    private void blockPose() {
        GlStateManager.translate(-0.5F, 0.2F, 0.0F);
        GlStateManager.rotate(30.0F, 0.0F, 1.0F, 0.0F);
        GlStateManager.rotate(-80.0F, 1.0F, 0.0F, 0.0F);
        GlStateManager.rotate(60.0F, 0.0F, 1.0F, 0.0F);
    }

    private void drinking(AbstractClientPlayer p, ItemStack item, float pt) {
        float f = p.getItemInUseCount() - pt + 1.0F;
        float f1 = f / item.getMaxItemUseDuration();
        float f2 = MathHelper.abs(MathHelper.cos(f / 4.0F * (float) Math.PI) * 0.1F);
        if (f1 >= 0.8F) f2 = 0.0F;
        GlStateManager.translate(0.0F, f2, 0.0F);
        float f3 = 1.0F - (float) Math.pow(f1, 27.0D);
        GlStateManager.translate(f3 * 0.6F, f3 * -0.5F, 0.0F);
        GlStateManager.rotate(f3 * 90.0F, 0.0F, 1.0F, 0.0F);
        GlStateManager.rotate(f3 * 10.0F, 1.0F, 0.0F, 0.0F);
        GlStateManager.rotate(f3 * 30.0F, 0.0F, 0.0F, 1.0F);
    }

    private void bow(AbstractClientPlayer p, ItemStack item, float pt) {
        GlStateManager.rotate(-18.0F, 0.0F, 0.0F, 1.0F);
        GlStateManager.rotate(-12.0F, 0.0F, 1.0F, 0.0F);
        GlStateManager.rotate(-8.0F, 1.0F, 0.0F, 0.0F);
        GlStateManager.translate(-0.9F, 0.2F, 0.0F);
        float f = item.getMaxItemUseDuration() - (p.getItemInUseCount() - pt + 1.0F);
        float f1 = f / 20.0F;
        f1 = (f1 * f1 + f1 * 2.0F) / 3.0F;
        if (f1 > 1.0F) f1 = 1.0F;
        if (f1 > 0.1F) {
            float f2 = MathHelper.sin((f - 0.1F) * 1.3F);
            float f3 = f1 - 0.1F;
            GlStateManager.translate(0.0F, f2 * f3 * 0.01F, 0.0F);
        }
        GlStateManager.translate(0.0F, 0.0F, f1 * 0.1F);
        GlStateManager.scale(1.0F, 1.0F, 1.0F + f1 * 0.2F);
    }
}
