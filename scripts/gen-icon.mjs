// 生成 Hangar 应用图标源图（1024×1024 PNG），零依赖。
// 用法: node scripts/gen-icon.mjs  →  输出 scripts/app-icon.png
// 之后: npx tauri icon scripts/app-icon.png  →  生成 src-tauri/icons/*
//
// 图形语言：深色圆角方块底 + 琥珀色「机库拱门」（半环拱 + 门梁）。
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const S = 1024;
const OUT = join(dirname(fileURLToPath(import.meta.url)), "app-icon.png");

const BG = [18, 19, 24];
const AMBER = [245, 165, 36];

// 坐标系以图像中心为原点。
function inRoundedRect(x, y, half, r) {
  const ax = Math.abs(x);
  const ay = Math.abs(y);
  if (ax <= half - r || ay <= half - r) return true;
  const dx = ax - (half - r);
  const dy = ay - (half - r);
  return dx * dx + dy * dy <= r * r;
}

// 机库拱门：上半环（外径 outer、内径 inner）+ 底部门梁。
const CY = 108;
const OUTER = 330;
const INNER = 252;
const BAR_H = 46;

function inGlyph(x, y) {
  if (Math.abs(x) <= OUTER && y >= CY && y <= CY + BAR_H) return true;
  const dy = y - CY;
  if (dy <= 0) {
    const d = Math.hypot(x, dy);
    return d <= OUTER && d >= INNER;
  }
  return false;
}

function sample(x, y) {
  const cx = x - S / 2;
  const cy = y - S / 2;
  if (!inRoundedRect(cx, cy, S / 2 - 8, 180)) return [0, 0, 0, 0];
  return inGlyph(cx, cy) ? AMBER : BG;
}

const STRIDE = 1 + S * 4;
const raw = Buffer.alloc(S * STRIDE);
const SUB = [
  [0.25, 0.25],
  [0.75, 0.25],
  [0.25, 0.75],
  [0.75, 0.75],
];

for (let y = 0; y < S; y++) {
  const rowStart = y * STRIDE;
  raw[rowStart] = 0; // PNG filter: None
  for (let x = 0; x < S; x++) {
    let r = 0;
    let g = 0;
    let b = 0;
    let a = 0;
    for (const [ox, oy] of SUB) {
      const c = sample(x + ox, y + oy);
      r += c[0];
      g += c[1];
      b += c[2];
      a += c[3] ?? 255;
    }
    const o = rowStart + 1 + x * 4;
    raw[o] = Math.round(r / 4);
    raw[o + 1] = Math.round(g / 4);
    raw[o + 2] = Math.round(b / 4);
    raw[o + 3] = Math.round(a / 4);
  }
}

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const t = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

writeFileSync(OUT, png);
console.log(`icon written: ${OUT} (${(png.length / 1024).toFixed(1)} KB)`);
