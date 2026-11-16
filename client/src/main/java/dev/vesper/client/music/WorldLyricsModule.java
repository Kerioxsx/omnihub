package dev.vesper.client.music;

import dev.vesper.client.fx.FxText;
import dev.vesper.client.fx.View;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.render.Colors;
import dev.vesper.client.render.Ease;
import dev.vesper.client.render.Gl;
import dev.vesper.client.render.GlyphFont;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.client.event.RenderWorldLastEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * Live lyrics of the song playing on your PC, in the Minecraft world:
 * floating in front of you, huge in the sky, above your head, or on the HUD.
 * The sung part of each line lights up as it plays. The song and its synced
 * lyrics come from OmniHub on this PC (Aurora's lyrics); only you see them.
 */
public final class WorldLyricsModule extends Module {
    public final ChoiceSetting place = choice("place", "Where", "front", new String[] {"front", "sky", "above", "hud"},
        new String[] {"Floating in front of you", "Huge in the sky", "Above your head", "On the HUD"});
    public final NumberSetting size = number("size", "Size", 1.0, 0.4, 3.0, 0.05, "×");
    public final NumberSetting distance = number("distance", "Distance", 6, 3, 14, 0.5, " blocks");
    public final NumberSetting lift = number("lift", "Height", 0.4, -1.5, 3, 0.1, " blocks");
    public final ColorSetting color = color("color", "Text colour", 0xCCFFFFFF);
    public final ColorSetting highlight = color("highlight", "Sung part colour", 0xFFFFD36E);
    public final ColorSetting outline = color("outline", "Outline", 0xC0140C24);
    public final BoolSetting next = bool("next", "Show the next line", true);
    public final BoolSetting songCard = bool("songCard", "Show the song when it changes", true);
    public final BoolSetting throughBlocks = bool("throughBlocks", "Visible through blocks", true);
    public final NumberSetting offset = number("offset", "Timing offset", 0, -3000, 3000, 50, " ms");

    private float yawSm = Float.NaN;
    private int shownLine = -2;
    private float lineChanged;
    private String prevText = "";
    private String songKey;
    private float songChanged = -100;

    public WorldLyricsModule() {
        super("lyrics", "World lyrics", "Live lyrics of the song playing on your PC, in your world (needs OmniHub with Aurora's lyrics).", Category.VISUALS, false, true);
    }

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase == TickEvent.Phase.END) LyricsFeed.want(enabled() && Minecraft.getMinecraft().theWorld != null);
    }

    /** The line to show now and how far through it the song is. */
    private Object[] current() {
        LyricsFeed.Snapshot s = LyricsFeed.get();
        if (s == null || s.key == null) return null;
        if (!s.key.equals(songKey)) {
            songKey = s.key;
            songChanged = Ease.seconds();
            shownLine = -2;
        }
        long pos = s.now(offset.i());
        int i = s.lineAt(pos);
        if (i != shownLine) {
            prevText = shownLine >= 0 && shownLine < s.text.length ? s.text[shownLine] : "";
            shownLine = i;
            lineChanged = Ease.seconds();
        }
        String line = i >= 0 ? s.text[i] : "";
        String nxt = i + 1 < s.text.length ? s.text[i + 1] : "";
        long end = i + 1 < s.ms.length ? s.ms[i + 1] : (i >= 0 ? s.ms[i] + 4000 : 0);
        float frac = i < 0 ? 0 : Ease.clamp01((pos - s.ms[i]) / (float) Math.max(300, Math.min(end - s.ms[i], 8000) * 0.92f));
        return new Object[] {s, line, nxt, frac};
    }

    private String card(LyricsFeed.Snapshot s) {
        return s.artist.isEmpty() ? s.title : s.title + "  ·  " + s.artist;
    }

    @SubscribeEvent
    public void onWorld(RenderWorldLastEvent e) {
        if (!enabled() || place.is("hud")) return;
        Minecraft mc = Minecraft.getMinecraft();
        EntityPlayerSP p = mc.thePlayer;
        Object[] c = current();
        if (p == null || c == null) return;
        LyricsFeed.Snapshot s = (LyricsFeed.Snapshot) c[0];
        RenderManager rm = mc.getRenderManager();
        float pt = e.partialTicks;
        double px = p.prevPosX + (p.posX - p.prevPosX) * pt, py = p.prevPosY + (p.posY - p.prevPosY) * pt + p.getEyeHeight(), pz = p.prevPosZ + (p.posZ - p.prevPosZ) * pt;
        float yaw = p.prevRotationYaw + (p.rotationYaw - p.prevRotationYaw) * pt;
        if (Float.isNaN(yawSm)) yawSm = yaw;
        float d = ((yaw - yawSm) % 360 + 540) % 360 - 180;
        yawSm += d * Math.min(1f, 0.08f * 60f / Math.max(20, Minecraft.getDebugFPS()));
        double ax, ay, az, h;
        double rad = Math.toRadians(yawSm);
        if (place.is("sky")) {
            ax = px - Math.sin(rad) * 40;
            az = pz + Math.cos(rad) * 40;
            ay = py + 16 + lift.f();
            h = 2.6 * size.f();
        } else if (place.is("above")) {
            ax = px;
            az = pz;
            ay = py + 0.9 + lift.f();
            h = 0.32 * size.f();
        } else {
            double dist = distance.f();
            ax = px - Math.sin(rad) * dist;
            az = pz + Math.cos(rad) * dist;
            ay = py + lift.f();
            h = 0.06 * dist * size.f();
        }
        Gl.beginGlow();
        View.capture();
        if (throughBlocks.on()) GlStateManager.disableDepth();
        drawBlock(s, (String) c[1], (String) c[2], (Float) c[3], ax - rm.viewerPosX, ay - rm.viewerPosY, az - rm.viewerPosZ, h);
        if (throughBlocks.on()) GlStateManager.enableDepth();
        Gl.endGlow();
    }

    private void drawBlock(LyricsFeed.Snapshot s, String line, String nxt, float frac, double x, double y, double z, double h) {
        float now = Ease.seconds();
        float tIn = Ease.outCubic(Ease.clamp01((now - lineChanged) / 0.28f));
        if (General.calm()) tIn = 1;
        int base = color.argb(), hi = highlight.argb(), out = outline.argb();
        // Line on its way out: slides up and fades.
        if (tIn < 1 && !prevText.isEmpty()) {
            FxText.push(x, y, z, h, 0);
            textLine(prevText, 0, -26 * tIn, 1, Colors.alpha(base, 1 - tIn), Colors.alpha(hi, 1 - tIn), Colors.alpha(out, 1 - tIn));
            FxText.pop();
        }
        FxText.push(x, y, z, h, 0);
        float lift = 22 * (1 - tIn);
        if (!line.isEmpty()) textLine(line, 0, lift, frac, Colors.alpha(base, tIn), Colors.alpha(hi, tIn), Colors.alpha(out, tIn));
        else if ("synced".equals(s.lyricsState) && s.playing) textLine("• • •", 0, lift, 1, base, hi, out);
        if (next.on() && !nxt.isEmpty()) {
            GlStateManager.pushMatrix();
            GlStateManager.translate(0, 24 + lift, 0);
            GlStateManager.scale(0.62f, 0.62f, 1);
            textLine(nxt, 0, 0, 0, Colors.alpha(base, 0.55f * tIn), hi, Colors.alpha(out, 0.5f * tIn));
            GlStateManager.popMatrix();
        }
        float cardA = songCard.on() ? 1 - Ease.clamp01((now - songChanged - 4f) / 0.6f) : 0;
        if (line.isEmpty() && !"synced".equals(s.lyricsState)) cardA = Math.max(cardA, songCard.on() ? 0.8f : 0);
        if (cardA > 0.01f) {
            GlStateManager.pushMatrix();
            GlStateManager.translate(0, -26, 0);
            GlStateManager.scale(0.55f, 0.55f, 1);
            textLine(card(s), 0, 0, 0, Colors.alpha(hi, cardA), hi, Colors.alpha(out, cardA));
            GlStateManager.popMatrix();
        }
        FxText.pop();
    }

    /** A centred line with its first `frac` lit up (by characters, smoothly). */
    private static void textLine(String s, float dx, float dy, float frac, int base, int hi, int out) {
        FxText.draw(s, dx, dy, base, out);
        if (frac <= 0) return;
        int n = s.length();
        int k = Math.min(n, (int) Math.floor(frac * n));
        if (k <= 0) return;
        float w = FxText.width(s);
        String sung = s.substring(0, k);
        float sw = FxText.width(sung);
        FxText.draw(sung, dx - w / 2 + sw / 2, dy, hi, 0);
    }

    @SubscribeEvent
    public void onHud(RenderGameOverlayEvent.Post e) {
        if (!enabled() || !place.is("hud") || e.type != RenderGameOverlayEvent.ElementType.ALL) return;
        Object[] c = current();
        if (c == null) return;
        LyricsFeed.Snapshot s = (LyricsFeed.Snapshot) c[0];
        String line = (String) c[1], nxt = (String) c[2];
        float frac = (Float) c[3];
        ScaledResolution sr = new ScaledResolution(Minecraft.getMinecraft());
        float cx = sr.getScaledWidth() / 2f, y = sr.getScaledHeight() * 0.68f;
        float sz = 13 * size.f();
        float tIn = General.calm() ? 1 : Ease.outCubic(Ease.clamp01((Ease.seconds() - lineChanged) / 0.28f));
        float dy = 8 * (1 - tIn);
        if (!line.isEmpty()) {
            float w = GlyphFont.width(line, sz, true);
            GlyphFont.drawCentered(line, cx + 1, y + dy + 1, sz, true, Colors.alpha(outline.argb(), tIn));
            GlyphFont.drawCentered(line, cx, y + dy, sz, true, Colors.alpha(color.argb(), tIn));
            int k = Math.min(line.length(), (int) Math.floor(frac * line.length()));
            if (k > 0) GlyphFont.draw(line.substring(0, k), cx - w / 2, y + dy, sz, true, Colors.alpha(highlight.argb(), tIn));
        }
        if (next.on() && !nxt.isEmpty()) GlyphFont.drawCentered(nxt, cx, y + sz * 1.4f + dy, sz * 0.7f, false, Colors.alpha(color.argb(), 0.5f * tIn));
        float cardA = songCard.on() ? 1 - Ease.clamp01((Ease.seconds() - songChanged - 4f) / 0.6f) : 0;
        if (cardA > 0.01f) GlyphFont.drawCentered(card(s), cx, y - sz * 1.2f, sz * 0.62f, false, Colors.alpha(highlight.argb(), cardA));
    }
}
