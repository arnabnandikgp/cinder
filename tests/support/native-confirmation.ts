import type { BlockheightBasedTransactionConfirmationStrategy, Connection } from "@solana/web3.js";

/** web3.js HTTP and subscription confirmation paths report failures differently. */
export async function confirmForkTransaction(connection: Connection, strategy: BlockheightBasedTransactionConfirmationStrategy) {
    try {
        return await connection.confirmTransaction(strategy, "confirmed");
    } catch (error) {
        // Normalize only a proved, confirmed transaction failure. A timeout,
        // transport error, missing status or merely processed receipt is not
        // evidence that a deliberately failing transaction reached the fork.
        const result = await connection.getSignatureStatuses([strategy.signature], { searchTransactionHistory: true });
        const status = result.value[0];
        if (!status?.err || !["confirmed", "finalized"].includes(status.confirmationStatus ?? "")) throw error;
        return { context: result.context, value: { err: status.err } };
    }
}
