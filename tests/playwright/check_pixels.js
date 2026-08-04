// One-shot PNG pixel probe for the A1.0.1 render-baseline fixtures.
// Decodes 8-bit RGBA PNG (zlib, unfilter) and counts pixels near expected colors.
const fs = require('fs');
const zlib = require('zlib');

function decodePng(file) {
  const buf = fs.readFileSync(file);
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error('not a png');
  let offset = 8, idat = [], w = 0, h = 0, colorType = 0;
  while (offset < buf.length) {
    const len = buf.readUInt32BE(offset);
    const type = buf.toString('ascii', offset + 4, offset + 8);
    if (type === 'IHDR') { w = buf.readUInt32BE(offset + 8); h = buf.readUInt32BE(offset + 12); colorType = buf.readUInt8(offset + 17); }
    else if (type === 'IDAT') idat.push(buf.subarray(offset + 8, offset + 8 + len));
    offset += 12 + len;
    if (type === 'IEND') break;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const ch = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[colorType];
  if (!ch) throw new Error(`unsupported color type ${colorType}`);
  const stride = w * ch;
  const out = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const ft = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let x = 0; x < stride; x++) {
      const a = x >= ch ? out[y * stride + x - ch] : 0;
      const b = y > 0 ? out[(y - 1) * stride + x] : 0;
      const c = x >= ch && y > 0 ? out[(y - 1) * stride + x - ch] : 0;
      let v = line[x];
      if (ft === 1) v = (v + a) & 255;
      else if (ft === 2) v = (v + b) & 255;
      else if (ft === 3) v = (v + ((a + b) >> 1)) & 255;
      else if (ft === 4) { const p = a + b - c; const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c); v = (v + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c)) & 255; }
      out[y * stride + x] = v;
    }
  }
  return { w, h, ch, data: out };
}

function countNear(img, rgb, tol) {
  let n = 0;
  for (let i = 0; i < img.data.length; i += img.ch) {
    if (Math.abs(img.data[i] - rgb[0]) <= tol && Math.abs(img.data[i + 1] - rgb[1]) <= tol && Math.abs(img.data[i + 2] - rgb[2]) <= tol) n++;
  }
  return n;
}

const root = '/Users/zerocool/repos/pystral_gate/.junie/plans/visual-fixtures/render-baseline';
const synthetic = [
  ['caveman square (red)', [200, 40, 40]],
  ['caveman circle (blue)', [40, 40, 200]],
  ['mage disc (magenta)', [200, 60, 220]],
  ['necro square (green)', [60, 180, 60]],
  ['necro triangle (teal)', [30, 120, 160]],
  ['skeleton triangle (yellow)', [230, 230, 60]],
  ['skeleton square (grey)', [90, 90, 90]],
];

const jobs = process.argv[2]
  ? [['probe', process.argv[2], synthetic]]
  : [
      ['render_synthetic', `${root}/render_synthetic/frame-1.png`, synthetic],
      ['player_boundary', `${root}/player_boundary/frame-1.png`, []],
    ];
for (const [label, file, expected] of jobs) {
  const img = decodePng(file);
  console.log(`\n== ${label} (${img.w}x${img.h}) ==`);
  // background sanity: corner pixel
  console.log(`corner pixel: [${img.data[0]}, ${img.data[1]}, ${img.data[2]}]`);
  if (expected.length) {
    for (const [name, rgb] of expected) {
      console.log(`${name} ${rgb}: ${countNear(img, rgb, 40)} px`);
    }
  }
}
