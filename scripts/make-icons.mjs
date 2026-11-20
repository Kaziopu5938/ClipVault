import sharp from "sharp";
import pngToIco from "png-to-ico";
import { mkdir, writeFile, readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const icons = join(root, "src-tauri", "icons");
const source = join(icons, "icon.svg");

await mkdir(icons, { recursive: true });

const pngSizes = [32, 128, 256, 512];
for (const size of pngSizes) {
  const out = join(icons, size === 256 ? "128x128@2x.png" : `${size}x${size}.png`);
  await sharp(source).resize(size, size).png().toFile(out);
  if (size === 256) await sharp(source).resize(256, 256).png().toFile(join(icons, "icon.png"));
  console.log("wrote", out);
}

const icoSizes = [16, 24, 32, 48, 64, 128, 256];
const buffers = [];
for (const size of icoSizes) {
  buffers.push(await sharp(source).resize(size, size).png().toBuffer());
}
await writeFile(join(icons, "icon.ico"), await pngToIco(buffers));
console.log("wrote icon.ico");

// Windows Store logos used by the bundler.
for (const [name, size] of [
  ["Square30x30Logo.png", 30],
  ["Square44x44Logo.png", 44],
  ["Square71x71Logo.png", 71],
  ["Square89x89Logo.png", 89],
  ["Square107x107Logo.png", 107],
  ["Square142x142Logo.png", 142],
  ["Square150x150Logo.png", 150],
  ["Square284x284Logo.png", 284],
  ["Square310x310Logo.png", 310],
  ["StoreLogo.png", 50],
]) {
  await sharp(source).resize(size, size).png().toFile(join(icons, name));
}
console.log("wrote Windows logos");

// Copy the mark to the web app.
await mkdir(join(root, "public"), { recursive: true });
await writeFile(join(root, "public", "clipvault-mark.svg"), await readFile(source));
