import { defineConfig } from "vite";

export default defineConfig({
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
    },
    build: {
        target: ["es2022", "chrome110", "safari15"],
        outDir: "dist",
        // Minifying rewrites colors like rgb(6 60 100 / .3) to hex, which rounds the alpha.
        // The stylesheet is small, so it ships as written.
        cssMinify: false,
        emptyOutDir: true,
    },
});
