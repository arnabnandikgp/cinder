import { expect } from "chai";
import type { Connection } from "@solana/web3.js";
import { confirmForkTransaction } from "./support/native-confirmation";

describe("native fork confirmation", () => {
    const strategy = { signature: "local-test-signature", blockhash: "local-test-blockhash", lastValidBlockHeight: 100 };
    const err = { InstructionError: [3, { Custom: 6017 }] };

    it("preserves a normally returned confirmation without another RPC", async () => {
        const result = { context: { slot: 42 }, value: { err: null } };
        const connection = { confirmTransaction: async () => result } as Connection;
        expect(await confirmForkTransaction(connection, strategy)).to.equal(result);
    });

    for (const confirmationStatus of ["confirmed", "finalized"]) {
        it(`normalizes a raw rejection only with a ${confirmationStatus} failed signature`, async () => {
            const connection = {
                confirmTransaction: async () => { throw err; },
                getSignatureStatuses: async (signatures: string[], options: unknown) => {
                    expect(signatures).to.deep.equal([strategy.signature]);
                    expect(options).to.deep.equal({ searchTransactionHistory: true });
                    return { context: { slot: 42 }, value: [{ err, confirmationStatus }] };
                },
            } as unknown as Connection;
            expect(await confirmForkTransaction(connection, strategy)).to.deep.equal({ context: { slot: 42 }, value: { err } });
        });
    }

    it("does not turn missing, processed or successful status into a failure proof", async () => {
        for (const status of [null, { err, confirmationStatus: "processed" }, { err: null, confirmationStatus: "confirmed" }]) {
            const transportError = new Error("confirmation unavailable");
            const connection = {
                confirmTransaction: async () => { throw transportError; },
                getSignatureStatuses: async () => ({ context: { slot: 42 }, value: [status] }),
            } as unknown as Connection;
            let failure: unknown;
            try { await confirmForkTransaction(connection, strategy); } catch (error) { failure = error; }
            expect(failure).to.equal(transportError);
        }
    });
});
