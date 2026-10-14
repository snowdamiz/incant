package dev.incant.player;

import android.content.res.AssetManager;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HashSet;

/** Extracts only the trusted exporter's bounded, hashed cooked asset catalog. */
final class PackageFiles {
    private static final long LIMIT = 512L * 1024 * 1024;
    private PackageFiles() {}
    static File extract(AssetManager assets, File cache) throws Exception {
        File directory = File.createTempFile("incant-player-", ".data", cache);
        if (!directory.delete() || !directory.mkdir()) throw new IOException("Private package directory failed");
        try {
            HashSet<String> seen = new HashSet<>();
            long total = 0;
            try (BufferedReader reader = new BufferedReader(new InputStreamReader(assets.open("incant/index.txt"), StandardCharsets.US_ASCII))) {
                String line;
                while ((line = boundedLine(reader)) != null) {
                    if (line.length() > 512 || seen.size() >= 8192) throw new IOException("Asset catalog limit");
                    String[] fields = line.split(" ", -1);
                    if (fields.length != 3 || !fields[0].matches("[0-9a-f]{64}") || !fields[1].matches("[0-9]{1,10}")
                        || !fields[2].matches("[a-zA-Z0-9_-]+(?:/[a-zA-Z0-9_-]+){0,3}(?:\\.[a-zA-Z0-9_-]+)?")
                        || !seen.add(fields[2])) throw new IOException("Invalid asset catalog");
                    long size = Long.parseLong(fields[1]); total += size;
                    if (size > 128L * 1024 * 1024 || total > LIMIT) throw new IOException("Cooked package size limit");
                    File file = new File(directory, fields[2]);
                    File parent = file.getParentFile();
                    if (!parent.isDirectory() && !parent.mkdirs()) throw new IOException("Package parent failed");
                    MessageDigest digest = MessageDigest.getInstance("SHA-256");
                    long copied = 0; byte[] buffer = new byte[16384];
                    try (InputStream input = assets.open("incant/data/" + fields[2]); OutputStream output = new FileOutputStream(file)) {
                        int n;
                        while ((n = input.read(buffer)) != -1) {
                            copied += n;
                            if (copied > size) throw new IOException("Asset size mismatch");
                            digest.update(buffer, 0, n); output.write(buffer, 0, n);
                        }
                    }
                    StringBuilder hash = new StringBuilder(64);
                    for (byte b : digest.digest()) hash.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
                    if (copied != size || !hash.toString().equals(fields[0])) throw new IOException("Asset identity mismatch");
                }
            }
            if (!seen.contains("package.json")) throw new IOException("Missing cooked package");
            return directory;
        } catch (Exception error) { remove(directory); throw error; }
    }
    private static String boundedLine(Reader reader) throws IOException {
        StringBuilder value = new StringBuilder();
        for (int c; (c = reader.read()) != -1;) {
            if (c == '\n') return value.toString();
            if (value.length() == 512 || c > 127) throw new IOException("Asset catalog line limit");
            value.append((char)c);
        }
        return value.length() == 0 ? null : value.toString();
    }
    static void remove(File directory) {
        File[] files = directory.listFiles();
        if (files != null) for (File file : files) { if (file.isDirectory()) remove(file); else file.delete(); }
        directory.delete();
    }
}
