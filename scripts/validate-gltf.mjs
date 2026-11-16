// Validate glTF/GLB files with the Khronos glTF Validator (Apache-2.0):
//   npm install --no-save gltf-validator@2.0.0-dev.3.10
//   node scripts/validate-gltf.mjs file.glb [...]
// Exits 1 if any file has an error (warnings and infos are printed only).
import { readFileSync } from 'node:fs';
import validator from 'gltf-validator';

let failed = false;
for (const file of process.argv.slice(2)) {
  const report = await validator.validateBytes(new Uint8Array(readFileSync(file)));
  const { numErrors, numWarnings, messages } = report.issues;
  console.log(`${file}: ${numErrors} errors, ${numWarnings} warnings`);
  for (const m of messages) {
    if (m.severity <= 1) console.log(`  [${m.severity === 0 ? 'error' : 'warning'}] ${m.code} ${m.pointer ?? ''}: ${m.message}`);
  }
  if (numErrors > 0) failed = true;
}
process.exit(failed ? 1 : 0);
