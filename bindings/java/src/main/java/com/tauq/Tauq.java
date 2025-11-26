package com.tauq;

import java.nio.file.Path;
import java.nio.file.Paths;

/**
 * Tauq (τq) Bindings for Java.
 * <p>
 * Provides methods to parse Tauq into JSON strings and format JSON strings into Tauq.
 * This class loads the native library 'tauq' via JNI.
 * </p>
 */
public class Tauq {

    static {
        // Attempt to load library from java.library.path
        // In a real jar distribution, this might unpack the .so/.dll/.dylib from resources.
        try {
            System.loadLibrary("tauq");
        } catch (UnsatisfiedLinkError e) {
            // Fallback for development: try to find in target/release
            String libName = System.mapLibraryName("tauq");
            Path libPath = Paths.get(System.getProperty("user.dir"), "../../target/release", libName);
            try {
                System.load(libPath.toAbsolutePath().toString());
            } catch (UnsatisfiedLinkError e2) {
                System.err.println("Failed to load native library 'tauq'. Ensure it is in java.library.path or target/release.");
                throw e;
            }
        }
    }

    /**
     * Parse Tauq source to JSON string.
     * 
     * @param tauqSource The Tauq input string.
     * @return JSON representation.
     * @throws IllegalArgumentException if parsing fails.
     */
    public static native String parseToJson(String tauqSource);

    /**
     * Execute Tauq Query (TQQ) source to JSON string.
     * 
     * @param input The TQQ source string.
     * @param safeMode If true, disables unsafe directives like !run.
     * @return JSON representation.
     * @throws IllegalArgumentException if execution fails.
     */
    public static native String execQuery(String input, boolean safeMode);

    /**
     * Minify Tauq source to single-line Tauq string.
     * 
     * @param tauqSource The Tauq input string.
     * @return Minified Tauq string.
     * @throws IllegalArgumentException if parsing fails.
     */
    public static native String minify(String tauqSource);

    /**
     * Format JSON string to Tauq.
     * 
     * @param jsonSource The JSON input string.
     * @return Tauq representation.
     * @throws IllegalArgumentException if JSON is invalid.
     */
    public static native String formatJson(String jsonSource);

    /**
     * Convenience method to parse Tauq and return a Jackson JsonNode (if available).
     * Since we want zero-dep bindings, we stick to String.
     * Users can use their preferred JSON library (Jackson, Gson) to parse the result.
     */
    public static String toJson(String tauqSource) {
        return parseToJson(tauqSource);
    }

    /**
     * Alias for formatJson.
     */
    public static String toTauq(String jsonSource) {
        return formatJson(jsonSource);
    }
}
