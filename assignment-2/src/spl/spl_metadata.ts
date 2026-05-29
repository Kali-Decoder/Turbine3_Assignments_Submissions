import "dotenv/config";
import { createSignerFromKeypair, publicKey, signerIdentity } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
    createMetadataAccountV3,
    CreateMetadataAccountV3InstructionAccounts,
    CreateMetadataAccountV3InstructionArgs,
    DataV2Args,
} from "@metaplex-foundation/mpl-token-metadata";
import bs58 from "bs58";

import wallet from "../../devnet-wallet.json";

// paste your mint address from spl_init.ts
const mint = publicKey("5dxYuJqWd9qPxPRrLfXJ746w8LH1xPR6S41dRNzDq48o");

const umi = createUmi(process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com");

const keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
const signer = createSignerFromKeypair(umi, keypair);

umi.use(signerIdentity(signer));

(async () => {
    try {
        const accounts: CreateMetadataAccountV3InstructionAccounts = {
            mint,
            mintAuthority: signer,
        };

        const data: DataV2Args = {
            name: "Kali-Decoder Token",
            symbol: "KDC",
            uri: "https://arweave.net/123456",
            sellerFeeBasisPoints: 0,
            creators: null,
            collection: null,
            uses: null,
        };

        const args: CreateMetadataAccountV3InstructionArgs = {
            data,
            isMutable: true,
            collectionDetails: null,
        };

        const tx = createMetadataAccountV3(umi, {
            ...accounts,
            ...args,
        });

        const result = await tx.sendAndConfirm(umi);
        console.log("signature: ", bs58.encode(Buffer.from(result.signature)));
    } catch (error) {
        console.log("error", error);
    }
})();
