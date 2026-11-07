export interface QuoteContext {
  now: number; expires: number; nonce: Uint8Array; boot: Uint8Array; handle: Uint8Array; key: Uint8Array;
}
export interface VerifiedQuote { key: Uint8Array; prologue: Uint8Array }
export function verifyWithRoot(quote: Uint8Array, policy: Uint8Array, context: QuoteContext, trustedRoot: Uint8Array): Promise<VerifiedQuote>;
