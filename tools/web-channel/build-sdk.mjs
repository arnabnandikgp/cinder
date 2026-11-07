// Tracked shipping entry; fixture SDK/root modules are NOT entrypoints here.
// Artifacts remain ignored and must be distributed with reviewed client releases.
import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';
await build({ absWorkingDir:fileURLToPath(new URL('.',import.meta.url)),entryPoints:['../../clients/private/src/browser.ts'],
  outfile:'pkg/sdk.js',bundle:true,format:'esm',platform:'browser',target:'es2022',legalComments:'eof' });
