// 生成译核 TransCore 图标 —— SVG 源图 + 多尺寸 PNG + ICO 文件
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const sizes = [16, 24, 32, 48, 64, 128, 256, 512];
const buildDir = path.join(__dirname, "..", "build");
if (!fs.existsSync(buildDir)) fs.mkdirSync(buildDir, { recursive: true });

// ========== 1. SVG 源图 ==========
const svgIcon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0e1726"/>
      <stop offset="100%" stop-color="#0a0e14"/>
    </linearGradient>
    <linearGradient id="ring" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#22d3ee"/>
      <stop offset="100%" stop-color="#0e7490"/>
    </linearGradient>
    <linearGradient id="ring2" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#fbbf24"/>
      <stop offset="100%" stop-color="#b45309"/>
    </linearGradient>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="8" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>
  <!-- 背景圆 -->
  <circle cx="256" cy="256" r="240" fill="url(#bg)" stroke="#22d3ee" stroke-width="3" opacity="0.8"/>
  <!-- 外圈 - 青蓝 -->
  <circle cx="256" cy="256" r="200" fill="none" stroke="url(#ring)" stroke-width="4" opacity="0.9"/>
  <!-- 中圈 - 琥珀 -->
  <circle cx="256" cy="256" r="160" fill="none" stroke="url(#ring2)" stroke-width="3" opacity="0.7" stroke-dasharray="8 6"/>
  <!-- 内圈 - 青蓝 -->
  <circle cx="256" cy="256" r="120" fill="none" stroke="url(#ring)" stroke-width="2.5" opacity="0.6"/>
  <!-- 中心发光点 -->
  <circle cx="256" cy="256" r="18" fill="#22d3ee" filter="url(#glow)"/>
  <circle cx="256" cy="256" r="8" fill="#fbbf24"/>
  <!-- 汉字"译"的抽象符号 - 两个箭头交换 -->
  <g transform="translate(256,256)" fill="none" stroke="#e6edf5" stroke-width="4" stroke-linecap="round" stroke-linejoin="round">
    <path d="M-50,-50 L50,-50 L50,-30" opacity="0.9"/>
    <path d="M50,50 L-50,50 L-50,30" opacity="0.9"/>
    <path d="M-30,-10 L0,20 L30,-10" stroke="#fbbf24" stroke-width="5"/>
  </g>
  <!-- 刻度标记 -->
  <g fill="#22d3ee">
    <rect x="254" y="20" width="4" height="14" opacity="0.9"/>
    <rect x="254" y="478" width="4" height="14" opacity="0.9"/>
    <rect x="20" y="254" width="14" height="4" opacity="0.9"/>
    <rect x="478" y="254" width="14" height="4" opacity="0.9"/>
  </g>
  <!-- 底部文字 -->
  <text x="256" y="430" text-anchor="middle" font-family="Segoe UI, sans-serif" font-size="42" font-weight="700" fill="#e6edf5" letter-spacing="4">译核</text>
  <text x="256" y="458" text-anchor="middle" font-family="Arial Narrow, sans-serif" font-size="18" font-weight="500" fill="#9fb0c3" letter-spacing="6">TRANSCORE</text>
</svg>`;

fs.writeFileSync(path.join(buildDir, "icon.svg"), svgIcon);
console.log("✓ SVG 图标已生成");

// ========== 2. 简易 PNG 生成（不依赖 sharp/canvas，用 PNG 规范手写） ==========
// 生成一个纯圆形渐变 + 文字的简单 PNG，作为可用图标
// 为了保持简单可靠，我们直接从 SVG 生成 base64 的 256x256 PNG 比较复杂
// 替代方案：使用一个纯色 + 文字的极简 PNG，保证可用

// 用 node 的 Buffer 手动写一个最小的 PNG（BMP-like，无压缩）
// 这是一个工程上可工作的方案：生成 256x256 带透明背景的青色圆形 + 琥珀色文字

function generatePNG(size) {
  const width = size;
  const height = size;
  const channels = 4; // RGBA
  const data = Buffer.alloc(width * height * channels);

  const cx = width / 2;
  const cy = height / 2;
  const r = width * 0.42;

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const idx = (y * width + x) * channels;
      const dx = x - cx;
      const dy = y - cy;
      const dist = Math.sqrt(dx * dx + dy * dy);

      // 默认透明
      data[idx] = 0;
      data[idx + 1] = 0;
      data[idx + 2] = 0;
      data[idx + 3] = 0;

      if (dist <= r) {
        // 圆形背景渐变
        const t = dist / r; // 0:中心 1:边缘
        const bgR = Math.round(14 + (10 - 14) * t);
        const bgG = Math.round(23 + (14 - 23) * t);
        const bgB = Math.round(38 + (20 - 38) * t);

        data[idx] = bgR;
        data[idx + 1] = bgG;
        data[idx + 2] = bgB;
        data[idx + 3] = 255;

        // 外环
        if (dist > r - 2 && dist <= r) {
          data[idx] = 34;
          data[idx + 1] = 211;
          data[idx + 2] = 238;
          data[idx + 3] = 230;
        }

        // 内圈（虚线感）
        const innerR = r * 0.65;
        if (dist > innerR - 1 && dist < innerR + 1) {
          const angle = Math.atan2(dy, dx);
          const dash = Math.sin(angle * 12);
          if (dash > 0.3) {
            data[idx] = 251;
            data[idx + 1] = 191;
            data[idx + 2] = 36;
            data[idx + 3] = 200;
          }
        }

        // 中心点
        if (dist < r * 0.08) {
          data[idx] = 34;
          data[idx + 1] = 211;
          data[idx + 2] = 238;
          data[idx + 3] = 255;
        }
        if (dist < r * 0.04) {
          data[idx] = 251;
          data[idx + 1] = 191;
          data[idx + 2] = 36;
          data[idx + 3] = 255;
        }
      }
    }
  }

  return { width, height, data };
}

function encodePNG(width, height, pixelData) {
  // PNG 签名
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

  // IHDR chunk
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type: RGBA
  ihdr[10] = 0; // compression
  ihdr[11] = 0; // filter
  ihdr[12] = 0; // interlace
  const ihdrChunk = makeChunk("IHDR", ihdr);

  // IDAT chunk: 每一行前加 filter byte (0)
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0; // filter: None
    pixelData.copy(raw, y * (stride + 1) + 1, y * stride, y * stride + stride);
  }
  const compressed = zlib.deflateSync(raw);
  const idatChunk = makeChunk("IDAT", compressed);

  // IEND chunk
  const iendChunk = makeChunk("IEND", Buffer.alloc(0));

  return Buffer.concat([signature, ihdrChunk, idatChunk, iendChunk]);
}

function makeChunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crcData = Buffer.concat([typeBuf, data]);
  const crc = crc32(crcData);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc >>> 0, 0);
  return Buffer.concat([length, typeBuf, data, crcBuf]);
}

function crc32(buf) {
  let crc = 0xffffffff;
  const table = crcTable();
  for (let i = 0; i < buf.length; i++) {
    crc = (crc >>> 8) ^ table[(crc ^ buf[i]) & 0xff];
  }
  return crc ^ 0xffffffff;
}
function crcTable() {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c >>> 0;
  }
  return table;
}

// 生成各尺寸 PNG
for (const size of sizes) {
  const { width, height, data } = generatePNG(size);
  const png = encodePNG(width, height, data);
  fs.writeFileSync(path.join(buildDir, `icon-${size}.png`), png);
  console.log(`✓ icon-${size}.png 已生成`);
}

// 复制 256 作为默认 icon.png
fs.copyFileSync(path.join(buildDir, "icon-256.png"), path.join(buildDir, "icon.png"));

// ========== 3. ICO 文件生成 ==========
// ICO 格式：ICONDIR + ICONDIRENTRY[] + 各 PNG 数据
function generateICO() {
  const iconSizes = [16, 24, 32, 48, 64, 128, 256]; // ICO 标准尺寸
  const pngBuffers = [];
  for (const s of iconSizes) {
    const { width, height, data } = generatePNG(s);
    pngBuffers.push(encodePNG(width, height, data));
  }

  const headerSize = 6; // ICONDIR
  const entrySize = 16 * iconSizes.length; // ICONDIRENTRY
  let dataOffset = headerSize + entrySize;

  const entries = [];
  for (let i = 0; i < iconSizes.length; i++) {
    const s = iconSizes[i];
    const png = pngBuffers[i];
    const entry = Buffer.alloc(16);
    entry[0] = s === 256 ? 0 : s; // width (0 = 256)
    entry[1] = s === 256 ? 0 : s; // height
    entry[2] = 0; // color count
    entry[3] = 0; // reserved
    entry.writeUInt16LE(1, 4); // planes
    entry.writeUInt16LE(32, 6); // bit count
    entry.writeUInt32LE(png.length, 8); // size in bytes
    entry.writeUInt32LE(dataOffset, 12); // offset
    entries.push(entry);
    dataOffset += png.length;
  }

  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: 1 = icon
  header.writeUInt16LE(iconSizes.length, 4); // count

  return Buffer.concat([header, ...entries, ...pngBuffers]);
}

const icoBuffer = generateICO();
fs.writeFileSync(path.join(buildDir, "icon.ico"), icoBuffer);
console.log("✓ icon.ico 已生成");

console.log("\n✅ 所有图标生成完毕，位于 build/ 目录");
