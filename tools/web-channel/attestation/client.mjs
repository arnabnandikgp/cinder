import { verifyWithRoot } from './verifier.mjs';
import { root, fingerprint } from './aws-root.mjs';
import { digest } from './profile.mjs';

// Fixed AWS-only entry point; a caller/relay cannot select a different root.
// Still qualification-only, not an SDK release or hardware-qualified profile.
export async function verifyWebQuote(quote, policy, context) {
  try {
    const trustedRoot = root();
    const hash = [...await digest('SHA-256', trustedRoot)].map(b => b.toString(16).padStart(2, '0')).join('');
    if (hash !== fingerprint) throw Error('root fingerprint');
    return await verifyWithRoot(quote, policy, context, trustedRoot);
  } catch { throw Error('Web channel unavailable'); }
}
