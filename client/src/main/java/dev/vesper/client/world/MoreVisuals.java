package dev.vesper.client.world;

import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;
import net.minecraft.block.Block;
import net.minecraft.block.material.Material;
import net.minecraft.client.Minecraft;
import net.minecraft.client.audio.PositionedSoundRecord;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.RenderGlobal;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.AxisAlignedBB;
import net.minecraft.util.BlockPos;
import net.minecraft.util.MovingObjectPosition;
import net.minecraft.util.ResourceLocation;
import net.minecraftforge.client.event.DrawBlockHighlightEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.opengl.GL11;

/** Hit sounds, a neon block outline and a client-side time of day. */
public final class MoreVisuals {
    private MoreVisuals() {
    }

    /** A sound when you hit something: Vesper's own sounds or Minecraft's. Only you hear it. */
    public static final class HitSounds extends Module {
        public static final String[] IDS = {"vesper:hit.pop", "vesper:hit.ding", "vesper:hit.thud", "vesper:hit.click", "vesper:hit.bass", "vesper:hit.crit", "random.successful_hit", "note.pling", "random.orb"};
        public final ChoiceSetting sound = choice("sound", "Sound", "vesper:hit.pop", IDS,
            new String[] {"Pop", "Ding", "Thud", "Click", "Bass", "Chime", "Arrow ding (Minecraft)", "Pling (Minecraft)", "Orb (Minecraft)"});
        public final ChoiceSetting crit = choice("crit", "On critical hits", "vesper:hit.crit", IDS,
            new String[] {"Pop", "Ding", "Thud", "Click", "Bass", "Chime", "Arrow ding (Minecraft)", "Pling (Minecraft)", "Orb (Minecraft)"});
        public final NumberSetting volume = number("volume", "Volume", 0.6, 0.05, 1, 0.05, "");
        public final NumberSetting pitch = number("pitch", "Pitch", 1.0, 0.5, 2, 0.05, "×");
        public final BoolSetting vary = bool("vary", "Vary the pitch a little", true);

        public HitSounds() {
            super("hitsounds", "Hit sounds", "A sound of your choice when you hit something (a different one on critical hits). Only you hear it.", Category.EFFECTS, false, true);
            CombatBus.listen(new CombatBus.Listener() {
                public void on(CombatVisualEvent e) {
                    if (e.type == CombatVisualEvent.Type.HIT_CONFIRMED) play();
                }
            });
        }

        void play() {
            Minecraft mc = Minecraft.getMinecraft();
            EntityPlayerSP p = mc.thePlayer;
            if (!enabled() || p == null) return;
            boolean isCrit = p.fallDistance > 0 && !p.onGround && !p.isOnLadder() && !p.isInWater() && p.ridingEntity == null;
            String id = isCrit ? crit.get() : sound.get();
            float pt = pitch.f() * (vary.on() ? 0.92f + (float) Math.random() * 0.16f : 1f);
            mc.getSoundHandler().playSound(new PositionedSoundRecord(new ResourceLocation(id), volume.f(), pt, (float) p.posX, (float) p.posY, (float) p.posZ));
        }
    }

    /** The outline of the block you look at, in neon, with an optional glowing fill. */
    public static final class BlockOutline extends Module {
        public final BoolSetting useWorld = bool("useWorld", "Use the world colour", true);
        public final ColorSetting color = color("color", "Colour (when not the world colour)", 0xFFFFFFFF);
        public final NumberSetting width = number("width", "Line width", 2.5, 1, 6, 0.5, "px");
        public final NumberSetting fill = number("fill", "Fill", 0.12, 0, 0.6, 0.02, "");
        public final BoolSetting pulse = bool("pulse", "Pulse", true);
        public final BoolSetting throughBlocks = bool("throughBlocks", "Visible through blocks", false);

        public BlockOutline() {
            super("blockoutline", "Block outline", "A neon outline (and glow) on the block you look at.", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onHighlight(DrawBlockHighlightEvent e) {
            if (!enabled() || e.target == null || e.target.typeOfHit != MovingObjectPosition.MovingObjectType.BLOCK) return;
            Minecraft mc = Minecraft.getMinecraft();
            BlockPos pos = e.target.getBlockPos();
            Block b = mc.theWorld.getBlockState(pos).getBlock();
            if (b.getMaterial() == Material.air || !mc.theWorld.getWorldBorder().contains(pos)) return;
            e.setCanceled(true);
            b.setBlockBoundsBasedOnState(mc.theWorld, pos);
            double px = e.player.lastTickPosX + (e.player.posX - e.player.lastTickPosX) * e.partialTicks;
            double py = e.player.lastTickPosY + (e.player.posY - e.player.lastTickPosY) * e.partialTicks;
            double pz = e.player.lastTickPosZ + (e.player.posZ - e.player.lastTickPosZ) * e.partialTicks;
            AxisAlignedBB bb = b.getSelectedBoundingBox(mc.theWorld, pos).expand(0.002, 0.002, 0.002).offset(-px, -py, -pz);
            int c = useWorld.on() ? General.accent() : color.argb();
            float k = pulse.on() ? 0.75f + 0.25f * (float) Math.sin(Ease.seconds() * 4) : 1f;
            GlStateManager.enableBlend();
            GlStateManager.tryBlendFuncSeparate(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA, 1, 0);
            GlStateManager.disableTexture2D();
            GlStateManager.depthMask(false);
            if (throughBlocks.on()) GlStateManager.disableDepth();
            if (fill.f() > 0) {
                GlStateManager.color(Colors.r(c), Colors.g(c), Colors.b(c), fill.f() * k);
                Tessellator t = Tessellator.getInstance();
                WorldRenderer w = t.getWorldRenderer();
                w.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION);
                double[][] f = {
                    {bb.minX, bb.minY, bb.minZ, bb.maxX, bb.minY, bb.minZ, bb.maxX, bb.minY, bb.maxZ, bb.minX, bb.minY, bb.maxZ},
                    {bb.minX, bb.maxY, bb.minZ, bb.minX, bb.maxY, bb.maxZ, bb.maxX, bb.maxY, bb.maxZ, bb.maxX, bb.maxY, bb.minZ},
                    {bb.minX, bb.minY, bb.minZ, bb.minX, bb.maxY, bb.minZ, bb.maxX, bb.maxY, bb.minZ, bb.maxX, bb.minY, bb.minZ},
                    {bb.minX, bb.minY, bb.maxZ, bb.maxX, bb.minY, bb.maxZ, bb.maxX, bb.maxY, bb.maxZ, bb.minX, bb.maxY, bb.maxZ},
                    {bb.minX, bb.minY, bb.minZ, bb.minX, bb.minY, bb.maxZ, bb.minX, bb.maxY, bb.maxZ, bb.minX, bb.maxY, bb.minZ},
                    {bb.maxX, bb.minY, bb.minZ, bb.maxX, bb.maxY, bb.minZ, bb.maxX, bb.maxY, bb.maxZ, bb.maxX, bb.minY, bb.maxZ},
                };
                for (double[] q : f) for (int i = 0; i < 12; i += 3) w.pos(q[i], q[i + 1], q[i + 2]).endVertex();
                t.draw();
            }
            // A wide faint pass under a thin bright one: neon.
            GL11.glLineWidth(width.f() * 3);
            GlStateManager.color(Colors.r(c), Colors.g(c), Colors.b(c), 0.25f * k);
            RenderGlobal.drawSelectionBoundingBox(bb);
            GL11.glLineWidth(width.f());
            GlStateManager.color(Colors.r(c), Colors.g(c), Colors.b(c), k);
            RenderGlobal.drawSelectionBoundingBox(bb);
            GL11.glLineWidth(1);
            if (throughBlocks.on()) GlStateManager.enableDepth();
            GlStateManager.depthMask(true);
            GlStateManager.enableTexture2D();
            GlStateManager.disableBlend();
            GlStateManager.color(1, 1, 1, 1);
        }
    }

    /** Your own time of day (only on your screen; the server's time is unchanged). */
    public static final class TimeChanger extends Module {
        public final ChoiceSetting time = choice("time", "Time", "night", new String[] {"sunrise", "day", "sunset", "night", "midnight", "custom"},
            new String[] {"Sunrise", "Day", "Sunset", "Night", "Midnight", "Custom"});
        public final NumberSetting hour = number("hour", "Custom hour", 20, 0, 23.75, 0.25, "h");
        public final BoolSetting moving = bool("moving", "Let it move slowly", false);
        private float drift;

        public TimeChanger() {
            super("timechanger", "Time changer", "Pick your own time of day (sunrise, day, sunset, night…), only on your screen.", Category.VISUALS, false, true);
        }

        private long ticks() {
            String t = time.get();
            long base = "sunrise".equals(t) ? 23200 : "day".equals(t) ? 6000 : "sunset".equals(t) ? 12500 : "night".equals(t) ? 15000 : "midnight".equals(t) ? 18000
                : (long) (((hour.f() - 6 + 24) % 24) * 1000);
            return base + (long) drift;
        }

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            apply(e.phase == TickEvent.Phase.END);
        }

        @SubscribeEvent
        public void onRender(TickEvent.RenderTickEvent e) {
            if (e.phase == TickEvent.Phase.START) apply(false);
        }

        private void apply(boolean advance) {
            Minecraft mc = Minecraft.getMinecraft();
            if (!enabled() || mc.theWorld == null) return;
            if (advance && moving.on()) drift = (drift + 1) % 24000;
            long day = mc.theWorld.getWorldTime() / 24000L;
            mc.theWorld.setWorldTime(day * 24000L + (ticks() % 24000L));
        }
    }
}
