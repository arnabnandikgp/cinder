// Public AWS-only entry point; no caller-selected fixture root or approval flag.
import { verifyWebQuote } from '../../../tools/web-channel/attestation/client.mjs';
import { WebChannel, type WebOptions } from './web-channel-core.ts';
export type { WebOptions, WebCore } from './web-channel-core.ts';
/** Browser AND Node fetch use the same Rust/WASM Noise core and independent
 * AWS quote verifier. This is bounded offline V1 integration, not release approval. */
export const connectWebChannel = (options: WebOptions): Promise<WebChannel> => WebChannel.connect(options, verifyWebQuote);
