// Build-only pinned tool. Inputs are tracked modules, output stays ignored.
import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';
await build({ absWorkingDir: fileURLToPath(new URL('..', import.meta.url)), entryPoints: ['attestation/suite.mjs'],
  outfile:'pkg/attestation.js', bundle:true, format:'esm', platform:'browser', target:'es2022', legalComments:'eof' });
