import sharp from 'sharp';
import pngToIco from 'png-to-ico';
import { writeFileSync, readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, '..', 'src-tauri', 'icons');
const svgPath = join(iconsDir, 'icon-source.svg');

// Read SVG content
const svgContent = readFileSync(svgPath);

// Higher density for sharper SVG rendering (pixels per inch)
// 300 DPI gives us crisp rendering even at small sizes
const SVG_DENSITY = 400;

// Icon sizes needed for various platforms
const sizes = {
  // Standard Tauri/Electron sizes
  '32x32.png': 32,
  '48x48.png': 48,   // Windows taskbar at 150% DPI
  '64x64.png': 64,   // Windows taskbar at 200% DPI
  '96x96.png': 96,   // Windows taskbar at 300% DPI
  '128x128.png': 128,
  '128x128@2x.png': 256,
  'icon.png': 512,

  // Windows Store logos
  'Square30x30Logo.png': 30,
  'Square44x44Logo.png': 44,
  'Square71x71Logo.png': 71,
  'Square89x89Logo.png': 89,
  'Square107x107Logo.png': 107,
  'Square142x142Logo.png': 142,
  'Square150x150Logo.png': 150,
  'Square284x284Logo.png': 284,
  'Square310x310Logo.png': 310,
  'StoreLogo.png': 50,
};

// ICO sizes (Windows) - comprehensive list for all DPI scaling scenarios
// Windows uses these for: taskbar, title bar, alt-tab, desktop shortcuts, explorer
const icoSizes = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 128, 256];

async function generateIcons() {
  console.log('Generating icons from SVG...\n');
  console.log(`Using SVG density: ${SVG_DENSITY} DPI for sharp rendering\n`);

  // Generate PNG files at various sizes with high quality settings
  for (const [filename, size] of Object.entries(sizes)) {
    const outputPath = join(iconsDir, filename);
    await sharp(svgContent, { density: SVG_DENSITY })
      .resize(size, size, {
        fit: 'contain',
        background: { r: 0, g: 0, b: 0, alpha: 0 },
        kernel: 'lanczos3' // High-quality downscaling algorithm
      })
      .png({
        compressionLevel: 9,
        adaptiveFiltering: true
      })
      .toFile(outputPath);
    console.log(`✓ Generated ${filename} (${size}x${size})`);
  }

  // Generate ICO file for Windows with all necessary sizes
  console.log('\nGenerating Windows ICO with', icoSizes.length, 'sizes...');
  const icoBuffers = [];
  for (const size of icoSizes) {
    const buffer = await sharp(svgContent, { density: SVG_DENSITY })
      .resize(size, size, {
        fit: 'contain',
        background: { r: 0, g: 0, b: 0, alpha: 0 },
        kernel: 'lanczos3'
      })
      .png({
        compressionLevel: 9,
        adaptiveFiltering: true
      })
      .toBuffer();
    icoBuffers.push(buffer);
    console.log(`  - Added ${size}x${size} to ICO`);
  }

  const icoBuffer = await pngToIco(icoBuffers);
  writeFileSync(join(iconsDir, 'icon.ico'), icoBuffer);
  console.log('✓ Generated icon.ico');

  // For macOS ICNS, we need the right PNGs which we've already generated
  // The actual .icns file would need a specialized tool, but Tauri can use the PNGs
  console.log('\nNote: For macOS icon.icns, use the generated PNGs with a tool like:');
  console.log('  - iconutil (macOS built-in)');
  console.log('  - https://cloudconvert.com/png-to-icns');
  console.log('  - Or keep existing icon.icns if it works');

  console.log('\n✅ Icon generation complete!');
}

generateIcons().catch(console.error);
