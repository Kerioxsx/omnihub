package dev.vesper.client.packs;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import dev.vesper.client.VesperClient;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.net.URL;
import java.net.URLEncoder;
import java.security.KeyStore;
import java.security.MessageDigest;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.SSLContext;
import javax.net.ssl.SSLSocketFactory;
import javax.net.ssl.TrustManagerFactory;

/**
 * Resource packs from Modrinth's public API (no account): search packs made
 * for 1.8.9, then download one into the game's resourcepacks folder,
 * checked against the SHA-1 Modrinth publishes. Only https to
 * api.modrinth.com and cdn.modrinth.com. The game's old Java may lack
 * today's root certificates, so Vesper adds a few public roots from its own
 * jar to Java's list.
 */
public final class Modrinth {
    private static final String UA = "Vesper-Client/" + VesperClient.VERSION + " (github.com/Kerioxsx/omnihub)";
    private static SSLSocketFactory ssl;

    private Modrinth() {
    }

    public static final class Pack {
        public final String id, title, description, author;
        public final long downloads;

        Pack(String id, String title, String description, String author, long downloads) {
            this.id = id;
            this.title = title;
            this.description = description;
            this.author = author;
            this.downloads = downloads;
        }
    }

    private static synchronized SSLSocketFactory ssl() {
        if (ssl != null) return ssl;
        try {
            KeyStore ks = KeyStore.getInstance(KeyStore.getDefaultType());
            ks.load(null, null);
            // Java's own roots first.
            TrustManagerFactory def = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());
            def.init((KeyStore) null);
            int n = 0;
            for (javax.net.ssl.TrustManager tm : def.getTrustManagers()) {
                if (tm instanceof javax.net.ssl.X509TrustManager) {
                    for (X509Certificate c : ((javax.net.ssl.X509TrustManager) tm).getAcceptedIssuers()) ks.setCertificateEntry("java-" + n++, c);
                }
            }
            InputStream in = Modrinth.class.getResourceAsStream("/assets/vesper/certs/roots.pem");
            if (in != null) {
                Collection<? extends Certificate> extra = CertificateFactory.getInstance("X.509").generateCertificates(in);
                int k = 0;
                for (Certificate c : extra) ks.setCertificateEntry("vesper-" + k++, c);
                in.close();
            }
            TrustManagerFactory tmf = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());
            tmf.init(ks);
            SSLContext ctx = SSLContext.getInstance("TLSv1.2");
            ctx.init(null, tmf.getTrustManagers(), null);
            ssl = ctx.getSocketFactory();
        } catch (Exception e) {
            VesperClient.LOG.warn("Couldn't add Vesper's root certificates: {}", e.toString());
            ssl = HttpsURLConnection.getDefaultSSLSocketFactory();
        }
        return ssl;
    }

    private static HttpsURLConnection open(String url) throws Exception {
        if (!url.startsWith("https://api.modrinth.com/") && !url.startsWith("https://cdn.modrinth.com/")) throw new IllegalArgumentException("only Modrinth is asked");
        HttpsURLConnection c = (HttpsURLConnection) new URL(url).openConnection();
        c.setSSLSocketFactory(ssl());
        c.setConnectTimeout(8000);
        c.setReadTimeout(20000);
        c.setRequestProperty("User-Agent", UA);
        return c;
    }

    private static byte[] read(HttpsURLConnection c, int max) throws Exception {
        int code = c.getResponseCode();
        if (code != 200) throw new IllegalStateException("Modrinth answered " + code);
        InputStream in = c.getInputStream();
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] buf = new byte[16384];
        int n;
        while ((n = in.read(buf)) > 0) {
            out.write(buf, 0, n);
            if (out.size() > max) throw new IllegalStateException("too big");
        }
        in.close();
        return out.toByteArray();
    }

    /** {@link #read} with up to three tries, a little longer apart each time. */
    private static byte[] fetch(String url, int max) throws Exception {
        Exception last = null;
        for (int n = 0; n < 3; n++) {
            if (n > 0) Thread.sleep(700L * n * n);
            try {
                return read(open(url), max);
            } catch (IllegalArgumentException e) {
                throw e;
            } catch (Exception e) {
                last = e;
                if (String.valueOf(e.getMessage()).contains("answered 404")) break;
            }
        }
        throw last;
    }

    private static String str(JsonObject o, String k) {
        JsonElement e = o.get(k);
        return e == null || e.isJsonNull() ? "" : e.getAsString();
    }

    /** Packs for 1.8.9 matching `query` (most downloaded first when empty). */
    public static List<Pack> search(String query) throws Exception {
        String facets = URLEncoder.encode("[[\"project_type:resourcepack\"],[\"versions:1.8.9\"]]", "UTF-8");
        String url = "https://api.modrinth.com/v2/search?limit=30&index=" + (query.trim().isEmpty() ? "downloads" : "relevance")
            + "&query=" + URLEncoder.encode(query.trim(), "UTF-8") + "&facets=" + facets;
        JsonObject o = new JsonParser().parse(new String(fetch(url, 4 << 20), "UTF-8")).getAsJsonObject();
        List<Pack> out = new ArrayList<Pack>();
        for (JsonElement e : o.getAsJsonArray("hits")) {
            JsonObject h = e.getAsJsonObject();
            out.add(new Pack(str(h, "project_id"), str(h, "title"), str(h, "description"), str(h, "author"), h.has("downloads") ? h.get("downloads").getAsLong() : 0));
        }
        return out;
    }

    /** Downloads the newest 1.8.9 file of `projectId` into `dir`. Returns the file name. */
    public static String install(String projectId, File dir) throws Exception {
        if (!projectId.matches("[A-Za-z0-9]{1,32}")) throw new IllegalArgumentException("bad id");
        String gv = URLEncoder.encode("[\"1.8.9\"]", "UTF-8");
        JsonArray versions = new JsonParser().parse(new String(fetch("https://api.modrinth.com/v2/project/" + projectId + "/version?game_versions=" + gv, 4 << 20), "UTF-8")).getAsJsonArray();
        if (versions.size() == 0) throw new IllegalStateException("No 1.8.9 version of this pack");
        JsonArray files = versions.get(0).getAsJsonObject().getAsJsonArray("files");
        JsonObject f = null;
        for (JsonElement e : files) if (f == null || e.getAsJsonObject().get("primary").getAsBoolean()) f = e.getAsJsonObject();
        if (f == null) throw new IllegalStateException("No file");
        String name = new File(str(f, "filename")).getName();
        if (!name.toLowerCase().endsWith(".zip") || name.contains("..")) throw new IllegalStateException("Not a .zip pack");
        String sha1 = str(f.getAsJsonObject("hashes"), "sha1");
        byte[] data = fetch(str(f, "url"), 256 << 20);
        MessageDigest md = MessageDigest.getInstance("SHA-1");
        StringBuilder hex = new StringBuilder();
        for (byte b : md.digest(data)) hex.append(String.format("%02x", b & 0xFF));
        if (!hex.toString().equalsIgnoreCase(sha1)) throw new IllegalStateException("The download didn't match Modrinth's checksum");
        if (!dir.isDirectory() && !dir.mkdirs()) throw new IllegalStateException("No resourcepacks folder");
        File tmp = new File(dir, name + ".part");
        FileOutputStream out = new FileOutputStream(tmp);
        out.write(data);
        out.close();
        File dest = new File(dir, name);
        if (dest.exists() && !dest.delete()) throw new IllegalStateException("Couldn't replace " + name);
        if (!tmp.renameTo(dest)) throw new IllegalStateException("Couldn't save " + name);
        return name;
    }
}
