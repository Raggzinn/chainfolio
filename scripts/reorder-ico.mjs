// Tauri uses the first image of icon.ico as the window/taskbar icon, and
// `tauri icon` writes the 16px image first, which Windows then upscales into a
// blurry taskbar icon. This rewrites the .ico with its images sorted largest first.
import { readFileSync, writeFileSync } from "node:fs";

const path = process.argv[2] ?? "src-tauri/icons/icon.ico";
const ico = readFileSync(path);

const count = ico.readUInt16LE(4);
const entries = Array.from({ length: count }, (_, i) => {
  const header = ico.subarray(6 + i * 16, 6 + (i + 1) * 16);
  const size = header.readUInt32LE(8);
  const offset = header.readUInt32LE(12);
  return {
    width: header[0] || 256,
    header: Buffer.from(header),
    data: ico.subarray(offset, offset + size),
  };
});

entries.sort((a, b) => b.width - a.width);

const out = [Buffer.from(ico.subarray(0, 6))];
let offset = 6 + count * 16;
for (const entry of entries) {
  entry.header.writeUInt32LE(offset, 12);
  out.push(entry.header);
  offset += entry.data.length;
}
out.push(...entries.map((e) => e.data));

writeFileSync(path, Buffer.concat(out));
console.log(`Reordered ${path}: ${entries.map((e) => e.width).join(", ")}`);
