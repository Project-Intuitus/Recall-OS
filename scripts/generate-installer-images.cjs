/**
 * Generate custom NSIS installer images for RECALL.OS
 * Creates branded header and sidebar images in BMP format (required by NSIS)
 */

const sharp = require('sharp');
const path = require('path');
const fs = require('fs');

const OUTPUT_DIR = path.join(__dirname, '..', 'src-tauri', 'installer');

// RECALL.OS brand colors
const COLORS = {
  bg: '#030712',           // Near black
  surface: '#0f172a',      // Dark slate
  accent: '#06b6d4',       // Cyan
  accentDark: '#0891b2',   // Darker cyan
  text: '#f8fafc',         // White text
  textMuted: '#94a3b8',    // Muted text
};

// Ensure output directory exists
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

/**
 * Convert raw RGBA buffer to BMP format
 * BMP stores pixels bottom-to-top, BGR order (no alpha for 24-bit)
 */
function rgbaToBmp(rgbaBuffer, width, height) {
  // BMP row size must be multiple of 4 bytes
  const rowSize = Math.ceil((width * 3) / 4) * 4;
  const pixelDataSize = rowSize * height;
  const fileSize = 54 + pixelDataSize; // 14 (file header) + 40 (DIB header) + pixels

  const bmp = Buffer.alloc(fileSize);
  let offset = 0;

  // BMP File Header (14 bytes)
  bmp.write('BM', offset); offset += 2;                    // Signature
  bmp.writeUInt32LE(fileSize, offset); offset += 4;       // File size
  bmp.writeUInt16LE(0, offset); offset += 2;              // Reserved
  bmp.writeUInt16LE(0, offset); offset += 2;              // Reserved
  bmp.writeUInt32LE(54, offset); offset += 4;             // Pixel data offset

  // DIB Header (BITMAPINFOHEADER - 40 bytes)
  bmp.writeUInt32LE(40, offset); offset += 4;             // DIB header size
  bmp.writeInt32LE(width, offset); offset += 4;           // Width
  bmp.writeInt32LE(height, offset); offset += 4;          // Height (positive = bottom-up)
  bmp.writeUInt16LE(1, offset); offset += 2;              // Color planes
  bmp.writeUInt16LE(24, offset); offset += 2;             // Bits per pixel
  bmp.writeUInt32LE(0, offset); offset += 4;              // Compression (none)
  bmp.writeUInt32LE(pixelDataSize, offset); offset += 4;  // Image size
  bmp.writeInt32LE(2835, offset); offset += 4;            // X pixels per meter (72 DPI)
  bmp.writeInt32LE(2835, offset); offset += 4;            // Y pixels per meter
  bmp.writeUInt32LE(0, offset); offset += 4;              // Colors in color table
  bmp.writeUInt32LE(0, offset); offset += 4;              // Important colors

  // Pixel data (bottom-to-top, BGR order)
  for (let y = height - 1; y >= 0; y--) {
    let rowOffset = offset;
    for (let x = 0; x < width; x++) {
      const srcIdx = (y * width + x) * 4; // RGBA source
      bmp[rowOffset++] = rgbaBuffer[srcIdx + 2]; // B
      bmp[rowOffset++] = rgbaBuffer[srcIdx + 1]; // G
      bmp[rowOffset++] = rgbaBuffer[srcIdx + 0]; // R
    }
    // Pad row to multiple of 4 bytes
    while (rowOffset < offset + rowSize) {
      bmp[rowOffset++] = 0;
    }
    offset += rowSize;
  }

  return bmp;
}

/**
 * Create the header image (150x57) - shown at top during installation steps
 */
async function createHeaderImage() {
  const width = 150;
  const height = 57;

  // Create SVG with gradient and branding
  const svg = `
    <svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="headerGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style="stop-color:${COLORS.bg};stop-opacity:1" />
          <stop offset="100%" style="stop-color:${COLORS.surface};stop-opacity:1" />
        </linearGradient>
        <linearGradient id="accentGrad" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" style="stop-color:${COLORS.accent};stop-opacity:1" />
          <stop offset="100%" style="stop-color:${COLORS.accentDark};stop-opacity:1" />
        </linearGradient>
      </defs>

      <!-- Background -->
      <rect width="${width}" height="${height}" fill="url(#headerGrad)"/>

      <!-- Accent line at bottom -->
      <rect x="0" y="${height - 3}" width="${width}" height="3" fill="url(#accentGrad)"/>

      <!-- Logo/Text -->
      <text x="12" y="36" font-family="Segoe UI, Arial, sans-serif" font-size="16" font-weight="700" fill="${COLORS.text}">
        RECALL<tspan fill="${COLORS.accent}">.OS</tspan>
      </text>
    </svg>
  `;

  const { data, info } = await sharp(Buffer.from(svg))
    .raw()
    .ensureAlpha()
    .toBuffer({ resolveWithObject: true });

  const bmp = rgbaToBmp(data, info.width, info.height);
  fs.writeFileSync(path.join(OUTPUT_DIR, 'header.bmp'), bmp);

  console.log('Created header.bmp (150x57)');
}

/**
 * Create the sidebar image (164x314) - shown on welcome and finish screens
 */
async function createSidebarImage() {
  const width = 164;
  const height = 314;

  // Create SVG with gradient, logo, and decorative elements
  const svg = `
    <svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="sidebarGrad" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" style="stop-color:${COLORS.bg};stop-opacity:1" />
          <stop offset="50%" style="stop-color:${COLORS.surface};stop-opacity:1" />
          <stop offset="100%" style="stop-color:${COLORS.bg};stop-opacity:1" />
        </linearGradient>
        <radialGradient id="orbGrad" cx="50%" cy="50%" r="50%">
          <stop offset="0%" style="stop-color:${COLORS.accent};stop-opacity:0.2" />
          <stop offset="100%" style="stop-color:${COLORS.accent};stop-opacity:0" />
        </radialGradient>
      </defs>

      <!-- Background -->
      <rect width="${width}" height="${height}" fill="url(#sidebarGrad)"/>

      <!-- Decorative orb (top) -->
      <circle cx="20" cy="60" r="80" fill="url(#orbGrad)"/>

      <!-- Decorative orb (bottom) -->
      <circle cx="140" cy="260" r="60" fill="url(#orbGrad)"/>

      <!-- Accent line on right edge -->
      <rect x="${width - 3}" y="0" width="3" height="${height}" fill="${COLORS.accent}" opacity="0.5"/>

      <!-- Logo area -->
      <text x="${width / 2}" y="100" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="22" font-weight="800" fill="${COLORS.text}">
        RECALL
      </text>
      <text x="${width / 2}" y="125" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="22" font-weight="800" fill="${COLORS.accent}">
        .OS
      </text>

      <!-- Tagline -->
      <text x="${width / 2}" y="160" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="9" fill="${COLORS.textMuted}">
        Personal AI Memory
      </text>

      <!-- Version badge -->
      <rect x="${width / 2 - 25}" y="175" width="50" height="18" rx="9" fill="${COLORS.accent}" opacity="0.2"/>
      <rect x="${width / 2 - 25}" y="175" width="50" height="18" rx="9" stroke="${COLORS.accent}" stroke-width="1" fill="none" opacity="0.5"/>
      <text x="${width / 2}" y="188" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="9" font-weight="600" fill="${COLORS.accent}">
        v1.0.4
      </text>

      <!-- Bottom text -->
      <text x="${width / 2}" y="285" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="8" fill="${COLORS.textMuted}">
        Project Intuitus
      </text>
      <text x="${width / 2}" y="298" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="7" fill="${COLORS.textMuted}" opacity="0.6">
        projectintuitus.com
      </text>
    </svg>
  `;

  const { data, info } = await sharp(Buffer.from(svg))
    .raw()
    .ensureAlpha()
    .toBuffer({ resolveWithObject: true });

  const bmp = rgbaToBmp(data, info.width, info.height);
  fs.writeFileSync(path.join(OUTPUT_DIR, 'sidebar.bmp'), bmp);

  console.log('Created sidebar.bmp (164x314)');
}

async function main() {
  console.log('Generating RECALL.OS installer images (BMP format)...\n');

  await createHeaderImage();
  await createSidebarImage();

  console.log('\nDone! Images saved to:', OUTPUT_DIR);
}

main().catch(console.error);
