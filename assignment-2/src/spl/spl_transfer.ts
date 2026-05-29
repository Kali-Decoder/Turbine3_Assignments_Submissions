import "dotenv/config";
import {
    address,
    appendTransactionMessageInstructions,
    assertIsTransactionWithBlockhashLifetime,
    createKeyPairSignerFromBytes,
    createSolanaRpc,
    createSolanaRpcSubscriptions,
    createTransactionMessage,
    getSignatureFromTransaction,
    sendAndConfirmTransactionFactory,
    setTransactionMessageFeePayerSigner,
    setTransactionMessageLifetimeUsingBlockhash,
    signTransactionMessageWithSigners,
} from "@solana/kit";
import {
    findAssociatedTokenPda,
    getCreateAssociatedTokenInstructionAsync,
    getTransferCheckedInstruction,
    TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";

import wallet from "../../devnet-wallet.json";

const rpc = createSolanaRpc(process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com");
const rpcSubscriptions = createSolanaRpcSubscriptions(
    process.env.SOLANA_WS_URL ?? "wss://api.devnet.solana.com"
);

// paste your mint address from spl_init.ts
const mint = address("5dxYuJqWd9qPxPRrLfXJ746w8LH1xPR6S41dRNzDq48o");

// paste the recipient wallet address
const to = address("EEQ2SfZx4uoxJWoCjkMPQTve3LdiabV6JQMCe9xrfYcZ");

(async () => {
    try {
        const signer = await createKeyPairSignerFromBytes(new Uint8Array(wallet));

        const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

        const [fromAta] = await findAssociatedTokenPda({
            mint,
            owner: signer.address,
            tokenProgram: TOKEN_PROGRAM_ADDRESS,
        });
        console.log(`Your fromAta is : ${fromAta}`);

        const [toAta] = await findAssociatedTokenPda({
            mint,
            owner: to,
            tokenProgram: TOKEN_PROGRAM_ADDRESS,
        });
        console.log(`Your toAta is : ${toAta}`);

        const createAtaIx = await getCreateAssociatedTokenInstructionAsync({
            payer: signer,
            mint,
            owner: to,
        });

        const transferIx = getTransferCheckedInstruction({
            source: fromAta,
            mint,
            destination: toAta,
            authority: signer,
            amount: 1_000_000n,
            decimals: 6,
        });

        const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

        const msg = createTransactionMessage({ version: 0 });
        const msgWithPayer = setTransactionMessageFeePayerSigner(signer, msg);
        const msgWithLifetime = setTransactionMessageLifetimeUsingBlockhash(
            latestBlockhash,
            msgWithPayer
        );

        const txMessage = appendTransactionMessageInstructions(
            [createAtaIx, transferIx],
            msgWithLifetime
        );

        const signedTx = await signTransactionMessageWithSigners(txMessage);
        assertIsTransactionWithBlockhashLifetime(signedTx);

        const signature = getSignatureFromTransaction(signedTx);

        await sendAndConfirm(signedTx, { commitment: "confirmed" });

        console.log(`transfer txid: ${signature}`);
    } catch (error) {
        console.log(error);
    }
})();
