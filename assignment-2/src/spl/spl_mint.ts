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
    getMintToInstruction,
    TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";

import wallet from "../../devnet-wallet.json";

const rpc = createSolanaRpc(process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com");
const rpcSubscriptions = createSolanaRpcSubscriptions(
    process.env.SOLANA_WS_URL ?? "wss://api.devnet.solana.com"
);

const token_decimals = 1_000_000n;

// paste your mint address from spl_init.ts
const mint = address("5dxYuJqWd9qPxPRrLfXJ746w8LH1xPR6S41dRNzDq48o");

(async () => {
    try {
        const signer = await createKeyPairSignerFromBytes(new Uint8Array(wallet));

        const [ata] = await findAssociatedTokenPda({
            mint,
            owner: signer.address,
            tokenProgram: TOKEN_PROGRAM_ADDRESS,
        });
        console.log(`Your ata is : ${ata}`);

        const createAtaIx = await getCreateAssociatedTokenInstructionAsync({
            payer: signer,
            mint,
            owner: signer.address,
        });

        const mintToIx = getMintToInstruction({
            mint,
            token: ata,
            mintAuthority: signer,
            amount: 100n * token_decimals,
        });

        const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

        const msg = createTransactionMessage({ version: 0 });
        const msgWithPayer = setTransactionMessageFeePayerSigner(signer, msg);
        const msgWithLifetime = setTransactionMessageLifetimeUsingBlockhash(
            latestBlockhash,
            msgWithPayer
        );

        const txMessage = appendTransactionMessageInstructions(
            [createAtaIx, mintToIx],
            msgWithLifetime
        );

        const signedTx = await signTransactionMessageWithSigners(txMessage);
        assertIsTransactionWithBlockhashLifetime(signedTx);

        const signature = getSignatureFromTransaction(signedTx);

        const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });
        await sendAndConfirm(signedTx, { commitment: "confirmed" });

        console.log(`mint txid: ${signature}`);
    } catch (error) {
        console.log(error);
    }
})();
