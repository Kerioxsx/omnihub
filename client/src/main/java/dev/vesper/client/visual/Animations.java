package dev.vesper.client.visual;

import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.fx.FxText;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Draw;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityLivingBase;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.client.event.RenderWorldLastEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.opengl.GL11;

/** Small motion touches: damage numbers, the hotbar, low health and the crosshair. */
public final class Animations {
    private Animations() {
    }

    /**
     * Numbers that pop out of anything that loses health near you (in hearts),
     * rise and fade; a different colour for healing. Read from the health the
     * game already shows, so it works on any server.
     */
    public static final class DamageNumbers extends Module {
        public final ColorSetting damage = color("damage", "Damage colour", 0xFFFF4A5A);
        public final ColorSetting crit = color("crit", "Big hits (3+ hearts)", 0xFFFFC23D);
        public final BoolSetting heal = bool("heal", "Show healing too", true);
        public final ColorSetting healColor = color("healColor", "Healing colour", 0xFF54F59A);
        public final BoolSetting onlyMine = bool("onlyMine", "Only what I hit", false);
        public final NumberSetting size = number("size", "Size", 1, 0.5, 2, 0.05, "×");
        public final NumberSetting range = number("range", "Range", 24, 6, 48, 1, " blocks");

        private final Map<Integer, Float> hp = new HashMap<Integer, Float>();
        private final List<float[]> pops = new ArrayList<float[]>(); // x, y, z, amount, start, drift
        private int lastHitId = -1;
        private float lastHitAt = -10;

        public DamageNumbers() {
            super("damagenumbers", "Damage numbers", "Numbers pop out of anything that gets hurt near you (in hearts), rise and fade.", Category.EFFECTS, false, true);
            CombatBus.listen(new CombatBus.Listener() {
                public void on(CombatVisualEvent e) {
                    if (e.type == CombatVisualEvent.Type.HIT_CONFIRMED) {
                        lastHitId = e.targetEntityId;
                        lastHitAt = Ease.seconds();
                    }
                }
            });
        }

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            Minecraft mc = Minecraft.getMinecraft();
            if (e.phase != TickEvent.Phase.END) return;
            if (mc.theWorld == null || mc.thePlayer == null) {
                hp.clear();
                return;
            }
            if (!enabled()) return;
            Map<Integer, Float> seen = new HashMap<Integer, Float>();
            double r2 = range.f() * range.f();
            for (Entity o : mc.theWorld.loadedEntityList) {
                if (!(o instanceof EntityLivingBase) || o == mc.thePlayer || o.getDistanceSqToEntity(mc.thePlayer) > r2) continue;
                EntityLivingBase en = (EntityLivingBase) o;
                float now = en.getHealth();
                Float before = hp.get(en.getEntityId());
                seen.put(en.getEntityId(), now);
                if (before == null) continue;
                float diff = before - now;
                boolean mine = en.getEntityId() == lastHitId && Ease.seconds() - lastHitAt < 1.5f;
                if (onlyMine.on() && !mine) continue;
                if (diff >= 0.5f || (heal.on() && diff <= -1f && now < en.getMaxHealth())) {
                    if (pops.size() > 40) pops.remove(0);
                    pops.add(new float[] {(float) en.posX, (float) (en.posY + en.height + 0.25), (float) en.posZ, diff / 2f, Ease.seconds(), (float) (Math.random() * 2 - 1)});
                }
            }
            hp.clear();
            hp.putAll(seen);
        }

        @SubscribeEvent
        public void onWorld(RenderWorldLastEvent e) {
            if (!enabled() || pops.isEmpty() || !General.fx()) return;
            RenderManager rm = Minecraft.getMinecraft().getRenderManager();
            float now = Ease.seconds();
            Gl.beginGlow();
            GlStateManager.disableDepth();
            View.capture();
            for (Iterator<float[]> it = pops.iterator(); it.hasNext();) {
                float[] p = it.next();
                float t = now - p[4];
                if (t > 1.1f) {
                    it.remove();
                    continue;
                }
                float rise = Ease.outCubic(Ease.clamp01(t / 1.1f));
                float pop = Ease.outBack(Ease.clamp01(t / 0.18f));
                float a = 1 - Ease.span(t, 0.7f, 1.1f);
                boolean healing = p[3] < 0;
                float amount = Math.abs(p[3]);
                int c = healing ? healColor.argb() : amount >= 3 ? crit.argb() : damage.argb();
                String s = (healing ? "+" : "") + (amount >= 10 ? String.valueOf(Math.round(amount)) : String.format("%.1f", amount));
                double x = p[0] - rm.viewerPosX + p[5] * 0.35 * rise, y = p[1] - rm.viewerPosY + 0.7 * rise, z = p[2] - rm.viewerPosZ;
                double h = 0.26 * size.f() * pop * (amount >= 3 ? 1.3 : 1);
                GlStateManager.blendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
                FxText.text(s, x, y, z, h, 0, Colors.alpha(c, a), Colors.alpha(0xFF000000, 0.8f * a));
            }
            GlStateManager.enableDepth();
            Gl.endGlow();
        }
    }

    /** A soft highlight that glides between hotbar slots instead of jumping. */
    public static final class SmoothHotbar extends Module {
        public final BoolSetting glow = bool("glow", "Glow in my world colour", true);
        public final NumberSetting speed = number("speed", "Speed", 18, 6, 40, 1, "");
        private float slot = -1;
        private long last;

        public SmoothHotbar() {
            super("smoothhotbar", "Smooth hotbar", "The selected-slot highlight glides from slot to slot, with a soft glow.", Category.HUD, true, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Post e) {
            Minecraft mc = Minecraft.getMinecraft();
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.HOTBAR || mc.thePlayer == null) return;
            long now = System.nanoTime();
            float dt = last == 0 ? 0.016f : Math.min(0.1f, (now - last) / 1e9f);
            last = now;
            int target = mc.thePlayer.inventory.currentItem;
            slot = slot < 0 ? target : Ease.approach(slot, target, speed.f(), dt);
            ScaledResolution sr = e.resolution;
            float x = sr.getScaledWidth() / 2f - 91 + slot * 20, y = sr.getScaledHeight() - 22;
            int c = glow.on() ? General.accent() : 0xFFFFFF;
            Gl.begin2D();
            Draw.glow(x + 12, y + 11, 18, Colors.withAlpha(c, 0.28f));
            Draw.roundOutline(x + 1, y + 1, 22, 22, 3, 1.5, Colors.withAlpha(c, 0.95f));
            Gl.end2D();
        }
    }

    /** The screen edges pulse red when your health is low, faster as it drops. */
    public static final class LowHealth extends Module {
        public final NumberSetting below = number("below", "Below", 6, 1, 14, 1, " hearts");
        public final NumberSetting strength = number("strength", "Strength", 0.6, 0.1, 1, 0.05, "");
        public final ColorSetting colour = color("colour", "Colour", 0xFFE0102A);

        public LowHealth() {
            super("lowhealth", "Low-health pulse", "The screen edges pulse red when your health is low, faster as it drops.", Category.HUD, false, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Post e) {
            Minecraft mc = Minecraft.getMinecraft();
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.ALL || mc.thePlayer == null || mc.thePlayer.capabilities.isCreativeMode) return;
            float hearts = mc.thePlayer.getHealth() / 2f, lim = below.f();
            if (hearts >= lim || hearts <= 0) return;
            float danger = 1 - hearts / lim;
            float beat = 0.5f + 0.5f * (float) Math.sin(Ease.seconds() * (3 + 6 * danger));
            float a = strength.f() * (0.35f + 0.65f * danger) * (0.55f + 0.45f * beat);
            int w = e.resolution.getScaledWidth(), h = e.resolution.getScaledHeight();
            int c = colour.argb();
            float edge = Math.min(w, h) * (0.12f + 0.1f * danger);
            Gl.begin2D();
            Draw.gradientV(0, 0, w, edge, Colors.alpha(c, a), Colors.alpha(c, 0));
            Draw.gradientV(0, h - edge, w, h, Colors.alpha(c, 0), Colors.alpha(c, a));
            Draw.gradientH(0, 0, edge, h, Colors.alpha(c, a), Colors.alpha(c, 0));
            Draw.gradientH(w - edge, 0, w, h, Colors.alpha(c, 0), Colors.alpha(c, a));
            Gl.end2D();
        }
    }

    /** A quick ring and four ticks burst from the crosshair when a hit lands. */
    public static final class HitPulse extends Module {
        public final ColorSetting colour = color("colour", "Colour", 0xFFFFFFFF);
        public final BoolSetting world = bool("world", "Use my world colour", false);
        public final NumberSetting size = number("size", "Size", 1, 0.5, 2, 0.05, "×");
        private float hitAt = -10;

        public HitPulse() {
            super("hitpulse", "Crosshair hit pulse", "A quick ring and four ticks burst from the crosshair when your hit lands.", Category.EFFECTS, false, true);
            CombatBus.listen(new CombatBus.Listener() {
                public void on(CombatVisualEvent e) {
                    if (e.type == CombatVisualEvent.Type.HIT_CONFIRMED) hitAt = Ease.seconds();
                }
            });
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Post e) {
            if (!enabled() || e.type != RenderGameOverlayEvent.ElementType.CROSSHAIRS) return;
            float t = Ease.seconds() - hitAt;
            if (t > 0.3f) return;
            float k = Ease.outCubic(t / 0.3f), a = 1 - k;
            float cx = e.resolution.getScaledWidth() / 2f, cy = e.resolution.getScaledHeight() / 2f, s = size.f();
            int c = world.on() ? (0xFF000000 | General.accent()) : colour.argb();
            Gl.begin2D();
            float r = (3 + 7 * k) * s;
            Draw.ring(cx, cy, r, r + 1.2f * s, Colors.alpha(c, a), Colors.alpha(c, a));
            for (int i = 0; i < 4; i++) {
                double ang = Math.PI / 4 + i * Math.PI / 2;
                float r1 = (5 + 6 * k) * s, r2 = r1 + 3 * s;
                Draw.line(cx + Math.cos(ang) * r1, cy + Math.sin(ang) * r1, cx + Math.cos(ang) * r2, cy + Math.sin(ang) * r2, 1.4f * s, Colors.alpha(c, a));
            }
            Gl.end2D();
        }
    }
}
