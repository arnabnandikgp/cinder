import type { QuoteContext, VerifiedQuote } from './verifier.mjs';
export function verifyWebQuote(quote: Uint8Array, policy: Uint8Array, context: QuoteContext): Promise<VerifiedQuote>;
