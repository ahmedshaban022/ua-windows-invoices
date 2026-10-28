/**
 * Generates the Windows icon set from the owner-supplied brand assets.
 *
 * Source of truth:
 *   assets/brand/mark.svg  — the hexagon mark, already recolored to #0b68e6
 *
 * Outputs (src-tauri/icons/):
 *   32x32.png, 128x128.png, 128x128@2x.png, icon.png, icon.ico
 *
 * Run with:  npm run icons
 */
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const brandDir = resolve(root, "assets/brand");
const iconsDir = resolve(root, "src-tauri/icons");

/** Render the mark onto a square canvas with a small transparent margin. */
async function renderMark(size) {
  const svg = await sharp(resolve(brandDir, "mark.svg"))
    .resize(size, size, {
      fit: "contain",
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    })
    .png()
    .toBuffer();
  return svg;
}

/** Build an .ico containing the given PNG sizes. */
function buildIco(pngBuffers) {
  // ICONDIR: reserved(2) type(2) count(2)
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: icon
  header.writeUInt16LE(pngBuffers.length, 4);

  const entries = [];
  let offset = 6 + pngBuffers.length * 16;

  for (const { size, buffer } of pngBuffers) {
    const entry = Buffer.alloc(16);
    entry.writeUInt8(size >= 256 ? 0 : size, 0); // width
    entry.writeUInt8(size >= 256 ? 0 : size, 1); // height
    entry.writeUInt8(0, 2); // palette count
    entry.writeUInt8(0, 3); // reserved
    entry.writeUInt16LE(1, 4); // color planes
    entry.writeUInt16LE(32, 6); // bits per pixel
    entry.writeUInt32LE(buffer.length, 8);
    entry.writeUInt32LE(offset, 12);
    entries.push(entry);
    offset += buffer.length;
  }

  return Buffer.concat([header, ...entries, ...pngBuffers.map((p) => p.buffer)]);
}

async function main() {
  await mkdir(iconsDir, { recursive: true });

  const targets = [
    { name: "32x32.png", size: 32 },
    { name: "128x128.png", size: 128 },
    { name: "128x128@2x.png", size: 256 },
    { name: "icon.png", size: 512 },
  ];

  for (const target of targets) {
    const buffer = await renderMark(target.size);
    await writeFile(resolve(iconsDir, target.name), buffer);
    console.log(`wrote icons/${target.name} (${target.size}x${target.size})`);
  }

  // The .ico bundles 16, 32, 48, 64 and 256 px variants.
  const icoSizes = [16, 32, 48, 64, 256];
  const icoBuffers = [];
  for (const size of icoSizes) {
    icoBuffers.push({ size, buffer: await renderMark(size) });
  }
  await writeFile(resolve(iconsDir, "icon.ico"), buildIco(icoBuffers));
  console.log(`wrote icons/icon.ico (${icoSizes.join(", ")} px)`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
