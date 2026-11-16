package dev.vesper.client.hud;

import net.minecraft.client.Minecraft;
import net.minecraft.client.network.NetHandlerPlayClient;
import net.minecraft.client.network.NetworkPlayerInfo;

/** Your latency as the server reports it in the player list. Hidden in singleplayer. */
final class PingElement extends LineElement {
    private boolean has;

    PingElement() {
        super("ping", "Ping", "Your latency from the server's player list (hidden in singleplayer).", false, 0.01f, 0.15f);
    }

    /** Response time in ms from the tab list, or -1 (singleplayer / unknown). */
    static int ping() {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.isSingleplayer() || mc.thePlayer == null) return -1;
        NetHandlerPlayClient nh = mc.getNetHandler();
        if (nh == null) return -1;
        NetworkPlayerInfo info = nh.getPlayerInfo(mc.thePlayer.getUniqueID());
        return info == null ? -1 : info.getResponseTime();
    }

    @Override
    public boolean hasContent() {
        return has;
    }

    @Override
    protected void compute(boolean sample) {
        int p = sample ? 23 : ping();
        has = p >= 0;
        label = "Ping";
        value = p + " ms";
        valueColor = p < 100 ? GOOD : p < 200 ? WARN : BAD;
    }
}
